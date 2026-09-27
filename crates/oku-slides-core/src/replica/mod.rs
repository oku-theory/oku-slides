//! Le deck dans un document Loro (CRDT) : historique complet et fusion.
//!
//! - [`DeckReplica`] : création, snapshot, lecture ([`DeckReplica::deck`]),
//!   réconciliation avec un `Deck` modifié, commit.
//! - Le `Deck` n'est qu'une vue ; le replica fait foi.

mod read;
mod rich;
mod schema;
mod undo;
mod write;

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

/// Le deck, son historique complet, et son identité (`deck_uid`).
pub struct DeckReplica {
    doc: LoroDoc,
    undo: Undo,
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
        Self::reconcile_doc(&doc, deck)?;
        doc.commit();
        // `Undo` est construit après ce commit initial : créer le deck n'est pas annulable.
        let undo = Undo::new(&doc);
        Ok(Self { doc, undo })
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
        Ok((Self { doc, undo }, read))
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

    /// Amène le replica à l'état de `deck` (sans committer).
    pub fn reconcile(&self, deck: &Deck) -> Result<(), ReplicaError> {
        Ok(self.reconcile_inner(deck)?)
    }

    fn reconcile_inner(&self, deck: &Deck) -> Result<(), ReplicaInner> {
        Self::reconcile_doc(&self.doc, deck)
    }

    fn reconcile_doc(doc: &LoroDoc, deck: &Deck) -> Result<(), ReplicaInner> {
        let json = serde_json::to_value(deck).context(DeckJsonSnafu)?;
        // `Deck` sérialise toujours en objet ; un autre cas serait un bogue de
        // `Serialize`, jamais un deck vide à réconcilier en silence.
        let obj = match json {
            serde_json::Value::Object(o) => o,
            _ => return NotADeckSnafu { what: "deck did not serialize to a JSON object" }.fail(),
        };
        write::sync_map(&doc.get_map(ROOT), &obj, Ctx::Deck)
    }

    /// Valide les changements en cours comme une étape d'annulation portant `label`.
    pub fn commit(&mut self, label: &str) {
        self.undo.label_next(label);
        self.doc.commit();
        // Un commit sans changement n'a rien empilé : le libellé ne doit pas fuir au suivant.
        self.undo.clear_next();
    }

    /// Valide les changements en cours sans les rendre annulables (canonicalisation
    /// au chargement, enregistrement de modèles).
    pub fn commit_silent(&mut self) {
        self.doc.set_next_commit_origin(undo::SILENT);
        self.doc.commit();
    }

    /// Défait la dernière étape locale ; `false` s'il n'y en avait pas.
    pub fn undo(&mut self) -> Result<bool, ReplicaError> {
        Ok(self.undo.manager().undo().context(LoroSnafu)?)
    }

    /// Rétablit la dernière étape locale défaite ; `false` s'il n'y en avait pas.
    pub fn redo(&mut self) -> Result<bool, ReplicaError> {
        Ok(self.undo.manager().redo().context(LoroSnafu)?)
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
        self.undo.manager().set_max_undo_steps(steps.max(1));
    }

    /// Nombre d'ops de l'historique (tests, mesures).
    pub fn len_ops(&self) -> usize {
        self.doc.len_ops()
    }

    /// Fusionne `other` dans `self` : seuls les changements que `self` n'a pas
    /// encore sont importés. Refuse deux replicas sans `deck_uid` commun.
    /// L'import et la relecture complète sont d'abord essayés sous garde sur
    /// une copie : une panique de Loro donne `Corrupted` et laisse `self` intact.
    pub fn merge(&mut self, other: &DeckReplica) -> Result<(), ReplicaError> {
        Ok(self.merge_inner(other)?)
    }

    fn merge_inner(&mut self, other: &DeckReplica) -> Result<(), ReplicaInner> {
        ensure!(self.deck_uid() == other.deck_uid(), UnrelatedSnafu);
        let updates = other.doc.export(ExportMode::updates(&self.doc.oplog_vv())).context(ExportSnafu)?;
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
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
