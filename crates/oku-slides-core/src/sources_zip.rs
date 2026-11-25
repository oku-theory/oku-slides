//! Writes and reads back the sources zip: manifest, `deck.loro`, assets, data bases
//! (`bases/<hex>.parquet`, streamed to and from the base store) and fonts.
//!
//! Then come the pack versions (`sources_packs.rs`), the theme versions (`sources_themes.rs`)
//! and the agent usage ledger, `usage.jsonl`, read last under its own cap. The read is
//! bounded: its bytes are not trusted.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Seek, Write};
use std::sync::Arc;

use oku_slides_data::BaseId;
use oku_slides_data::limits::{MAX_BASE_BYTES, MAX_DECK_BASE_BYTES};
use snafu::{IntoError, OptionExt, ResultExt, ensure};
use zip::write::SimpleFileOptions;

use super::packs::{PackPart, RawPack, pack_dir, pack_entry, report_discarded, verify_packs, write_packs};
use super::themes::{THEME_VERSIONS_DIR, report_discarded_themes, theme_entry, verify_themes, write_themes};
use super::{SOURCES_FORMAT, SOURCES_VERSION, SourceBundle, SourceLimits};
use crate::error::{
    BadNameSnafu, BaseSnafu, IoSnafu, JsonSnafu, MissingDeckSnafu, NotSourcesSnafu, ReplicaError, ReplicaSnafu,
    SourcesInner, TooLargeSnafu, TotalTooLargeSnafu, UnsupportedVersionSnafu, ZipSnafu,
};
use crate::pack::PackVersion;
use crate::pack_kind::PackKind;
use crate::replica::DeckReplica;
use crate::sources_bases::BaseFiles;
use crate::usage::{USAGE_FILE, USAGE_MAX_BYTES, UsageLedger};
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

/// Folder of the data bases in the zip: `bases/<hex>.parquet`.
const BASES_DIR: &str = "bases/";

/// Writes one stored `bases/<hex>.parquet` entry per base the deck uses now, copied from `bases`.
fn write_bases<W: Write + Seek>(
    zw: &mut zip::ZipWriter<W>,
    bundle: &SourceBundle,
    bases: Option<&dyn BaseFiles>,
    stored: SimpleFileOptions,
) -> Result<(), SourcesInner> {
    let mut written = BTreeSet::new();
    for data in bundle.deck().data.values() {
        let Some(base) = &data.table.base else { continue };
        if !written.insert(&base.id) {
            continue;
        }
        let id = || BaseSnafu { id: base.id.as_str() };
        let store = bases.ok_or_else(|| id().into_error("no base store".into()))?;
        let mut r = store.open(&base.id).map_err(|e| id().into_error(e))?;
        zw.start_file(format!("{BASES_DIR}{}.parquet", base.id.hex()), stored).context(ZipSnafu)?;
        std::io::copy(&mut r, zw).context(IoSnafu)?;
    }
    Ok(())
}

/// The bases an opened zip put into the store, and what the deck may still hold of them.
struct BaseBudget {
    known: BTreeSet<BaseId>,
    /// Bytes left under [`MAX_DECK_BASE_BYTES`].
    left: u64,
    /// The zip's whole budget, for the error.
    total: usize,
}

impl BaseBudget {
    /// Streams entry `file` (`<hex>.parquet`) of announced size `size` into `store`, within
    /// [`MAX_BASE_BYTES`], the deck's bases and the zip's `budget`; past them: `TotalTooLarge`.
    fn put(
        &mut self,
        store: &dyn BaseFiles,
        file: &str,
        f: impl Read,
        size: usize,
        budget: &mut usize,
    ) -> Result<(), SourcesInner> {
        let total = self.total;
        let hex = file.strip_suffix(".parquet").context(BadNameSnafu { name: file })?;
        let id = BaseId::try_from(format!("sha256:{hex}")).ok().context(BadNameSnafu { name: file })?;
        let cap = MAX_BASE_BYTES.min(self.left).min(u64::try_from(*budget).unwrap_or(u64::MAX));
        ensure!(u64::try_from(size).unwrap_or(u64::MAX) <= cap, TotalTooLargeSnafu { max: total });
        let mut counted = Counted { inner: f.take(cap.saturating_add(1)), read: 0 };
        let put = store.put(&mut counted, &id, cap);
        ensure!(counted.read <= cap, TotalTooLargeSnafu { max: total });
        put.map_err(|e| BaseSnafu { id: id.as_str() }.into_error(e))?;
        *budget -= usize::try_from(counted.read).unwrap_or(usize::MAX);
        self.left -= counted.read;
        self.known.insert(id);
        Ok(())
    }
}

/// A reader that counts what it gave.
struct Counted<R> {
    inner: R,
    read: u64,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n as u64;
        Ok(n)
    }
}

/// Reads `usage.jsonl`, last, under [`USAGE_MAX_BYTES`] only; never fails the deck.
///
/// Its cap is its own, outside `SourceLimits.total_bytes`: a deck whose assets fill the budget
/// keeps its ledger, so its next save does not write it out empty.
fn read_usage<R: Read + Seek>(za: &mut zip::ZipArchive<R>) -> (UsageLedger, Vec<Issue>) {
    let dropped = |issue: Issue| (UsageLedger::default(), vec![issue]);
    if za.index_for_name(USAGE_FILE).is_none() {
        return (UsageLedger::default(), Vec::new());
    }
    // An encrypted entry or an unsupported compression method is refused here.
    let f = match za.by_name(USAGE_FILE) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(error = %e, "usage.jsonl unreadable");
            return dropped(Issue::usage_unreadable());
        }
    };
    if f.size() > USAGE_MAX_BYTES {
        return dropped(Issue::usage_too_large());
    }
    let cap = usize::try_from(USAGE_MAX_BYTES).unwrap_or(usize::MAX);
    let announced = usize::try_from(f.size()).unwrap_or(cap);
    let mut data = Vec::with_capacity(announced.min(MAX_INITIAL_CAPACITY));
    match f.take(USAGE_MAX_BYTES + 1).read_to_end(&mut data) {
        // A size header that lied: more bytes than announced.
        Ok(_) if data.len() > cap => dropped(Issue::usage_too_large()),
        Ok(_) => UsageLedger::from_jsonl(&data),
        // A CRC mismatch surfaces here, at the end of the entry.
        Err(e) => {
            tracing::warn!(error = %e, "usage.jsonl unreadable");
            dropped(Issue::usage_unreadable())
        }
    }
}

impl SourceBundle {
    pub(super) fn write_zip(&self, usage: bool, bases: Option<&dyn BaseFiles>) -> Result<Vec<u8>, SourcesInner> {
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

            for (id, bytes) in self.assets.iter() {
                zw.start_file(format!("assets/{}", asset_name(id)?), stored).context(ZipSnafu)?;
                zw.write_all(bytes).context(IoSnafu)?;
            }
            write_bases(&mut zw, self, bases, stored)?;
            for (name, bytes) in &self.fonts {
                zw.start_file(format!("fonts/{}", safe_name(name)?), stored).context(ZipSnafu)?;
                zw.write_all(bytes).context(IoSnafu)?;
            }
            write_packs(&mut zw, &self.packs, deflate)?;
            write_themes(&mut zw, &self.themes, deflate)?;
            if usage && !self.usage.is_empty() {
                zw.start_file(USAGE_FILE, deflate).context(ZipSnafu)?;
                zw.write_all(&self.usage.to_jsonl()).context(IoSnafu)?;
            }
            zw.finish().context(ZipSnafu)?;
        }
        Ok(buf.into_inner())
    }

    pub(super) fn read_zip(
        bytes: &[u8],
        limits: SourceLimits,
        registry: Arc<WidgetRegistry>,
        bases: Option<&dyn BaseFiles>,
    ) -> Result<(Self, Vec<Issue>), SourcesInner> {
        let max = limits.replica_bytes;
        let mut za = zip::ZipArchive::new(Cursor::new(bytes)).context(ZipSnafu)?;
        let mut manifest: Option<Manifest> = None;
        let mut snapshot: Option<Snapshot> = None;
        let (mut assets, mut fonts) = (BTreeMap::new(), BTreeMap::new());
        let mut raw_packs: BTreeMap<(PackKind, PackVersion), RawPack> = BTreeMap::new();
        let mut raw_themes: BTreeMap<PackVersion, Vec<u8>> = BTreeMap::new();
        // Octets décompressés encore permis (anti-bombe de décompression).
        let mut budget = limits.total_bytes;
        let mut received = BaseBudget { known: BTreeSet::new(), left: MAX_DECK_BASE_BYTES, total: limits.total_bytes };
        for i in 0..za.len() {
            // `usage.jsonl` is read after the loop (`read_usage`): skipped by name before
            // `by_index` opens it, since an entry it cannot open (encrypted) must not fail the deck.
            if za.name_for_index(i).and_then(Result::ok).as_deref() == Some(USAGE_FILE) {
                continue;
            }
            let f = za.by_index(i).context(ZipSnafu)?;
            let name = f.name().context(ZipSnafu)?.into_owned();
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
            let kind = pack_dir(&name);
            if name == THEME_VERSIONS_DIR || kind.is_some_and(|k| name.strip_prefix(k.files_dir()) == Some("/")) {
                continue; // Entrée de répertoire nue, comme `widgets/` ou `themes/`.
            }
            if name.starts_with(THEME_VERSIONS_DIR) {
                let Some(version) = theme_entry(&name)? else { continue };
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                raw_themes.insert(version, data);
                continue;
            }
            if let Some(kind) = kind {
                let (version, part) = pack_entry(kind, &name)?;
                if matches!(part, PackPart::Ignored) {
                    continue; // Dossier ou fichier inconnu : ni lu, ni compté.
                }
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                raw_packs.entry((kind, version)).or_default().put(part, data);
                continue;
            }
            if let Some(file) = name.strip_prefix(BASES_DIR) {
                if let Some(store) = bases {
                    received.put(store, file, f, size, &mut budget)?;
                }
                continue; // Without a store: neither read nor counted, the data read as missing.
            }
            let known = name == "manifest.json" || name.starts_with("assets/") || name.starts_with("fonts/");
            if !known {
                continue; // Entrées inconnues (dont l'ancien `deck.json`) : ni lues, ni comptées.
            }
            let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
            match name.as_str() {
                "manifest.json" => {
                    manifest = Some(serde_json::from_slice(&data).context(JsonSnafu { file: "manifest.json" })?);
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
        // Older: not a sources file this app reads, never converted (spec t-188).
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
        let (mut bundle, mut issues) = Self::from_replica(replica, read, assets, fonts, packs, themes, received.known);
        report_discarded(&mut issues, &discarded_packs);
        report_discarded_themes(&mut issues, &discarded_themes);
        let (usage, usage_issues) = read_usage(&mut za);
        bundle.usage = usage;
        issues.extend(usage_issues);
        Ok((bundle, issues))
    }
}
