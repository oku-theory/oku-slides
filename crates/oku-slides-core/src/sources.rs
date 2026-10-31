//! Le zip des sources (format `oku-slides-sources`, version 13) : le replica
//! Loro du deck (`deck.loro`), les assets d'origine, les polices, les
//! versions de packs de widgets (`widgets/<hex>/widgets.json`,
//! `widgets/<hex>/widgets/<nom>.typ`) et de layouts (`layouts/<hex>/layouts.json`,
//! `layouts/<hex>/layouts/<nom>.typ`), et les versions de thèmes
//! (`themes/<hex>/theme.json`, voir `sources_themes.rs`). C'est ce blob qui
//! est stocké dans le PDF.
//!
//! [`SourceBundle`] détient le replica ; le [`Deck`] n'en est qu'une vue,
//! remplacée par [`SourceBundle::apply`], [`SourceBundle::undo`] et
//! [`SourceBundle::redo`]. Invariant : la vue est toujours égale à la
//! relecture du replica (`replica().deck().0 == *deck()`). La relecture est
//! canonique ; la vue l'est donc aussi : [`SourceBundle::new`] met le deck
//! reçu sous forme canonique ([`canonicalize`](crate::canonicalize)), et
//! `apply` attend un deck déjà canonique (celui que rend
//! [`apply_all`](crate::apply_all)), ce que vérifie un `debug_assert!`.
//! Un zip d'une version antérieure (1 : `deck.json` ; 2 à 12) n'est plus lu.
//!
//! Le bundle porte le registre des widgets, que son replica utilise à
//! l'écriture ; la fusion et la restauration le passent à `validate`.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use snafu::{ResultExt, ensure};

use crate::error::{ReplicaSnafu, SourcesError, TooDeepSnafu};
use crate::locate::MAX_SLOT_DEPTH;
use crate::model::Deck;
use crate::ops::Moved;
use crate::pack::{PackFiles, PackManifest, PackVersion};
use crate::pack_kind::PackKind;
use crate::replica::{DeckRead, DeckReplica, HistoryPreview, ReplicaLimits};
use crate::usage::UsageLedger;
use crate::validate::{Issue, validate};
use crate::widget::WidgetRegistry;

#[path = "sources_packs.rs"]
mod packs;
#[path = "sources_themes.rs"]
mod themes;
#[path = "sources_zip.rs"]
mod zip_io;

use themes::theme_issues;

/// Value of the `format` field in a source zip's manifest.
pub const SOURCES_FORMAT: &str = "oku-slides-sources";
/// Version of the source format; follows each change of `REPLICA_FORMAT`.
pub const SOURCES_VERSION: u32 = 13;

/// Plafonds de lecture d'un zip de sources (octets non fiables).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLimits {
    /// Taille maximale de `deck.loro` (réglage `[sources] max_replica_mib`).
    pub replica_bytes: usize,
    /// Taille décompressée maximale de tout ce qui est lu du zip : manifeste,
    /// `deck.loro`, assets, polices, packs, thèmes (réglage `[sources] max_sources_mib`).
    pub total_bytes: usize,
    /// Délai de compaction, en secondes (réglage `[sources] compact_after_days`) ; 0 : jamais.
    pub compact_after_secs: i64,
}

impl SourceLimits {
    /// Les limites d'ouverture du replica.
    #[must_use]
    pub fn replica(&self) -> ReplicaLimits {
        ReplicaLimits { max_bytes: self.replica_bytes, compact_after_secs: self.compact_after_secs }
    }
}

/// Identifiant d'un asset : `sha256:<hex>` de son contenu.
#[must_use]
pub fn asset_id(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

/// Le deck (vue du replica), ses assets et ses polices.
#[derive(Debug)]
pub struct SourceBundle {
    replica: DeckReplica,
    /// Vue matérialisée du replica, égale à sa relecture : jamais modifiée hors
    /// de `apply`, `apply_silent`, `undo`, `redo`, `restore`, `merge`.
    deck: Deck,
    /// Assets par identifiant (`sha256:…`), en ajout seul.
    pub assets: BTreeMap<String, Vec<u8>>,
    /// Polices par nom de fichier, en ajout seul.
    pub fonts: BTreeMap<String, Vec<u8>>,
    /// Versions de packs de modèles par empreinte, en ajout seul : celle du
    /// deck et toutes celles que son historique a référencées.
    pub packs: BTreeMap<PackVersion, PackFiles>,
    /// Versions de thèmes par empreinte, en ajout seul : celle du deck et
    /// toutes celles que son historique a référencées.
    pub themes: BTreeMap<PackVersion, PackFiles>,
    /// Agent usage by turn, add-only (`usage.jsonl`); never part of the deck or the replica.
    pub usage: UsageLedger,
}

/// Two bundles are equal when they show the same deck, assets, fonts, packs, themes and usage.
///
/// The replica's history does not count.
impl PartialEq for SourceBundle {
    fn eq(&self, other: &Self) -> bool {
        self.deck == other.deck
            && self.assets == other.assets
            && self.fonts == other.fonts
            && self.packs == other.packs
            && self.themes == other.themes
            && self.usage == other.usage
    }
}

impl SourceBundle {
    /// A new bundle: new replica (new `deck_uid`), with no asset, font, pack or theme.
    ///
    /// Its usage ledger (`usage.jsonl`) starts empty too. The deck is first put in canonical form (which repairs duplicate
    /// ids: an exact copy is dropped), then its host parameters are
    /// normalized as by the ops (row and column ids
    /// set: a JSON deck also merges cell by cell), then it is
    /// put back in canonical form. Deduplication must precede
    /// normalization, which would set different random ids on each
    /// copy, and the read-back would no longer see an exact copy there. A deck where an element
    /// exceeds [`MAX_SLOT_DEPTH`] is refused: reading the replica back
    /// would stop it at that bound, and the view would differ.
    ///
    /// # Errors
    ///
    /// Element beyond [`MAX_SLOT_DEPTH`], or replica that refuses the deck.
    pub fn new(mut deck: Deck, registry: Arc<WidgetRegistry>) -> Result<Self, SourcesError> {
        let too_deep = deck.slides.iter().flat_map(|s| s.walk()).any(|l| l.depth >= MAX_SLOT_DEPTH);
        ensure!(!too_deep, TooDeepSnafu { max: MAX_SLOT_DEPTH });
        crate::canonicalize(&mut deck);
        crate::ops::normalize_hosts(&registry, &mut deck);
        crate::canonicalize(&mut deck);
        let replica = DeckReplica::new(&deck, registry).context(ReplicaSnafu)?;
        let (assets, fonts, packs, themes, usage) = Default::default();
        Ok(Self { replica, deck, assets, fonts, packs, themes, usage })
    }

    /// A bundle around a replica read back by [`DeckReplica::from_snapshot`].
    ///
    /// It takes the first read the replica gave (made under guard: it is not
    /// redone here). Also returns what that read could not read, and the deck's
    /// pack and theme versions when they are missing. The usage ledger starts
    /// empty; `read_zip` sets it.
    #[must_use]
    pub fn from_replica(
        replica: DeckReplica,
        (deck, mut issues): DeckRead,
        assets: BTreeMap<String, Vec<u8>>,
        fonts: BTreeMap<String, Vec<u8>>,
        packs: BTreeMap<PackVersion, PackFiles>,
        themes: BTreeMap<PackVersion, PackFiles>,
    ) -> (Self, Vec<Issue>) {
        let bundle = Self { replica, deck, assets, fonts, packs, themes, usage: UsageLedger::default() };
        issues.extend(bundle.embedded_issues());
        (bundle, issues)
    }

    /// Chaque épinglage du deck (widgets et layouts) : version embarquée, de
    /// la sorte de sa carte, manifeste conforme, besoins épinglés parmi les
    /// packs de widgets. Une version absente n'est signalée qu'une fois, même
    /// épinglée sous deux espaces de noms. Un espace de noms épinglé sous les
    /// deux sortes (fusion, deck écrit à la main) est signalé : le rendu n'en
    /// garde qu'une.
    fn embedded_issues(&self) -> Vec<Issue> {
        let mut issues: Vec<Issue> = self
            .deck
            .widgets
            .keys()
            .filter(|ns| self.deck.layouts.contains_key(*ns))
            .map(Issue::pack_kind_conflict)
            .collect();
        let mut missing = HashSet::new();
        let maps = [(PackKind::Widgets, &self.deck.widgets), (PackKind::Layouts, &self.deck.layouts)];
        for (kind, pins) in maps {
            for (ns, pin) in pins {
                match self.packs.get(&pin.version) {
                    None => {
                        if missing.insert(&pin.version) {
                            issues.push(Issue::pack_version_missing(&pin.version));
                        }
                    }
                    Some(files) if files.kind() != kind => issues.push(Issue::pack_kind_mismatch(ns)),
                    Some(files) => match PackManifest::from_files(files) {
                        Ok(m) if m.id == pin.id && m.namespace == *ns && m.requires == pin.requires => {}
                        Err(e) => match e.token_entry() {
                            Some(entry) => issues.push(Issue::pack_tokens_invalid(ns, &entry.to_string())),
                            None => issues.push(Issue::pack_pin_mismatch(ns)),
                        },
                        Ok(_) => issues.push(Issue::pack_pin_mismatch(ns)),
                    },
                }
                for required in pin.requires.iter().filter(|r| !self.deck.widgets.contains_key(*r)) {
                    issues.push(Issue::pack_requirement_missing(ns, required));
                }
            }
        }
        issues.extend(theme_issues(&self.deck, &self.themes));
        issues
    }

    /// Ajoute une version de pack et renvoie son empreinte.
    pub fn add_pack(&mut self, files: PackFiles) -> PackVersion {
        add_versioned(&mut self.packs, files)
    }

    /// Ajoute une version de thème (`theme.json`, sans widget) et renvoie son empreinte.
    pub fn add_theme(&mut self, files: PackFiles) -> PackVersion {
        add_versioned(&mut self.themes, files)
    }

    /// The deck, canonical view equal to the replica's read-back.
    #[must_use]
    pub fn deck(&self) -> &Deck {
        &self.deck
    }

    /// The Loro replica, source of truth of the deck.
    #[must_use]
    pub fn replica(&self) -> &DeckReplica {
        &self.replica
    }

    /// Le registre des widgets du replica.
    #[must_use]
    pub fn registry(&self) -> &WidgetRegistry {
        self.replica.registry()
    }

    /// Remplace le registre des widgets (voir [`DeckReplica::set_registry`]).
    pub fn set_registry(&mut self, registry: Arc<WidgetRegistry>) {
        self.replica.set_registry(registry);
    }

    /// Nom de l'auteur de ce pair, inscrit dans `peers` à sa première édition.
    pub fn set_author(&mut self, name: Option<String>) {
        self.replica.set_author(name);
    }

    /// Replaces the deck with `deck`, in one undo step `label`.
    ///
    /// `deck` is canonical (see the module invariant). With `merge`, the application joins
    /// the top step with the same key (see `DeckReplica::apply`).
    ///
    /// # Errors
    ///
    /// Those of [`DeckReplica::apply`], or author registration refused.
    pub fn apply(&mut self, deck: Deck, label: &str, merge: Option<&str>) -> Result<(), SourcesError> {
        self.apply_moving(deck, label, merge, &[])
    }

    /// Like `apply`, keeping the node of the elements in `moves` (see
    /// [`DeckReplica::apply_moving`]).
    ///
    /// # Errors
    ///
    /// Those of [`Self::apply`].
    pub fn apply_moving(
        &mut self,
        deck: Deck,
        label: &str,
        merge: Option<&str>,
        moves: &[Moved],
    ) -> Result<(), SourcesError> {
        // Non atomique (voir `DeckReplica::apply`) : les changements déjà
        // faits restent dans le doc ; le prochain commit, ou le prochain
        // export (dont le snapshot de reprise), valide ceux en attente.
        // Avant la réconciliation : la première édition de ce pair inscrit
        // son auteur dans le même commit.
        self.replica.claim_author().context(ReplicaSnafu)?;
        self.replica.apply_moving(&deck, label, merge, moves).context(ReplicaSnafu)?;
        self.deck = deck;
        self.debug_assert_view_matches_replica();
        Ok(())
    }

    /// Replaces the deck with `deck` (canonical), without an undo step.
    ///
    /// # Errors
    ///
    /// Those of [`DeckReplica::apply_silent`].
    pub fn apply_silent(&mut self, deck: Deck) -> Result<(), SourcesError> {
        // Même non-atomicité que `apply`.
        self.replica.apply_silent(&deck).context(ReplicaSnafu)?;
        self.deck = deck;
        self.debug_assert_view_matches_replica();
        Ok(())
    }

    /// Builds de débogage seulement : la vue est la relecture du replica. Un
    /// deck non canonique passé à `apply` la ferait diverger.
    fn debug_assert_view_matches_replica(&self) {
        debug_assert!(
            self.replica.deck().0 == self.deck,
            "la vue du deck diffère de la relecture du replica : deck non canonique ?"
        );
    }

    /// Ouvre un groupe d'annulation (voir [`DeckReplica::begin_group`]).
    pub fn begin_group(&mut self, label: &str) {
        self.replica.begin_group(label);
    }

    /// Ferme le groupe d'annulation ouvert.
    pub fn end_group(&mut self) {
        self.replica.end_group();
    }

    /// Étapes d'annulation empilées (voir [`DeckReplica::undo_steps`]).
    #[must_use]
    pub fn undo_steps(&self) -> u64 {
        self.replica.undo_steps()
    }

    /// Oublie l'historique d'annulation (voir [`DeckReplica::clear_undo`]).
    pub fn clear_undo(&mut self) {
        self.replica.clear_undo();
    }

    /// Undoes the last step; `false` if there was none.
    ///
    /// # Errors
    ///
    /// Loro refuses to apply the undo.
    pub fn undo(&mut self) -> Result<bool, SourcesError> {
        let done = self.replica.undo().context(ReplicaSnafu)?;
        if done {
            self.reread();
        }
        Ok(done)
    }

    /// Redoes the last undone step; `false` if there was none.
    ///
    /// # Errors
    ///
    /// Loro refuses to apply the redo.
    pub fn redo(&mut self) -> Result<bool, SourcesError> {
        let done = self.replica.redo().context(ReplicaSnafu)?;
        if done {
            self.reread();
        }
        Ok(done)
    }

    /// Relit la vue après un annuler ou un rétablir. Les `Issue` sont écartées :
    /// elles ne peuvent venir que d'un état déjà relu (et signalé) au chargement.
    fn reread(&mut self) {
        self.deck = self.replica.deck().0;
    }

    /// Nombre maximal d'étapes d'annulation conservées ; `0` vaut `1`.
    pub fn set_undo_limit(&mut self, steps: usize) {
        self.replica.set_undo_limit(steps);
    }

    /// Les versions du deck, lisibles sur une copie.
    #[must_use]
    pub fn history(&self) -> HistoryPreview {
        self.replica.history()
    }

    /// Brings the deck back to version `index` of `preview`, in a new commit.
    ///
    /// The version is clamped to the last one, and the commit is the undoable
    /// step `label`: the deck becomes
    /// exactly `preview.deck_at(index)`, host data normalized. An
    /// empty history changes nothing.
    ///
    /// As on opening or merging, a partly read version is
    /// restored as it was read (the one the preview showed); what
    /// could not be read is returned. On failure, the replica and the view
    /// are intact, with nothing pending ([`DeckReplica::restore`]). The
    /// returned problems include those `validate` reports.
    ///
    /// # Errors
    ///
    /// Version unreadable in the preview, or that the replica refuses to write.
    pub fn restore(&mut self, preview: &HistoryPreview, index: usize, label: &str) -> Result<Vec<Issue>, SourcesError> {
        if preview.is_empty() {
            return Ok(Vec::new());
        }
        let (mut deck, issues) = preview.deck_at(index).context(ReplicaSnafu)?;
        // Comme `new`, que l'aperçu emprunte : sans identifiants, le jeu de
        // données serait rangé opaque (fusion grossière) et la vue différerait.
        // Pas de `canonicalize` : `deck_at` relit le replica, qui répare déjà les ids.
        crate::ops::normalize_hosts(self.replica.registry(), &mut deck);
        self.replica.restore(&deck, label).context(ReplicaSnafu)?;
        self.deck = deck;
        self.debug_assert_view_matches_replica();
        Ok(self.with_validation(issues))
    }

    /// Merges `other`: its history, then its assets, fonts, packs, themes and usage.
    ///
    /// The usage ledgers are united by turn.
    /// On equal names, ours are kept. The view is read back; its unreadable parts are
    /// returned, followed by what `validate` reports (the bundle's registry).
    ///
    /// # Errors
    ///
    /// Those of [`DeckReplica::merge`]: unrelated deck, or malformed changes.
    pub fn merge(&mut self, other: SourceBundle) -> Result<(crate::MergeReport, Vec<Issue>), SourcesError> {
        let report = self.replica.merge(&other.replica).context(ReplicaSnafu)?;
        for (id, bytes) in other.assets {
            self.assets.entry(id).or_insert(bytes);
        }
        for (name, bytes) in other.fonts {
            self.fonts.entry(name).or_insert(bytes);
        }
        for (version, files) in other.packs {
            self.packs.entry(version).or_insert(files);
        }
        for (version, files) in other.themes {
            self.themes.entry(version).or_insert(files);
        }
        let usage_issues = self.usage.union(other.usage);
        let (deck, issues) = self.replica.deck();
        self.deck = deck;
        let mut issues = self.with_validation(issues);
        issues.extend(usage_issues);
        Ok((report, issues))
    }

    /// `issues` (ce que la relecture n'a pas pu lire), puis ce que
    /// [`validate`] signale sur la vue avec le registre et les assets du
    /// bundle : un paramètre gardé par un `SetKind` concurrent, un asset
    /// manquant… ; enfin les versions de pack et de thème du deck, si elles manquent. Un
    /// problème déjà signalé ne l'est pas deux fois.
    fn with_validation(&self, mut issues: Vec<Issue>) -> Vec<Issue> {
        let assets: HashSet<String> = self.assets.keys().cloned().collect();
        for issue in validate(&self.deck, self.registry(), Some(&assets)) {
            if !issues.contains(&issue) {
                issues.push(issue);
            }
        }
        for issue in self.embedded_issues() {
            if !issues.contains(&issue) {
                issues.push(issue);
            }
        }
        issues
    }

    /// Ajoute un asset et renvoie son identifiant.
    pub fn add_asset(&mut self, bytes: Vec<u8>) -> String {
        let id = asset_id(&bytes);
        self.assets.insert(id.clone(), bytes);
        id
    }

    /// The sources zip, the usage ledger included.
    ///
    /// # Errors
    ///
    /// Tree too deep to be reopened, or zip write refused.
    pub fn to_zip(&self) -> Result<Vec<u8>, SourcesError> {
        self.to_zip_with(true)
    }

    /// The sources zip; `usage: false` leaves `usage.jsonl` out (usage not stored in decks).
    ///
    /// # Errors
    ///
    /// Tree too deep to be reopened, or zip write refused.
    pub fn to_zip_with(&self, usage: bool) -> Result<Vec<u8>, SourcesError> {
        Ok(self.write_zip(usage)?)
    }

    /// Reads a source zip back. The bytes come from a file: untrusted;
    /// `deck.loro` is read only if it fits in `limits.replica_bytes`, and the
    /// whole uncompressed in `limits.total_bytes`. Also returns what the
    /// replica read-back could not read, and the pack and
    /// theme versions dropped (content that does not yield its digest) or missing.
    /// `usage.jsonl` is read last, under its own cap [`crate::USAGE_MAX_BYTES`]
    /// outside `limits.total_bytes`; a ledger that cannot be read is reported, never an error.
    ///
    /// # Errors
    ///
    /// Unreadable zip or of another version, `deck.loro` missing, entry too
    /// large, invalid name, or replica refused.
    pub fn from_zip(
        bytes: &[u8],
        limits: SourceLimits,
        registry: Arc<WidgetRegistry>,
    ) -> Result<(Self, Vec<Issue>), SourcesError> {
        Ok(Self::read_zip(bytes, limits, registry)?)
    }
}

/// Range `files` sous son empreinte (sans écraser) et la renvoie.
fn add_versioned(map: &mut BTreeMap<PackVersion, PackFiles>, files: PackFiles) -> PackVersion {
    let version = files.version();
    map.entry(version.clone()).or_insert(files);
    version
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod sources_tests;

#[cfg(test)]
#[path = "sources_usage_tests.rs"]
mod sources_usage_tests;
