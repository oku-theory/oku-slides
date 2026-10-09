//! Le zip des sources (format `oku-slides-sources`, version 6) : le replica
//! Loro du deck (`deck.loro`), les assets d'origine, les polices, les
//! versions de packs de modèles (`packs/<hex>/pack.json`,
//! `packs/<hex>/widgets/<nom>.typ`) et les versions de thèmes
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
//! Un zip d'une version antérieure (1 : `deck.json` ; 2 à 5) n'est plus lu.
//!
//! Le bundle porte le registre des widgets, que son replica utilise à
//! l'écriture ; la fusion et la restauration le passent à `validate`.

use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use snafu::{OptionExt, ResultExt, ensure};
use zip::write::SimpleFileOptions;

use crate::error::{
    BadNameSnafu, IoSnafu, JsonSnafu, MissingDeckSnafu, NotSourcesSnafu, ReplicaError, ReplicaSnafu, SourcesError,
    SourcesInner, TooDeepSnafu, TooLargeSnafu, TotalTooLargeSnafu, UnsupportedVersionSnafu, ZipSnafu,
};
use crate::locate::MAX_SLOT_DEPTH;
use crate::model::Deck;
use crate::pack::{PackFiles, PackVersion, WidgetFile};
use crate::replica::{DeckRead, DeckReplica, HistoryPreview};
use crate::validate::{Issue, validate};
use crate::widget::WidgetRegistry;

#[path = "sources_themes.rs"]
mod themes;

use themes::{THEME_JSON, THEME_VERSIONS_DIR, theme_entry, theme_issues, verify_themes};

pub const SOURCES_FORMAT: &str = "oku-slides-sources";
pub const SOURCES_VERSION: u32 = 7;

/// Entrée du replica dans le zip.
const DECK_LORO: &str = "deck.loro";

/// Dossier des versions de packs dans le zip.
const PACKS_DIR: &str = "packs/";

/// Le manifeste d'une version de pack, dans son dossier.
const PACK_JSON: &str = "pack.json";
/// Les fichiers de widgets d'une version de pack, dans son dossier.
const WIDGETS_DIR: &str = "widgets";

/// Capacité initiale maximale d'une entrée lue : la taille annoncée par
/// l'en-tête du zip n'est qu'une indication, non fiable.
const MAX_INITIAL_CAPACITY: usize = 1024 * 1024;

/// Plafonds de lecture d'un zip de sources (octets non fiables).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLimits {
    /// Taille maximale de `deck.loro` (réglage `[sources] max_replica_mib`).
    pub replica_bytes: usize,
    /// Taille décompressée maximale de tout ce qui est lu du zip : manifeste,
    /// `deck.loro`, assets, polices, packs, thèmes (réglage `[sources] max_sources_mib`).
    pub total_bytes: usize,
}

/// Identifiant d'un asset : `sha256:<hex>` de son contenu.
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
}

/// Deux bundles sont égaux s'ils montrent le même deck, les mêmes assets,
/// polices, packs et thèmes ; l'historique du replica n'entre pas en compte.
impl PartialEq for SourceBundle {
    fn eq(&self, other: &Self) -> bool {
        self.deck == other.deck
            && self.assets == other.assets
            && self.fonts == other.fonts
            && self.packs == other.packs
            && self.themes == other.themes
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Manifest {
    format: String,
    version: u32,
}

/// `deck.loro` tel que trouvé dans le zip, avant le contrôle du manifeste.
enum Snapshot {
    Bytes(Vec<u8>),
    /// Taille annoncée au-delà du maximum : l'entrée n'a pas été lue.
    TooLarge(usize),
}

fn safe_name(name: &str) -> Result<&str, SourcesInner> {
    let ok = !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
        && !name.starts_with('.');
    ensure!(ok, BadNameSnafu { name });
    Ok(name)
}

/// Nom de fichier d'un asset : l'empreinte hex de `sha256:<hex>`, rien d'autre.
fn asset_name(id: &str) -> Result<&str, SourcesInner> {
    let hex = id.strip_prefix("sha256:").unwrap_or(id);
    ensure!(!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()), BadNameSnafu { name: id });
    Ok(hex)
}

/// Fichiers d'une version de pack lus du zip, avant vérification.
#[derive(Default)]
struct RawPack {
    pack_json: Option<Vec<u8>>,
    widgets: BTreeMap<WidgetFile, Vec<u8>>,
}

/// Ce qu'est une entrée `packs/<hex>/…`.
enum PackPart {
    Manifest,
    Widget(WidgetFile),
    /// Dossier nu, ou fichier inconnu : ni lu, ni compté.
    Ignored,
}

/// `packs/<hex>/<fichier>` : l'empreinte (64 hex minuscules, sinon refusée)
/// et la partie. Sous `widgets/`, un nom hors de [`WidgetFile`] est refusé,
/// comme tout sous-dossier autre que `widgets/`.
fn pack_entry(name: &str) -> Result<(&str, PackPart), SourcesInner> {
    let rest = name.strip_prefix(PACKS_DIR).unwrap_or(name);
    let (hex, file) = rest.split_once('/').unwrap_or((rest, ""));
    ensure!(PackVersion::from_hex(hex).is_some(), BadNameSnafu { name });
    let part = match file.split_once('/') {
        None if file == PACK_JSON => PackPart::Manifest,
        None => PackPart::Ignored,
        Some((WIDGETS_DIR, "")) => PackPart::Ignored,
        Some((WIDGETS_DIR, widget)) => {
            PackPart::Widget(WidgetFile::from_file_name(widget).context(BadNameSnafu { name })?)
        }
        Some(_) => return BadNameSnafu { name }.fail(),
    };
    Ok((hex, part))
}

/// Lit une entrée du zip : au plus `cap` octets pour elle, et au plus ce qui
/// reste de `budget` (décompressé, tout le zip compris). La taille annoncée
/// `announced` ne sert qu'à la capacité initiale, plafonnée. Dépasser le
/// budget est une erreur ; la lecture s'arrête au premier octet de trop.
fn read_entry(
    f: impl Read,
    announced: usize,
    cap: usize,
    budget: &mut usize,
    total: usize,
) -> Result<Vec<u8>, SourcesInner> {
    let limit = cap.min(budget.saturating_add(1));
    let mut data = Vec::with_capacity(announced.min(limit).min(MAX_INITIAL_CAPACITY));
    f.take(u64::try_from(limit).unwrap_or(u64::MAX)).read_to_end(&mut data).context(IoSnafu)?;
    ensure!(data.len() <= *budget, TotalTooLargeSnafu { max: total });
    *budget -= data.len();
    Ok(data)
}

impl SourceBundle {
    /// Un bundle neuf : nouveau replica (nouveau `deck_uid`), sans asset, police, pack ni thème.
    /// Le deck est d'abord mis sous forme canonique, ses paramètres d'hôte
    /// normalisés comme par les ops (identifiants des lignes et des colonnes
    /// posés : un deck JSON fusionne aussi cellule par cellule). Un deck dont un élément
    /// dépasse [`MAX_SLOT_DEPTH`] est refusé : la relecture du replica
    /// l'arrêterait à cette borne, et la vue en différerait.
    pub fn new(mut deck: Deck, registry: Arc<WidgetRegistry>) -> Result<Self, SourcesError> {
        let too_deep = deck.slides.iter().flat_map(|s| s.walk()).any(|l| l.depth >= MAX_SLOT_DEPTH);
        ensure!(!too_deep, TooDeepSnafu { max: MAX_SLOT_DEPTH });
        crate::ops::normalize_hosts(&registry, &mut deck);
        crate::canonicalize(&mut deck);
        let replica = DeckReplica::new(&deck, registry).context(ReplicaSnafu)?;
        let (assets, fonts, packs, themes) = Default::default();
        Ok(Self { replica, deck, assets, fonts, packs, themes })
    }

    /// Un bundle autour d'un replica relu par [`DeckReplica::from_snapshot`],
    /// avec la première lecture qu'il a rendue (faite sous garde : elle n'est
    /// pas refaite ici). Renvoie aussi ce que cette lecture n'a pas pu lire,
    /// et les versions de pack et de thème du deck si elles manquent.
    pub fn from_replica(
        replica: DeckReplica,
        (deck, mut issues): DeckRead,
        assets: BTreeMap<String, Vec<u8>>,
        fonts: BTreeMap<String, Vec<u8>>,
        packs: BTreeMap<PackVersion, PackFiles>,
        themes: BTreeMap<PackVersion, PackFiles>,
    ) -> (Self, Vec<Issue>) {
        let bundle = Self { replica, deck, assets, fonts, packs, themes };
        issues.extend(bundle.embedded_issues());
        (bundle, issues)
    }

    /// Les versions de pack et de thème du deck manquent-elles dans les sources ?
    fn embedded_issues(&self) -> Vec<Issue> {
        let mut issues = match &self.deck.templates {
            Some(r) if !self.packs.contains_key(&r.version) => vec![Issue::pack_version_missing(&r.version)],
            _ => Vec::new(),
        };
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

    pub fn deck(&self) -> &Deck {
        &self.deck
    }

    pub fn replica(&self) -> &DeckReplica {
        &self.replica
    }

    /// Le registre des widgets du replica.
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

    /// Remplace le deck par `deck` (canonique : voir l'invariant du module),
    /// en une étape d'annulation `label`. Avec `merge`, l'application rejoint
    /// l'étape du sommet de même clé (voir `DeckReplica::apply`).
    pub fn apply(&mut self, deck: Deck, label: &str, merge: Option<&str>) -> Result<(), SourcesError> {
        // Non atomique (voir `DeckReplica::apply`) : les changements déjà
        // faits restent dans le doc ; le prochain commit, ou le prochain
        // export (dont le snapshot de reprise), valide ceux en attente.
        // Avant la réconciliation : la première édition de ce pair inscrit
        // son auteur dans le même commit.
        self.replica.claim_author().context(ReplicaSnafu)?;
        self.replica.apply(&deck, label, merge).context(ReplicaSnafu)?;
        self.deck = deck;
        self.debug_assert_view_matches_replica();
        Ok(())
    }

    /// Remplace le deck par `deck` (canonique), sans étape d'annulation.
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
    pub fn undo_steps(&self) -> u64 {
        self.replica.undo_steps()
    }

    /// Oublie l'historique d'annulation (voir [`DeckReplica::clear_undo`]).
    pub fn clear_undo(&mut self) {
        self.replica.clear_undo();
    }

    /// Défait la dernière étape ; `false` s'il n'y en avait pas.
    pub fn undo(&mut self) -> Result<bool, SourcesError> {
        let done = self.replica.undo().context(ReplicaSnafu)?;
        if done {
            self.reread();
        }
        Ok(done)
    }

    /// Rétablit la dernière étape défaite ; `false` s'il n'y en avait pas.
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
    pub fn history(&self) -> HistoryPreview {
        self.replica.history()
    }

    /// Ramène le deck à la version `index` de `preview` (bornée à la
    /// dernière), par un nouveau commit annulable `label` : le deck devient
    /// exactement `preview.deck_at(index)`, données d'hôte normalisées. Un
    /// historique vide ne change rien.
    ///
    /// Comme à l'ouverture ou à la fusion, une version lue en partie est
    /// restaurée telle qu'elle a été lue (celle que l'aperçu a montrée) ; ce
    /// qui n'a pas pu être lu est rendu. En cas d'échec, le replica et la vue
    /// sont intacts, sans rien en attente ([`DeckReplica::restore`]). Les
    /// problèmes rendus comprennent ceux que signale `validate`.
    pub fn restore(&mut self, preview: &HistoryPreview, index: usize, label: &str) -> Result<Vec<Issue>, SourcesError> {
        if preview.is_empty() {
            return Ok(Vec::new());
        }
        let (mut deck, issues) = preview.deck_at(index).context(ReplicaSnafu)?;
        // Comme `new`, que l'aperçu emprunte : sans identifiants, le jeu de
        // données serait rangé opaque (fusion grossière) et la vue différerait.
        crate::ops::normalize_hosts(self.replica.registry(), &mut deck);
        self.replica.restore(&deck, label).context(ReplicaSnafu)?;
        self.deck = deck;
        self.debug_assert_view_matches_replica();
        Ok(self.with_validation(issues))
    }

    /// Fusionne `other` : son historique, puis ses assets, polices, packs et
    /// thèmes (à nom égal, on garde les nôtres). La vue est relue ; ses parties illisibles sont
    /// rendues, suivies de ce que signale `validate` (registre du bundle).
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
        let (deck, issues) = self.replica.deck();
        self.deck = deck;
        Ok((report, self.with_validation(issues)))
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

    pub fn to_zip(&self) -> Result<Vec<u8>, SourcesError> {
        Ok(self.write_zip()?)
    }

    fn write_zip(&self) -> Result<Vec<u8>, SourcesInner> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let deflate = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            // Images et polices sont déjà compressées : on les stocke tels quels.
            let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

            zw.start_file("manifest.json", deflate).context(ZipSnafu)?;
            let manifest = Manifest { format: SOURCES_FORMAT.into(), version: SOURCES_VERSION };
            let manifest = serde_json::to_vec(&manifest).context(JsonSnafu { file: "manifest.json" })?;
            zw.write_all(&manifest).context(IoSnafu)?;

            // Le snapshot Loro se compresse encore bien (t-204, `roadmap.json` :
            // zip de 6 884 octets stocké, 5 011 compressé, soit 27 % de gain) : compressé.
            zw.start_file(DECK_LORO, deflate).context(ZipSnafu)?;
            zw.write_all(&self.replica.snapshot().context(ReplicaSnafu)?).context(IoSnafu)?;

            for (id, bytes) in &self.assets {
                zw.start_file(format!("assets/{}", asset_name(id)?), stored).context(ZipSnafu)?;
                zw.write_all(bytes).context(IoSnafu)?;
            }
            for (name, bytes) in &self.fonts {
                zw.start_file(format!("fonts/{}", safe_name(name)?), stored).context(ZipSnafu)?;
                zw.write_all(bytes).context(IoSnafu)?;
            }
            for (version, files) in &self.packs {
                let dir = format!("{PACKS_DIR}{}", version.hex());
                zw.start_file(format!("{dir}/{PACK_JSON}"), deflate).context(ZipSnafu)?;
                zw.write_all(&files.pack_json().context(JsonSnafu { file: PACK_JSON })?).context(IoSnafu)?;
                for (name, source) in files.widgets() {
                    zw.start_file(format!("{dir}/{WIDGETS_DIR}/{}", name.file_name()), deflate).context(ZipSnafu)?;
                    zw.write_all(source.as_bytes()).context(IoSnafu)?;
                }
            }
            for (version, files) in &self.themes {
                zw.start_file(format!("{THEME_VERSIONS_DIR}{}/{THEME_JSON}", version.hex()), deflate)
                    .context(ZipSnafu)?;
                zw.write_all(&files.pack_json().context(JsonSnafu { file: THEME_JSON })?).context(IoSnafu)?;
            }
            zw.finish().context(ZipSnafu)?;
        }
        Ok(buf.into_inner())
    }

    /// Relit un zip de sources. Les octets viennent d'un fichier : non fiables ;
    /// `deck.loro` n'est lu que s'il tient dans `limits.replica_bytes`, et le
    /// tout décompressé dans `limits.total_bytes`. Renvoie aussi ce que la
    /// relecture du replica n'a pas pu lire, et les versions de packs et de
    /// thèmes écartées (contenu qui ne donne pas son empreinte) ou manquantes.
    pub fn from_zip(
        bytes: &[u8],
        limits: SourceLimits,
        registry: Arc<WidgetRegistry>,
    ) -> Result<(Self, Vec<Issue>), SourcesError> {
        Ok(Self::read_zip(bytes, limits, registry)?)
    }

    fn read_zip(
        bytes: &[u8],
        limits: SourceLimits,
        registry: Arc<WidgetRegistry>,
    ) -> Result<(Self, Vec<Issue>), SourcesInner> {
        let max = limits.replica_bytes;
        let mut za = zip::ZipArchive::new(Cursor::new(bytes)).context(ZipSnafu)?;
        let mut manifest: Option<Manifest> = None;
        let mut snapshot: Option<Snapshot> = None;
        let (mut assets, mut fonts) = (BTreeMap::new(), BTreeMap::new());
        let mut raw_packs: BTreeMap<String, RawPack> = BTreeMap::new();
        let mut raw_themes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        // Octets décompressés encore permis (anti-bombe de décompression).
        let mut budget = limits.total_bytes;
        for i in 0..za.len() {
            let f = za.by_index(i).context(ZipSnafu)?;
            let name = f.name().to_owned();
            let size = usize::try_from(f.size()).unwrap_or(usize::MAX);
            if name == DECK_LORO {
                // Anti-bombe : la taille annoncée est contrôlée avant toute lecture, et
                // la lecture reste bornée si l'en-tête ment (`from_snapshot` refuse le surplus).
                snapshot = Some(if size > max {
                    Snapshot::TooLarge(size)
                } else {
                    Snapshot::Bytes(read_entry(f, size, max.saturating_add(1), &mut budget, limits.total_bytes)?)
                });
                continue;
            }
            if name == PACKS_DIR || name == THEME_VERSIONS_DIR {
                continue; // Entrée de répertoire nue, comme `packs/<hex>/`.
            }
            if name.starts_with(THEME_VERSIONS_DIR) {
                let Some(hex) = theme_entry(&name)? else { continue };
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                raw_themes.insert(hex.to_owned(), data);
                continue;
            }
            if name.starts_with(PACKS_DIR) {
                let (hex, part) = pack_entry(&name)?;
                if matches!(part, PackPart::Ignored) {
                    continue; // Dossier ou fichier inconnu : ni lu, ni compté.
                }
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                let slot = raw_packs.entry(hex.to_owned()).or_default();
                match part {
                    PackPart::Manifest => slot.pack_json = Some(data),
                    PackPart::Widget(widget) => {
                        slot.widgets.insert(widget, data);
                    }
                    PackPart::Ignored => {}
                }
                continue;
            }
            let known = name == "manifest.json" || name.starts_with("assets/") || name.starts_with("fonts/");
            if !known {
                continue; // Entrées inconnues (dont l'ancien `deck.json`) : ni lues, ni comptées.
            }
            let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
            match name.as_str() {
                "manifest.json" => {
                    manifest = Some(serde_json::from_slice(&data).context(JsonSnafu { file: "manifest.json" })?)
                }
                n if n.starts_with("assets/") => {
                    let hex = asset_name(&n["assets/".len()..])?;
                    assets.insert(format!("sha256:{hex}"), data);
                }
                n if n.starts_with("fonts/") => {
                    let file = safe_name(&n["fonts/".len()..])?.to_owned();
                    fonts.insert(file, data);
                }
                _ => {}
            }
        }
        let manifest = manifest.context(NotSourcesSnafu)?;
        ensure!(manifest.format == SOURCES_FORMAT, NotSourcesSnafu);
        // Plus ancien : format ignoré, sans conversion (spec t-188).
        ensure!(manifest.version >= SOURCES_VERSION, NotSourcesSnafu);
        ensure!(manifest.version == SOURCES_VERSION, UnsupportedVersionSnafu { version: manifest.version });
        let (replica, read) = match snapshot.context(MissingDeckSnafu)? {
            Snapshot::Bytes(data) => DeckReplica::from_snapshot(&data, max, registry).context(ReplicaSnafu)?,
            Snapshot::TooLarge(size) => {
                return Err(ReplicaError::from(TooLargeSnafu { size, max }.build())).context(ReplicaSnafu);
            }
        };
        let mut packs = BTreeMap::new();
        let mut mismatched = Vec::new();
        for (hex, RawPack { pack_json, widgets }) in raw_packs {
            let verified = pack_json
                .and_then(|p| PackFiles::verified(&hex, &p, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice()))));
            match verified {
                Some((version, files)) => {
                    packs.insert(version, files);
                }
                None => {
                    tracing::warn!(version = %hex, "version de pack altérée écartée");
                    mismatched.push(Issue::pack_version_mismatch(&hex));
                }
            }
        }
        let (themes, themes_mismatched) = verify_themes(raw_themes);
        let (bundle, mut issues) = Self::from_replica(replica, read, assets, fonts, packs, themes);
        issues.extend(mismatched);
        issues.extend(themes_mismatched);
        Ok((bundle, issues))
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
