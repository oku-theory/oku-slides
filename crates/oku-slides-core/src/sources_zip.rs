//! Écriture et relecture du zip des sources : manifeste, `deck.loro`, assets,
//! polices, puis les versions de packs (`sources_packs.rs`) et de thèmes
//! (`sources_themes.rs`). La relecture est bornée : ses octets ne sont pas fiables.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use snafu::{OptionExt, ResultExt, ensure};
use zip::write::SimpleFileOptions;

use super::packs::{PACKS_DIR, PackPart, RawPack, pack_entry, report_discarded, verify_packs, write_packs};
use super::themes::{THEME_VERSIONS_DIR, report_discarded_themes, theme_entry, verify_themes, write_themes};
use super::{SOURCES_FORMAT, SOURCES_VERSION, SourceBundle, SourceLimits};
use crate::error::{
    BadNameSnafu, IoSnafu, JsonSnafu, MissingDeckSnafu, NotSourcesSnafu, ReplicaError, ReplicaSnafu, SourcesInner,
    TooLargeSnafu, TotalTooLargeSnafu, UnsupportedVersionSnafu, ZipSnafu,
};
use crate::pack::PackVersion;
use crate::replica::DeckReplica;
use crate::validate::Issue;
use crate::widget::WidgetRegistry;

/// Entrée du replica dans le zip.
const DECK_LORO: &str = "deck.loro";

/// Capacité initiale maximale d'une entrée lue : la taille annoncée par
/// l'en-tête du zip n'est qu'une indication, non fiable.
const MAX_INITIAL_CAPACITY: usize = 1024 * 1024;

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
    pub(super) fn write_zip(&self) -> Result<Vec<u8>, SourcesInner> {
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
            write_packs(&mut zw, &self.packs, deflate)?;
            write_themes(&mut zw, &self.themes, deflate)?;
            zw.finish().context(ZipSnafu)?;
        }
        Ok(buf.into_inner())
    }

    pub(super) fn read_zip(
        bytes: &[u8],
        limits: SourceLimits,
        registry: Arc<WidgetRegistry>,
    ) -> Result<(Self, Vec<Issue>), SourcesInner> {
        let max = limits.replica_bytes;
        let mut za = zip::ZipArchive::new(Cursor::new(bytes)).context(ZipSnafu)?;
        let mut manifest: Option<Manifest> = None;
        let mut snapshot: Option<Snapshot> = None;
        let (mut assets, mut fonts) = (BTreeMap::new(), BTreeMap::new());
        let mut raw_packs: BTreeMap<PackVersion, RawPack> = BTreeMap::new();
        let mut raw_themes: BTreeMap<PackVersion, Vec<u8>> = BTreeMap::new();
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
                let Some(version) = theme_entry(&name)? else { continue };
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                raw_themes.insert(version, data);
                continue;
            }
            if name.starts_with(PACKS_DIR) {
                let (version, part) = pack_entry(&name)?;
                if matches!(part, PackPart::Ignored) {
                    continue; // Dossier ou fichier inconnu : ni lu, ni compté.
                }
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                raw_packs.entry(version).or_default().put(part, data);
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
            Snapshot::Bytes(data) => {
                DeckReplica::from_snapshot(&data, limits.replica(), registry).context(ReplicaSnafu)?
            }
            Snapshot::TooLarge(size) => {
                return Err(ReplicaError::from(TooLargeSnafu { size, max }.build())).context(ReplicaSnafu);
            }
        };
        let (packs, discarded_packs) = verify_packs(raw_packs);
        let (themes, discarded_themes) = verify_themes(raw_themes);
        let (bundle, mut issues) = Self::from_replica(replica, read, assets, fonts, packs, themes);
        report_discarded(&mut issues, &discarded_packs);
        report_discarded_themes(&mut issues, &discarded_themes);
        Ok((bundle, issues))
    }
}
