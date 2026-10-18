//! Les versions de packs dans le zip des sources : `widgets/<hex>/widgets.json` et
//! `widgets/<hex>/widgets/<nom>.typ`.
//!
//! Chaque version est vérifiée par son empreinte à la lecture ; une version
//! dont le contenu ne la donne pas est écartée et signalée une seule fois (le
//! deck qui la référence ne la dit pas en plus « manquante »). Un chemin
//! invalide (dossier autre que 64 hex minuscules, sous-dossier autre que
//! `widgets/`, nom de widget invalide) refuse le zip.

use std::collections::BTreeMap;
use std::io::{Seek, Write};

use snafu::{OptionExt, ResultExt};
use zip::write::SimpleFileOptions;

use crate::error::{BadNameSnafu, IoSnafu, JsonSnafu, SourcesInner, ZipSnafu};
use crate::pack::{PackFiles, PackVersion, WidgetFile};
use crate::validate::Issue;

/// Dossier des versions de packs dans le zip.
pub(super) const PACKS_DIR: &str = "widgets/";

/// Le manifeste d'une version de pack, dans son dossier.
const PACK_JSON: &str = "widgets.json";
/// Les fichiers de widgets d'une version de pack, dans son dossier.
const WIDGETS_DIR: &str = "widgets";

/// Fichiers d'une version de pack lus du zip, avant vérification.
#[derive(Default)]
pub(super) struct RawPack {
    pack_json: Option<Vec<u8>>,
    widgets: BTreeMap<WidgetFile, Vec<u8>>,
}

impl RawPack {
    /// Range `data`, lu pour la partie `part`.
    pub(super) fn put(&mut self, part: PackPart, data: Vec<u8>) {
        match part {
            PackPart::Manifest => self.pack_json = Some(data),
            PackPart::Widget(widget) => {
                self.widgets.insert(widget, data);
            }
            PackPart::Ignored => {}
        }
    }
}

/// Ce qu'est une entrée `widgets/<hex>/…`.
pub(super) enum PackPart {
    Manifest,
    Widget(WidgetFile),
    /// Dossier nu, ou fichier inconnu : ni lu, ni compté.
    Ignored,
}

/// `widgets/<hex>/<fichier>` : la version annoncée par le dossier (64 hex
/// minuscules, sinon refusée) et la partie. Sous `widgets/`, un nom hors de
/// [`WidgetFile`] est refusé, comme tout sous-dossier autre que `widgets/`.
pub(super) fn pack_entry(name: &str) -> Result<(PackVersion, PackPart), SourcesInner> {
    let rest = name.strip_prefix(PACKS_DIR).unwrap_or(name);
    let (hex, file) = rest.split_once('/').unwrap_or((rest, ""));
    let version = PackVersion::from_hex(hex).context(BadNameSnafu { name })?;
    let part = match file.split_once('/') {
        None if file == PACK_JSON => PackPart::Manifest,
        None => PackPart::Ignored,
        Some((WIDGETS_DIR, "")) => PackPart::Ignored,
        Some((WIDGETS_DIR, widget)) => {
            PackPart::Widget(WidgetFile::from_file_name(widget).context(BadNameSnafu { name })?)
        }
        Some(_) => return BadNameSnafu { name }.fail(),
    };
    Ok((version, part))
}

/// Les versions lues, vérifiées ; les écartées (sans `widgets.json`, ou dont le
/// contenu ne donne pas l'empreinte annoncée) sont rendues à part.
pub(super) fn verify_packs(
    raw: BTreeMap<PackVersion, RawPack>,
) -> (BTreeMap<PackVersion, PackFiles>, Vec<PackVersion>) {
    let mut packs = BTreeMap::new();
    let mut discarded = Vec::new();
    for (version, RawPack { pack_json, widgets }) in raw {
        let files = pack_json
            .and_then(|p| PackFiles::from_bytes(&p, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice()))))
            .filter(|f| f.matches(&version));
        match files {
            Some(files) => {
                packs.insert(version, files);
            }
            None => {
                tracing::warn!(%version, "version de pack altérée écartée");
                discarded.push(version);
            }
        }
    }
    (packs, discarded)
}

/// Signale chaque version écartée une seule fois : altérée, et non plus
/// manquante pour le deck qui la référence.
pub(super) fn report_discarded(issues: &mut Vec<Issue>, discarded: &[PackVersion]) {
    for version in discarded {
        let missing = Issue::pack_version_missing(version);
        issues.retain(|i| *i != missing);
        issues.push(Issue::pack_version_mismatch(version.hex()));
    }
}

/// Écrit chaque version sous `widgets/<hex>/` : `widgets.json`, puis ses widgets.
pub(super) fn write_packs<W: Write + Seek>(
    zw: &mut zip::ZipWriter<W>,
    packs: &BTreeMap<PackVersion, PackFiles>,
    options: SimpleFileOptions,
) -> Result<(), SourcesInner> {
    for (version, files) in packs {
        let dir = format!("{PACKS_DIR}{}", version.hex());
        zw.start_file(format!("{dir}/{PACK_JSON}"), options).context(ZipSnafu)?;
        zw.write_all(&files.pack_json().context(JsonSnafu { file: PACK_JSON })?).context(IoSnafu)?;
        for (name, source) in files.widgets() {
            zw.start_file(format!("{dir}/{WIDGETS_DIR}/{}", name.file_name()), options).context(ZipSnafu)?;
            zw.write_all(source.as_bytes()).context(IoSnafu)?;
        }
    }
    Ok(())
}
