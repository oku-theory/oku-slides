//! Le zip des sources (format `oku-slides-sources`, version 2) : le replica
//! Loro du deck (`deck.loro`), les assets d'origine et les polices. C'est ce
//! blob qui est stocké dans le PDF.
//!
//! [`SourceBundle`] détient le replica ; le [`Deck`] n'en est qu'une vue,
//! remplacée par [`SourceBundle::apply`], [`SourceBundle::undo`] et
//! [`SourceBundle::redo`]. Invariant : la vue est toujours égale à la
//! relecture du replica (`replica().deck().0 == *deck()`). La relecture est
//! canonique ; la vue l'est donc aussi : [`SourceBundle::new`] met le deck
//! reçu sous forme canonique ([`canonicalize`](crate::canonicalize)), et
//! `apply` attend un deck déjà canonique (celui que rend
//! [`apply_all`](crate::apply_all)), ce que vérifie un `debug_assert!`.
//! Un zip de version 1 (`deck.json`) n'est plus lu.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use sha2::{Digest, Sha256};
use snafu::{OptionExt, ResultExt, ensure};
use zip::write::SimpleFileOptions;

use crate::error::{
    BadNameSnafu, IoSnafu, JsonSnafu, MissingDeckSnafu, NotSourcesSnafu, ReplicaError, ReplicaSnafu, SourcesError,
    SourcesInner, TooLargeSnafu, TotalTooLargeSnafu, UnsupportedVersionSnafu, ZipSnafu,
};
use crate::model::Deck;
use crate::replica::{DeckRead, DeckReplica};
use crate::validate::Issue;

pub const SOURCES_FORMAT: &str = "oku-slides-sources";
pub const SOURCES_VERSION: u32 = 2;

/// Entrée du replica dans le zip.
const DECK_LORO: &str = "deck.loro";

/// Capacité initiale maximale d'une entrée lue : la taille annoncée par
/// l'en-tête du zip n'est qu'une indication, non fiable.
const MAX_INITIAL_CAPACITY: usize = 1024 * 1024;

/// Plafonds de lecture d'un zip de sources (octets non fiables).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLimits {
    /// Taille maximale de `deck.loro` (réglage `[sources] max_replica_mib`).
    pub replica_bytes: usize,
    /// Taille décompressée maximale de tout ce qui est lu du zip : manifeste,
    /// `deck.loro`, assets, polices (réglage `[sources] max_sources_mib`).
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
    /// de `apply`, `apply_silent`, `undo`, `redo`.
    deck: Deck,
    /// Assets par identifiant (`sha256:…`), en ajout seul.
    pub assets: BTreeMap<String, Vec<u8>>,
    /// Polices par nom de fichier, en ajout seul.
    pub fonts: BTreeMap<String, Vec<u8>>,
}

/// Deux bundles sont égaux s'ils montrent le même deck, les mêmes assets et
/// les mêmes polices ; l'historique du replica n'entre pas en compte.
impl PartialEq for SourceBundle {
    fn eq(&self, other: &Self) -> bool {
        self.deck == other.deck && self.assets == other.assets && self.fonts == other.fonts
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
    /// Un bundle neuf : nouveau replica (nouveau `deck_uid`), sans asset ni police.
    /// Le deck est d'abord mis sous forme canonique.
    pub fn new(mut deck: Deck) -> Result<Self, SourcesError> {
        crate::canonicalize(&mut deck);
        let replica = DeckReplica::new(&deck).context(ReplicaSnafu)?;
        Ok(Self { replica, deck, assets: BTreeMap::new(), fonts: BTreeMap::new() })
    }

    /// Un bundle autour d'un replica relu par [`DeckReplica::from_snapshot`],
    /// avec la première lecture qu'il a rendue (faite sous garde : elle n'est
    /// pas refaite ici). Renvoie aussi ce que cette lecture n'a pas pu lire.
    pub fn from_replica(
        replica: DeckReplica,
        (deck, issues): DeckRead,
        assets: BTreeMap<String, Vec<u8>>,
        fonts: BTreeMap<String, Vec<u8>>,
    ) -> (Self, Vec<Issue>) {
        (Self { replica, deck, assets, fonts }, issues)
    }

    pub fn deck(&self) -> &Deck {
        &self.deck
    }

    pub fn replica(&self) -> &DeckReplica {
        &self.replica
    }

    /// Nom de l'auteur de ce pair, inscrit dans `peers` à sa première édition.
    pub fn set_author(&mut self, name: Option<String>) {
        self.replica.set_author(name);
    }

    /// Remplace le deck par `deck` (canonique : voir l'invariant du module),
    /// en une étape d'annulation `label`.
    pub fn apply(&mut self, deck: Deck, label: &str) -> Result<(), SourcesError> {
        // Non atomique : si la réconciliation échoue en cours de route, les
        // changements déjà faits restent en attente dans le doc, sans commit.
        // Le prochain commit, ou le prochain export (dont le snapshot de
        // reprise), les valide implicitement.
        // Avant la réconciliation : la première édition de ce pair inscrit
        // son auteur dans le même commit.
        self.replica.claim_author().context(ReplicaSnafu)?;
        self.replica.reconcile(&deck).context(ReplicaSnafu)?;
        self.replica.commit(label);
        self.deck = deck;
        self.debug_assert_view_matches_replica();
        Ok(())
    }

    /// Remplace le deck par `deck` (canonique), sans étape d'annulation.
    pub fn apply_silent(&mut self, deck: Deck) -> Result<(), SourcesError> {
        // Même non-atomicité que `apply`.
        self.replica.reconcile(&deck).context(ReplicaSnafu)?;
        self.replica.commit_silent();
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

    /// Fusionne `other` : son historique, puis ses assets et polices (à nom égal,
    /// on garde les nôtres). La vue est relue ; ses parties illisibles sont rendues.
    pub fn merge(&mut self, other: SourceBundle) -> Result<(crate::MergeReport, Vec<Issue>), SourcesError> {
        let report = self.replica.merge(&other.replica).context(ReplicaSnafu)?;
        for (id, bytes) in other.assets {
            self.assets.entry(id).or_insert(bytes);
        }
        for (name, bytes) in other.fonts {
            self.fonts.entry(name).or_insert(bytes);
        }
        let (deck, issues) = self.replica.deck();
        self.deck = deck;
        Ok((report, issues))
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
            zw.finish().context(ZipSnafu)?;
        }
        Ok(buf.into_inner())
    }

    /// Relit un zip de sources. Les octets viennent d'un fichier : non fiables ;
    /// `deck.loro` n'est lu que s'il tient dans `limits.replica_bytes`, et le
    /// tout décompressé dans `limits.total_bytes`. Renvoie aussi ce que la
    /// relecture du replica n'a pas pu lire.
    pub fn from_zip(bytes: &[u8], limits: SourceLimits) -> Result<(Self, Vec<Issue>), SourcesError> {
        Ok(Self::read_zip(bytes, limits)?)
    }

    fn read_zip(bytes: &[u8], limits: SourceLimits) -> Result<(Self, Vec<Issue>), SourcesInner> {
        let max = limits.replica_bytes;
        let mut za = zip::ZipArchive::new(Cursor::new(bytes)).context(ZipSnafu)?;
        let mut manifest: Option<Manifest> = None;
        let mut snapshot: Option<Snapshot> = None;
        let (mut assets, mut fonts) = (BTreeMap::new(), BTreeMap::new());
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
            Snapshot::Bytes(data) => DeckReplica::from_snapshot(&data, max).context(ReplicaSnafu)?,
            Snapshot::TooLarge(size) => {
                return Err(ReplicaError::from(TooLargeSnafu { size, max }.build())).context(ReplicaSnafu);
            }
        };
        Ok(Self::from_replica(replica, read, assets, fonts))
    }
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod sources_tests;
