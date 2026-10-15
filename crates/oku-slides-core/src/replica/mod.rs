//! Le deck dans un document Loro (CRDT) : historique complet et fusion.
//!
//! - [`DeckReplica`] : création, snapshot, lecture ([`DeckReplica::deck`]),
//!   application d'un `Deck` modifié ([`DeckReplica::apply`]), annuler.
//! - Le `Deck` n'est qu'une vue ; le replica fait foi.
//! - Modules : `schema`, `read`, `write`, `tree` (le deck en un seul
//!   `LoroTree`, slides en racine ; `tree/deck`), `host` (options des hôtes),
//!   `rich`, `undo`, `authors`, `history`, `compact`, `entry` (octets non
//!   fiables : rejeu, profondeur bornée) ; tests à côté (`*_tests.rs`, dont
//!   `list_tests`, `host_tests` et `tree_tests`). Voir `ARCHITECTURE.md`.

mod authors;
mod compact;
mod entry;
mod history;
mod host;
mod read;
mod rich;
mod schema;
mod tree;
mod undo;
mod write;

pub use authors::MergeReport;

/// Limites d'ouverture d'un replica relu d'un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaLimits {
    /// Taille maximale du snapshot (réglage `[sources] max_replica_mib`).
    pub max_bytes: usize,
    /// Délai de compaction des nœuds morts, en secondes ; 0 : jamais (t-770).
    pub compact_after_secs: i64,
}

impl From<usize> for ReplicaLimits {
    /// Une limite d'octets seule, sans compaction : tests et outils.
    fn from(max_bytes: usize) -> Self {
        Self { max_bytes, compact_after_secs: 0 }
    }
}
pub use history::{HistoryPreview, SlideChanges, Version, slide_changes};

use std::sync::Arc;

use loro::{Container, ContainerTrait, ExportMode, LoroDoc, LoroValue, TreeParentId, ValueOrContainer};
use snafu::{ResultExt, ensure};

use crate::error::{
    DeckJsonSnafu, ExportSnafu, ImportSnafu, LoroSnafu, MalformedDeckTreeSnafu, NotADeckSnafu, ReplicaError,
    ReplicaInner, TooLargeSnafu, UnrelatedSnafu, UnsupportedDeckVersionSnafu, UnsupportedReplicaFormatSnafu,
};
use crate::model::{DECK_VERSION, Deck};
use crate::ops::Moved;
use crate::widget::WidgetRegistry;
use entry::{check_entry, guarded, replayable_updates};
pub(crate) use rich::SEP as PARAGRAPH_SEPARATOR;
use schema::{Ctx, SLIDE, SLIDES};
use undo::Undo;

const ROOT: &str = "deck";
const META: &str = "meta";
const DECK_UID: &str = "deck_uid";
/// Clé de `meta` : format du replica.
const FORMAT: &str = "format";
/// 3 : le deck en un seul `LoroTree`, suppression douce (ADR-0006).
/// 4 : marques datées, t-770. Le changer impose de changer `SOURCES_VERSION`.
const REPLICA_FORMAT: i64 = 4;

/// Préfixe du message d'un commit intermédiaire d'une application (avant
/// un `mov` de liste) : l'état qui suit n'est pas une version du deck.
pub(crate) const PART: &str = "oku-part:";

fn part_message(label: &str) -> String {
    format!("{PART}{label}")
}

/// Le deck, son historique complet, et son identité (`deck_uid`).
///
/// Le registre des widgets sert à l'écriture : type de chaque paramètre et
/// paramètres créés d'avance. La lecture s'en passe.
pub struct DeckReplica {
    doc: LoroDoc,
    undo: Undo,
    registry: Arc<WidgetRegistry>,
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

/// `meta.format`, s'il est un entier (`LoroValue::I64` seulement).
fn format_of(doc: &LoroDoc) -> Option<i64> {
    match doc.get_map(META).get(FORMAT) {
        Some(ValueOrContainer::Value(LoroValue::I64(n))) => Some(n),
        _ => None,
    }
}

/// Refuse un document d'un autre format.
fn check_format(doc: &LoroDoc) -> Result<(), ReplicaInner> {
    match format_of(doc) {
        Some(REPLICA_FORMAT) => Ok(()),
        found => {
            UnsupportedReplicaFormatSnafu { found: found.map_or_else(|| "none".to_owned(), |n| n.to_string()) }.fail()
        }
    }
}

/// Refuse un arbre du deck que la réconciliation ne produit jamais (R0).
///
/// Refusés : `deck.slides` qui n'est pas un arbre, `@slide` hors de la
/// racine, une marque de suppression mal formée, un `@slot` qui n'est pas une
/// carte fusionnable. Sans ce refus, la lecture cacherait ou déplacerait un
/// nœud, ou tout `apply` échouerait et le deck resterait figé. Une racine
/// sans `@slide = true` est acceptée : Loro en crée une en défaisant un `mov`.
/// Refusés aussi : un arbre autre que `deck.slides` (tout arbre s'il manque),
/// ou un nœud, même supprimé, au-delà de `MAX_TREE_DEPTH` (contrôlés en
/// premier, sur un document rejoué).
///
/// Un seul parcours itératif des nœuds (supprimés par Loro exclus), linéaire
/// en nœuds et en clés de méta.
pub(super) fn check_trees(doc: &LoroDoc) -> Result<(), ReplicaInner> {
    let tree = match doc.get_map(ROOT).get(SLIDES) {
        None => None,
        Some(ValueOrContainer::Container(Container::Tree(tree))) => Some(tree),
        Some(_) => return MalformedDeckTreeSnafu { what: "slides are not a tree" }.fail(),
    };
    // En premier, même sans arbre du deck : arbre unique, profondeur bornée (t-809).
    let deleted = check_entry(doc, tree.as_ref())?;
    let Some(tree) = tree else { return Ok(()) };
    for node in tree.nodes() {
        if deleted.contains(&node) {
            continue;
        }
        let meta = tree.get_meta(node).context(LoroSnafu)?;
        let root = tree.parent(node) == Some(TreeParentId::Root);
        for key in meta.keys() {
            if key.starts_with(schema::DELETED) {
                ensure!(tree::is_mark(&meta, &key), MalformedDeckTreeSnafu { what: "malformed deletion mark" });
            } else if &*key == SLIDE {
                ensure!(root, MalformedDeckTreeSnafu { what: "@slide outside the root" });
            } else if &*key == schema::SLOT {
                let mergeable = matches!(
                    meta.get(&key),
                    Some(ValueOrContainer::Container(Container::Map(m))) if m.id().is_mergeable()
                );
                ensure!(mergeable, MalformedDeckTreeSnafu { what: "@slot is not a mergeable map" });
            }
        }
    }
    Ok(())
}

fn read_deck(doc: &LoroDoc) -> Result<DeckRead, ReplicaInner> {
    read::deck(&doc.get_map(ROOT))
}

impl DeckReplica {
    /// Nouveau replica, avec un nouveau `deck_uid`.
    pub fn new(deck: &Deck, registry: Arc<WidgetRegistry>) -> Result<Self, ReplicaError> {
        Ok(Self::create(deck, registry)?)
    }

    fn create(deck: &Deck, registry: Arc<WidgetRegistry>) -> Result<Self, ReplicaInner> {
        let doc = LoroDoc::new();
        configure(&doc);
        // Le pair créateur est tiré au hasard sur 64 bits : un identifiant suffisant.
        let uid = format!("{:016x}", doc.peer_id());
        doc.get_map(META).insert(DECK_UID, uid).context(LoroSnafu)?;
        doc.get_map(META).insert(FORMAT, REPLICA_FORMAT).context(LoroSnafu)?;
        // Des insertions seulement : aucun `mov`, rien à découper.
        Self::reconcile_doc(&doc, deck, &registry, &|| (), None, &[])?;
        doc.commit();
        // `Undo` est construit après ce commit initial : créer le deck n'est pas annulable.
        let undo = Undo::new(&doc);
        Ok(Self { doc, undo, registry, author: None })
    }

    /// Relit un snapshot, puis compacte ses nœuds morts (t-770).
    ///
    /// Les octets viennent d'un fichier : non fiables. Un `usize` vaut une
    /// limite d'octets sans compaction.
    /// Renvoie aussi la première lecture complète du deck (celle de
    /// [`Self::deck`]), faite sous garde : Loro ne décode l'état d'un conteneur
    /// qu'au premier accès, et panique (`unwrap`) sur un état malformé.
    /// Un deck d'une autre version que [`DECK_VERSION`] est refusé. L'état du
    /// snapshot n'est jamais lu : ses opérations sont rejouées (t-809).
    pub fn from_snapshot(
        bytes: &[u8],
        limits: impl Into<ReplicaLimits>,
        registry: Arc<WidgetRegistry>,
    ) -> Result<(Self, DeckRead), ReplicaError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Ok(Self::import_snapshot(bytes, limits.into(), registry, now)?)
    }

    fn import_snapshot(
        bytes: &[u8],
        limits: ReplicaLimits,
        registry: Arc<WidgetRegistry>,
        now: i64,
    ) -> Result<(Self, DeckRead), ReplicaInner> {
        let max = limits.max_bytes;
        ensure!(bytes.len() <= max, TooLargeSnafu { size: bytes.len(), max });
        let doc = LoroDoc::new();
        configure(&doc);
        let (doc, read) = guarded(doc, |doc| {
            // L'état reçu n'est jamais lu : le document est rejoué de ses opérations (t-809).
            doc.import(&replayable_updates(bytes)?).context(ImportSnafu)?;
            ensure!(!uid_of(doc).is_empty(), NotADeckSnafu { what: "no deck_uid" });
            check_format(doc)?;
            check_trees(doc)?;
            let read = read_deck(doc)?;
            // Sous la garde : la compaction lit les changes de Loro, non fiables.
            compact::compact_doc(doc, now, limits.compact_after_secs)?;
            Ok(read)
        })?;
        // `Undo` après l'import : l'état relu n'est pas une étape annulable.
        let undo = Undo::new(&doc);
        Ok((Self { doc, undo, registry, author: None }, read))
    }

    /// Remplace le registre des widgets (pack installé ou retiré). Les
    /// paramètres nouvellement déclarés sont créés à la prochaine application.
    pub fn set_registry(&mut self, registry: Arc<WidgetRegistry>) {
        self.registry = registry;
    }

    pub fn registry(&self) -> &WidgetRegistry {
        &self.registry
    }

    /// Le snapshot à enregistrer ; `TooDeep` si sa réouverture serait refusée.
    ///
    /// Des retraits suivis de déplacements vers le bas empilent des nœuds
    /// cachés au-delà de la profondeur relue : l'enregistrement est refusé
    /// plutôt que d'écrire un fichier qui ne se rouvrirait pas (t-866).
    /// Défaire le dernier déplacement suffit à enregistrer de nouveau.
    pub fn snapshot(&self) -> Result<Vec<u8>, ReplicaError> {
        if let Some(tree) = tree::deck_tree_of(&self.doc) {
            entry::check_depth(&tree)?;
        }
        Ok(self.doc.export(ExportMode::Snapshot).context(ExportSnafu)?)
    }

    pub fn deck_uid(&self) -> String {
        uid_of(&self.doc)
    }

    /// Le deck relu depuis le replica, et ce qui n'a pas pu être lu.
    ///
    /// Tout replica ouvert ou fusionné a été relu et accepté, et toute
    /// écriture refuse un deck d'une autre version : la version se lit donc
    /// toujours. Sinon, le deck rendu est vide et la version signalée.
    pub fn deck(&self) -> DeckRead {
        read_deck(&self.doc).unwrap_or_else(|_| {
            let issue = crate::Issue {
                slide: None,
                element: None,
                message: oku_slides_i18n::Message::new("issue-unreadable-field").arg("field", "version"),
            };
            (Deck::empty(), vec![issue])
        })
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
    /// Avec `merge`, l'application rejoint l'étape du sommet si elle porte la
    /// même clé (une séance de frappe = un ⌘Z) et garde le libellé de sa
    /// première application ; sans clé, ou avec une autre, c'est une nouvelle
    /// étape. Un groupe ouvert l'emporte sur la clé (voir `undo.rs`).
    ///
    /// Non atomique : si la réconciliation échoue en cours de route, les
    /// changements déjà faits restent dans le doc ; ceux qui ne sont pas encore
    /// committés rejoignent le prochain commit, et son étape.
    pub fn apply(&mut self, deck: &Deck, label: &str, merge: Option<&str>) -> Result<(), ReplicaError> {
        self.apply_moving(deck, label, merge, &[])
    }

    /// Comme `apply`, en gardant le nœud des éléments de `moves` (ADR-0006).
    ///
    /// Chaque entrée annonce un élément passé d'une slide à l'autre (op
    /// `move_element`) : il est apparié à son nœud même si son id figure dans
    /// d'autres slides. Une entrée qui ne se résout pas est ignorée.
    pub fn apply_moving(
        &mut self,
        deck: &Deck,
        label: &str,
        merge: Option<&str>,
        moves: &[Moved],
    ) -> Result<(), ReplicaError> {
        let doc = &self.doc;
        let split = || {
            doc.set_next_commit_message(&part_message(label));
            doc.commit();
        };
        let born = || {
            doc.set_next_commit_message(&part_message(label));
            doc.set_next_commit_origin(undo::SILENT);
            doc.commit();
        };
        Self::reconcile_doc(doc, deck, &self.registry, &split, Some(&born), moves)?;
        // Le libellé est aussi le message du change Loro : avec
        // `set_change_merge_interval(-1)`, un commit fait un change.
        self.doc.set_next_commit_message(label);
        self.doc.commit();
        // Un tour de l'agent (groupe ouvert) est une étape sous le libellé du
        // groupe ; chaque change Loro garde le libellé de l'opération.
        self.undo.close_step(label, merge);
        Ok(())
    }

    /// Amène le replica à l'état de `deck`, sans étape d'annulation.
    pub fn apply_silent(&mut self, deck: &Deck) -> Result<(), ReplicaError> {
        let doc = &self.doc;
        let commit = || {
            doc.set_next_commit_origin(undo::SILENT);
            doc.commit();
        };
        Self::reconcile_doc(doc, deck, &self.registry, &commit, None, &[])?;
        commit();
        Ok(())
    }

    /// Refuse un deck d'une autre version que [`DECK_VERSION`].
    fn reconcile_doc(
        doc: &LoroDoc,
        deck: &Deck,
        registry: &WidgetRegistry,
        split: write::Split<'_>,
        born: Option<write::Split<'_>>,
        moves: &[Moved],
    ) -> Result<(), ReplicaInner> {
        ensure!(deck.version == DECK_VERSION, UnsupportedDeckVersionSnafu { found: deck.version.to_string() });
        let json = serde_json::to_value(deck).context(DeckJsonSnafu)?;
        // `Deck` sérialise toujours en objet ; un autre cas serait un bogue de
        // `Serialize`, jamais un deck vide à réconcilier en silence.
        let obj = match json {
            serde_json::Value::Object(o) => o,
            _ => return NotADeckSnafu { what: "deck did not serialize to a JSON object" }.fail(),
        };
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        write::Writer { registry, split, born, mark_key: schema::mark_key(doc.peer_id()), moves, now }.sync_map(
            &doc.get_map(ROOT),
            &obj,
            Ctx::Deck,
        )
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

    /// Étapes d'annulation empilées depuis la création, jamais décrémenté.
    ///
    /// Une application qui ne l'augmente pas a rejoint l'étape du sommet
    /// (même clé de fusion, ou groupe déjà ouvert).
    pub fn undo_steps(&self) -> u64 {
        self.undo.steps()
    }

    /// Oublie toutes les étapes d'annulation et de rétablissement.
    ///
    /// Un groupe ouvert le reste : la prochaine application y ouvre son étape.
    pub fn clear_undo(&mut self) {
        self.undo.clear();
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

    /// Les versions du deck, lisibles sur une copie.
    pub fn history(&self) -> HistoryPreview {
        history::build(&self.doc)
    }

    /// Amène le replica à `deck` (une version relue par
    /// [`HistoryPreview::deck_at`]) en une étape annulable `label`, par le
    /// même chemin que toute édition : inscription de l'auteur, puis
    /// [`Self::apply`]. Les noms d'auteurs plus récents restent : seule la
    /// racine `deck` est écrite.
    ///
    /// Pas de `revert_to` : sur une `LoroMovableList` de conteneurs (items de
    /// liste), il perd des items ou échoue (`ContainerDeleted`) dès qu'un
    /// déplacement suit la version visée.
    ///
    /// Tout ou rien, contrairement à `apply` : Loro n'a pas d'annulation d'un
    /// commit, et une réconciliation qui échoue en route laisserait des
    /// changements dans le doc. Elle est donc d'abord essayée sous garde sur
    /// une copie ; `self` n'est écrit qu'ensuite, par la même réconciliation,
    /// déterministe.
    pub fn restore(&mut self, deck: &Deck, label: &str) -> Result<(), ReplicaError> {
        let trial = self.doc.fork();
        configure(&trial);
        guarded(trial, |doc| {
            Self::reconcile_doc(doc, deck, &self.registry, &|| doc.commit(), Some(&|| doc.commit()), &[])
        })?;
        self.claim_author()?;
        self.apply(deck, label, None)
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
        // L'autre replica a été ouvert par rejeu ; l'essai refuse un arbre
        // étranger ou trop profond. L'import final dans `self` n'en récurse
        // pas (sonde t-809).
        let trial = self.doc.fork();
        configure(&trial);
        guarded(trial, |doc| {
            doc.import(&updates).context(ImportSnafu)?;
            check_format(doc)?;
            check_trees(doc)?;
            read_deck(doc)
        })?;
        self.doc.import(&updates).context(ImportSnafu)?;
        Ok(authors::report_since(&self.doc, &before))
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
#[path = "list_tests.rs"]
mod list_tests;

#[cfg(test)]
#[path = "host_tests.rs"]
mod host_tests;
