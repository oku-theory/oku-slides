//! Le deck dans un document Loro (CRDT) : historique complet et fusion.
//!
//! - [`DeckReplica`] : création, snapshot, lecture ([`DeckReplica::deck`]),
//!   application d'un `Deck` modifié ([`DeckReplica::apply`]), annuler.
//! - Le `Deck` n'est qu'une vue ; le replica fait foi.

mod authors;
mod read;
mod rich;
mod schema;
mod undo;
mod write;

pub use authors::MergeReport;

use std::panic::{AssertUnwindSafe, catch_unwind};

use loro::{ExportMode, LoroDoc, LoroValue, ValueOrContainer};
use snafu::{ResultExt, ensure};

use crate::error::{
    DeckJsonSnafu, ExportSnafu, ImportPanickedSnafu, ImportSnafu, LoroSnafu, NotADeckSnafu, ReplicaError, ReplicaInner,
    TooLargeSnafu, UnrelatedSnafu,
};
use crate::model::Deck;
pub(crate) use rich::SEP as PARAGRAPH_SEPARATOR;
use schema::Ctx;
use undo::Undo;

const ROOT: &str = "deck";
const META: &str = "meta";
const DECK_UID: &str = "deck_uid";

/// Préfixe du message d'un commit intermédiaire d'une application (avant
/// un `mov`) : l'état qui suit n'est pas une version du deck.
pub(crate) const PART: &str = "oku-part:";

fn part_message(label: &str) -> String {
    format!("{PART}{label}")
}

/// Le deck, son historique complet, et son identité (`deck_uid`).
pub struct DeckReplica {
    doc: LoroDoc,
    undo: Undo,
    /// Nom de l'auteur de ce pair, inscrit dans `peers` à sa première édition.
    author: Option<String>,
}

impl std::fmt::Debug for DeckReplica {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `undo` n'est pas `Debug` : on ne montre que ce qui vient du doc.
        f.debug_struct("DeckReplica")
            .field("deck_uid", &self.deck_uid())
            .field("changes", &self.doc.len_changes())
            .finish()
    }
}

/// Un deck relu, et ce qui n'a pas pu être lu.
pub type DeckRead = (Deck, Vec<crate::Issue>);

/// Réglages communs à tout `LoroDoc` du deck (non persistés par Loro).
fn configure(doc: &LoroDoc) {
    doc.set_record_timestamp(true);
    doc.config_text_style(rich::styles());
    // Sans cela, Loro fusionne les commits locaux d'une fenêtre de 1000 s
    // portant le même message en un seul *change* : un commit, un change.
    doc.set_change_merge_interval(-1);
}

fn uid_of(doc: &LoroDoc) -> String {
    match doc.get_map(META).get(DECK_UID) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
        _ => String::new(),
    }
}

fn read_deck(doc: &LoroDoc) -> DeckRead {
    read::materialize(read::read_map(&doc.get_map(ROOT), Ctx::Deck))
}

/// Exécute `f` (import d'octets non fiables, puis première lecture) sous
/// `catch_unwind` : une panique de Loro devient une erreur `Corrupted`.
///
/// Loro décode l'état d'un conteneur au premier accès, pas à l'import : la
/// première lecture complète (`deck_uid`, puis tout l'arbre `deck`) doit donc
/// se faire ici. Après une panique, les verrous internes de `doc` sont
/// empoisonnés et son drop paniquerait à nouveau : `doc` est abandonné sans
/// drop (fuite assumée, pour un fichier corrompu).
///
/// Limite connue (t-223) : une valeur plate imbriquée sur des dizaines de
/// milliers de niveaux tient en quelques Kio (LZ4). Loro la décode au premier
/// accès et la libère au drop, récursivement : la pile déborde et le
/// processus s'arrête, ce que ni `catch_unwind` ni le plafond d'octets
/// n'empêchent.
///
/// Reste hors garde ce que la première lecture n'atteint pas : les conteneurs
/// hors de `deck` et `meta`, et les sous-arbres au-delà de `read::MAX_DEPTH`.
/// Loro peut les décoder plus tard (export d'un snapshot, réconciliation,
/// annuler) ; une panique y remonterait alors à l'appelant.
fn guarded<T>(doc: LoroDoc, f: impl FnOnce(&LoroDoc) -> Result<T, ReplicaInner>) -> Result<(LoroDoc, T), ReplicaInner> {
    match catch_unwind(AssertUnwindSafe(|| f(&doc))) {
        Ok(r) => r.map(|t| (doc, t)),
        Err(_) => {
            std::mem::forget(doc);
            ImportPanickedSnafu.fail()
        }
    }
}

impl DeckReplica {
    /// Nouveau replica, avec un nouveau `deck_uid`.
    pub fn new(deck: &Deck) -> Result<Self, ReplicaError> {
        Ok(Self::create(deck)?)
    }

    fn create(deck: &Deck) -> Result<Self, ReplicaInner> {
        let doc = LoroDoc::new();
        configure(&doc);
        // Le pair créateur est tiré au hasard sur 64 bits : un identifiant suffisant.
        let uid = format!("{:016x}", doc.peer_id());
        doc.get_map(META).insert(DECK_UID, uid).context(LoroSnafu)?;
        // Des insertions seulement : aucun `mov`, rien à découper.
        Self::reconcile_doc(&doc, deck, &|| ())?;
        doc.commit();
        // `Undo` est construit après ce commit initial : créer le deck n'est pas annulable.
        let undo = Undo::new(&doc);
        Ok(Self { doc, undo, author: None })
    }

    /// Relit un snapshot. Les octets viennent d'un fichier : non fiables.
    ///
    /// Renvoie aussi la première lecture complète du deck (celle de
    /// [`Self::deck`]), faite sous garde : Loro ne décode l'état d'un conteneur
    /// qu'au premier accès, et panique (`unwrap`) sur un état malformé.
    pub fn from_snapshot(bytes: &[u8], max_bytes: usize) -> Result<(Self, DeckRead), ReplicaError> {
        Ok(Self::import_snapshot(bytes, max_bytes)?)
    }

    fn import_snapshot(bytes: &[u8], max: usize) -> Result<(Self, DeckRead), ReplicaInner> {
        ensure!(bytes.len() <= max, TooLargeSnafu { size: bytes.len(), max });
        let doc = LoroDoc::new();
        configure(&doc);
        let (doc, read) = guarded(doc, |doc| {
            doc.import(bytes).context(ImportSnafu)?;
            ensure!(!uid_of(doc).is_empty(), NotADeckSnafu { what: "no deck_uid" });
            Ok(read_deck(doc))
        })?;
        // `Undo` après l'import : l'état relu n'est pas une étape annulable.
        let undo = Undo::new(&doc);
        Ok((Self { doc, undo, author: None }, read))
    }

    pub fn snapshot(&self) -> Result<Vec<u8>, ReplicaError> {
        Ok(self.doc.export(ExportMode::Snapshot).context(ExportSnafu)?)
    }

    pub fn deck_uid(&self) -> String {
        uid_of(&self.doc)
    }

    /// Le deck relu depuis le replica, et ce qui n'a pas pu être lu.
    pub fn deck(&self) -> DeckRead {
        read_deck(&self.doc)
    }

    /// Identifiant de ce pair (session d'édition).
    pub fn peer_id(&self) -> u64 {
        self.doc.peer_id()
    }

    /// Nom de l'auteur de ce pair ; un nom blanc vaut aucun.
    pub fn set_author(&mut self, name: Option<String>) {
        self.author = name.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty());
    }

    /// Nom inscrit pour `peer` dans `peers`, s'il y en a un.
    pub fn author_of(&self, peer: u64) -> Option<String> {
        authors::author_of(&self.doc, peer)
    }

    /// Nombre de *changes* Loro (tests, mesures).
    pub fn len_changes(&self) -> usize {
        self.doc.len_changes()
    }

    /// Inscrit l'auteur de ce pair, s'il en a un, avant sa première édition.
    pub(crate) fn claim_author(&self) -> Result<(), ReplicaError> {
        match &self.author {
            Some(name) => Ok(authors::claim(&self.doc, name)?),
            None => Ok(()),
        }
    }

    /// Amène le replica à l'état de `deck`, en une étape d'annulation `label`.
    /// Dans un groupe, le libellé du groupe l'emporte.
    ///
    /// Non atomique : si la réconciliation échoue en cours de route, les
    /// changements déjà faits restent dans le doc ; ceux qui ne sont pas encore
    /// committés rejoignent le prochain commit, et son étape.
    pub fn apply(&mut self, deck: &Deck, label: &str) -> Result<(), ReplicaError> {
        let doc = &self.doc;
        let split = || {
            doc.set_next_commit_message(&part_message(label));
            doc.commit();
        };
        Self::reconcile_doc(doc, deck, &split)?;
        // Le libellé est aussi le message du change Loro : avec
        // `set_change_merge_interval(-1)`, un commit fait un change.
        self.doc.set_next_commit_message(label);
        self.doc.commit();
        // Un tour de l'agent (groupe ouvert) est une étape sous le libellé du
        // groupe ; chaque change Loro garde le libellé de l'opération.
        self.undo.close_step(label);
        Ok(())
    }

    /// Amène le replica à l'état de `deck`, sans étape d'annulation.
    pub fn apply_silent(&mut self, deck: &Deck) -> Result<(), ReplicaError> {
        let doc = &self.doc;
        let commit = || {
            doc.set_next_commit_origin(undo::SILENT);
            doc.commit();
        };
        Self::reconcile_doc(doc, deck, &commit)?;
        commit();
        Ok(())
    }

    fn reconcile_doc(doc: &LoroDoc, deck: &Deck, split: write::Split<'_>) -> Result<(), ReplicaInner> {
        let json = serde_json::to_value(deck).context(DeckJsonSnafu)?;
        // `Deck` sérialise toujours en objet ; un autre cas serait un bogue de
        // `Serialize`, jamais un deck vide à réconcilier en silence.
        let obj = match json {
            serde_json::Value::Object(o) => o,
            _ => return NotADeckSnafu { what: "deck did not serialize to a JSON object" }.fail(),
        };
        write::sync_map(&doc.get_map(ROOT), &obj, Ctx::Deck, split)
    }

    /// Ouvre un groupe : les applications qui suivent forment une seule étape
    /// d'annulation, portant `label`. Un groupe encore ouvert est d'abord fermé.
    pub fn begin_group(&mut self, label: &str) {
        self.undo.begin_group(label);
    }

    /// Ferme le groupe ouvert (sans effet s'il n'y en a pas).
    pub fn end_group(&mut self) {
        self.undo.end_group();
    }

    pub fn in_group(&self) -> bool {
        self.undo.in_group()
    }

    /// Défait la dernière étape locale ; `false` s'il n'y en avait pas.
    /// Ferme d'abord le groupe ouvert.
    pub fn undo(&mut self) -> Result<bool, ReplicaError> {
        Ok(self.undo.undo().context(LoroSnafu)?)
    }

    /// Rétablit la dernière étape locale défaite ; `false` s'il n'y en avait pas.
    /// Ferme d'abord le groupe ouvert.
    pub fn redo(&mut self) -> Result<bool, ReplicaError> {
        Ok(self.undo.redo().context(LoroSnafu)?)
    }

    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    /// Libellé de la prochaine étape défaite par [`Self::undo`], s'il y en a une.
    pub fn undo_label(&self) -> Option<String> {
        self.undo.top_undo()
    }

    /// Libellé de la prochaine étape rétablie par [`Self::redo`], s'il y en a une.
    pub fn redo_label(&self) -> Option<String> {
        self.undo.top_redo()
    }

    /// Nombre maximal d'étapes d'annulation conservées (les plus anciennes sont
    /// perdues). `0` vaut `1` : une étape au moins reste annulable.
    pub fn set_undo_limit(&mut self, steps: usize) {
        self.undo.set_limit(steps);
    }

    /// Nombre d'ops de l'historique (tests, mesures).
    pub fn len_ops(&self) -> usize {
        self.doc.len_ops()
    }

    /// Fusionne `other` dans `self` : seuls les changements que `self` n'a pas
    /// encore sont importés. Refuse deux replicas sans `deck_uid` commun.
    /// L'import et la relecture complète sont d'abord essayés sous garde sur
    /// une copie : une panique de Loro donne `Corrupted` et laisse `self` intact.
    /// Le rapport dit combien de modifications ont été importées et par qui.
    pub fn merge(&mut self, other: &DeckReplica) -> Result<MergeReport, ReplicaError> {
        Ok(self.merge_inner(other)?)
    }

    fn merge_inner(&mut self, other: &DeckReplica) -> Result<MergeReport, ReplicaInner> {
        ensure!(self.deck_uid() == other.deck_uid(), UnrelatedSnafu);
        let before = self.doc.oplog_vv();
        let updates = other.doc.export(ExportMode::updates(&before)).context(ExportSnafu)?;
        // Répétition sous garde, sur une copie : importer puis relire tout le
        // deck. Une panique n'empoisonne que la copie ; `self` reste intact.
        // L'import dans `self`, identique et déterministe, suit sans garde.
        let trial = self.doc.fork();
        configure(&trial);
        guarded(trial, |doc| {
            doc.import(&updates).context(ImportSnafu)?;
            Ok(read_deck(doc))
        })?;
        self.doc.import(&updates).context(ImportSnafu)?;
        Ok(authors::report_since(&self.doc, &before))
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
