//! Historique du deck : ses versions dans l'ordre, la lecture d'une
//! version, et la comparaison d'une version avec le deck courant.
//!
//! L'ordre est un tri topologique des *changes* (Lamport, puis pair) : tout
//! préfixe est une coupe causale. Un cran montre l'état après **tous** les
//! changes du préfixe, branches concurrentes comprises.
//!
//! Un cran par édition, pas par change : une édition qui déplace des items
//! de liste ou des lignes de données fait un commit par `mov` (t-263), dont
//! seuls les intermédiaires portent le message `oku-part:<libellé>`. Ils n'ont
//! pas de cran ; celui du commit final de l'édition les contient. Les `mov`
//! de l'arbre du deck n'ajoutent aucun commit (ADR-0006). Le commit de
//! compaction (t-770) n'a pas de cran.
//!
//! Une version se lit par rejeu de ses opérations, jamais par `checkout` (t-809).

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::mem::ManuallyDrop;
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind};

use loro::{ExportMode, Frontiers, ID, LoroDoc, VersionVector};
use snafu::{OptionExt, ResultExt};

use super::{DeckRead, PART, authors, check_trees, configure, read_deck};
use crate::error::{ExportSnafu, ImportPanickedSnafu, ImportSnafu, NotADeckSnafu, ReplicaError, ReplicaInner};
use crate::model::Deck;

/// Une version : l'état juste après une édition (son dernier commit).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Version {
    pub author: Option<String>,
    /// Secondes Unix (horloge de la machine de l'auteur : affichée, jamais triée).
    pub timestamp: i64,
    /// Libellé de l'édition (« Dictée : … ») ; `None` pour la création.
    pub label: Option<String>,
    /// Première version de ce pair dans l'ordre : début d'une session d'édition.
    pub session_start: bool,
    /// Édition dont un commit réunit des branches concurrentes (après une fusion).
    pub merge: bool,
}

/// Les versions d'un deck, et une copie du document pour les lire.
pub struct HistoryPreview {
    /// Copie détachée ; jamais libérée si une lecture a paniqué (verrous empoisonnés).
    doc: Option<LoroDoc>,
    poisoned: Cell<bool>,
    versions: Vec<Version>,
    cuts: Vec<Frontiers>,
}

impl std::fmt::Debug for HistoryPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryPreview").field("versions", &self.versions.len()).finish()
    }
}

impl Drop for HistoryPreview {
    fn drop(&mut self) {
        // Libérer un document empoisonné par une panique ferait tomber l'app :
        // on le laisse fuir, comme `guarded` le fait à l'import.
        if self.poisoned.get()
            && let Some(doc) = self.doc.take()
        {
            std::mem::forget(doc);
        }
    }
}

pub(super) fn build(doc: &LoroDoc) -> HistoryPreview {
    let heads: Vec<ID> = doc.oplog_frontiers().iter().collect();
    let mut metas = Vec::new();
    // Un historique cohérent ne fait pas échouer le parcours ; en cas d'erreur,
    // la liste est simplement plus courte.
    let _ = doc.travel_change_ancestors(&heads, &mut |m| {
        metas.push(m);
        ControlFlow::Continue(())
    });
    metas.sort_by_key(|m| (m.lamport, m.id.peer));
    let (mut vv, mut seen) = (VersionVector::default(), HashSet::new());
    let (mut versions, mut cuts) = (Vec::with_capacity(metas.len()), Vec::with_capacity(metas.len()));
    // Commits intermédiaires d'une édition (un `mov` de liste par commit,
    // t-263), par pair : ils n'ont pas de cran, le commit final le porte.
    let mut parts: HashMap<u64, bool> = HashMap::new();
    let mut last_part = None;
    for m in metas {
        let len = i32::try_from(m.len).unwrap_or(i32::MAX);
        vv.extend_to_include_last_id(ID::new(m.id.peer, m.id.counter + len - 1));
        let message = m.message.as_deref().unwrap_or_default();
        if message == super::compact::COMPACT {
            continue;
        }
        let merge = m.deps.len() > 1;
        if let Some(label) = message.strip_prefix(PART) {
            *parts.entry(m.id.peer).or_default() |= merge;
            last_part = Some((m.id.peer, m.timestamp, label.to_owned()));
            continue;
        }
        let merge = parts.remove(&m.id.peer).unwrap_or_default() || merge;
        cuts.push(doc.vv_to_frontiers(&vv));
        versions.push(Version {
            author: authors::author_of(doc, m.id.peer),
            timestamp: m.timestamp,
            label: Some(message.to_owned()).filter(|s| !s.is_empty()),
            session_start: seen.insert(m.id.peer),
            merge,
        });
    }
    // Une édition interrompue (réconciliation en échec) laisse des commits
    // intermédiaires sans commit final : un dernier cran les montre.
    if let (false, Some((peer, timestamp, label))) = (parts.is_empty(), last_part) {
        cuts.push(doc.vv_to_frontiers(&vv));
        versions.push(Version {
            author: authors::author_of(doc, peer),
            timestamp,
            label: Some(label).filter(|s| !s.is_empty()),
            session_start: seen.insert(peer),
            merge: parts.into_values().any(|m| m),
        });
    }
    let copy = doc.fork();
    configure(&copy);
    HistoryPreview { doc: Some(copy), poisoned: Cell::new(false), versions, cuts }
}

impl HistoryPreview {
    pub fn versions(&self) -> &[Version] {
        &self.versions
    }

    pub fn len(&self) -> usize {
        self.versions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    /// Tests seulement : l'aperçu se comporte comme après une panique de
    /// lecture (version illisible) ; toute lecture suivante échoue.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn poison_for_tests(&self) {
        self.poisoned.set(true);
    }

    pub(super) fn cut(&self, index: usize) -> Option<&Frontiers> {
        self.cuts.get(index.min(self.cuts.len().saturating_sub(1)))
    }

    /// Le deck juste après la version `index` (bornée à la dernière).
    ///
    /// La version est rejouée dans un document neuf à partir de ses
    /// opérations (`updates_till`), contrôlée comme à l'ouverture, lue puis
    /// libérée : jamais de `checkout` (t-809). Une panique empoisonne la copie.
    pub fn deck_at(&self, index: usize) -> Result<DeckRead, ReplicaError> {
        let (Some(doc), Some(cut)) = (self.doc.as_ref(), self.cut(index)) else {
            return Ok((Deck::empty(), Vec::new()));
        };
        if self.poisoned.get() {
            return Err(ImportPanickedSnafu.build().into());
        }
        let version = ManuallyDrop::new(LoroDoc::new());
        configure(&version);
        let read = catch_unwind(AssertUnwindSafe(|| -> Result<DeckRead, ReplicaInner> {
            let vv = doc.frontiers_to_vv(cut).context(NotADeckSnafu { what: "unknown version" })?;
            let updates = doc.export(ExportMode::updates_till(&vv)).context(ExportSnafu)?;
            version.import(&updates).context(ImportSnafu)?;
            check_trees(&version)?;
            read_deck(&version)
        }));
        match read {
            Ok(read) => {
                // Une version acceptée, ou refusée avant d'être lue, se libère sans récursion.
                drop(ManuallyDrop::into_inner(version));
                Ok(read?)
            }
            Err(_) => {
                self.poisoned.set(true);
                Err(ImportPanickedSnafu.build().into())
            }
        }
    }
}

/// Les slides de `version` comparées à celles de `current`, par `id`.
pub fn slide_changes(version: &Deck, current: &Deck) -> SlideChanges {
    let now: BTreeMap<&str, _> = current.slides.iter().map(|s| (s.id.as_str(), s)).collect();
    let then: BTreeMap<&str, _> = version.slides.iter().map(|s| (s.id.as_str(), s)).collect();
    SlideChanges {
        added_since: current
            .slides
            .iter()
            .filter(|s| !then.contains_key(s.id.as_str()))
            .map(|s| s.id.clone())
            .collect(),
        removed_since: version
            .slides
            .iter()
            .filter(|s| !now.contains_key(s.id.as_str()))
            .map(|s| s.id.clone())
            .collect(),
        changed_since: version
            .slides
            .iter()
            .filter(|s| now.get(s.id.as_str()).is_some_and(|c| *c != *s))
            .map(|s| s.id.clone())
            .collect(),
    }
}

/// Ce qui distingue une version du deck courant, slide par slide.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct SlideChanges {
    /// Slides du deck courant absentes de la version.
    pub added_since: Vec<String>,
    /// Slides de la version absentes du deck courant.
    pub removed_since: Vec<String>,
    /// Slides présentes des deux côtés, mais différentes.
    pub changed_since: Vec<String>,
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;

#[cfg(test)]
mod send_test {
    use super::HistoryPreview;

    fn assert_send<T: Send>() {}

    #[test]
    fn history_preview_is_send() {
        assert_send::<HistoryPreview>();
    }
}
