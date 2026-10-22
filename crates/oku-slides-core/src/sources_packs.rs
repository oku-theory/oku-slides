//! Les versions de packs dans le zip des sources, par sorte ([`PackKind`]) :
//! `widgets/<hex>/widgets.json` et `widgets/<hex>/widgets/<nom>.typ` pour un
//! pack de widgets, `layouts/<hex>/layouts.json` et `layouts/<hex>/layouts/<nom>.typ`
//! pour un pack de layouts.
//!
//! Chaque version est vérifiée par son empreinte à la lecture, dans la sorte
//! de son dossier ; une version dont le contenu ne la donne pas est écartée et
//! signalée une seule fois (le deck qui la référence ne la dit pas en plus
//! « manquante »). Un chemin invalide (dossier autre que 64 hex minuscules,
//! sous-dossier autre que celui des fichiers de la sorte, nom de widget
//! invalide) refuse le zip.

use std::collections::BTreeMap;
use std::io::{Seek, Write};

use snafu::{OptionExt, ResultExt};
use zip::write::SimpleFileOptions;

use crate::error::{BadNameSnafu, IoSnafu, JsonSnafu, SourcesInner, ZipSnafu};
use crate::pack::{PackFiles, PackVersion, WidgetFile};
use crate::pack_kind::PackKind;
use crate::validate::Issue;

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

/// Ce qu'est une entrée `<sorte>/<hex>/…`.
pub(super) enum PackPart {
    Manifest,
    Widget(WidgetFile),
    /// Dossier nu, ou fichier inconnu : ni lu, ni compté.
    Ignored,
}

/// La sorte dont `name` est sous le dossier (`widgets/` ou `layouts/`), si elle existe.
///
/// Le dossier d'une sorte dans le zip porte le nom de son dossier de fichiers.
pub(super) fn pack_dir(name: &str) -> Option<PackKind> {
    PackKind::ALL.into_iter().find(|k| name.strip_prefix(k.files_dir()).is_some_and(|rest| rest.starts_with('/')))
}

/// `<sorte>/<hex>/<fichier>` : la version annoncée par le dossier (64 hex
/// minuscules, sinon refusée) et la partie. Sous le dossier des fichiers de
/// la sorte, un nom hors de [`WidgetFile`] est refusé, comme tout autre sous-dossier.
pub(super) fn pack_entry(kind: PackKind, name: &str) -> Result<(PackVersion, PackPart), SourcesInner> {
    let rest = name.strip_prefix(kind.files_dir()).and_then(|r| r.strip_prefix('/')).unwrap_or(name);
    let (hex, file) = rest.split_once('/').unwrap_or((rest, ""));
    let version = PackVersion::from_hex(hex).context(BadNameSnafu { name })?;
    let part = match file.split_once('/') {
        None if file == kind.manifest_file() => PackPart::Manifest,
        None => PackPart::Ignored,
        Some((dir, "")) if dir == kind.files_dir() => PackPart::Ignored,
        Some((dir, widget)) if dir == kind.files_dir() => {
            PackPart::Widget(WidgetFile::from_file_name(widget).context(BadNameSnafu { name })?)
        }
        Some(_) => return BadNameSnafu { name }.fail(),
    };
    Ok((version, part))
}

/// Les versions lues, vérifiées dans leur sorte ; les écartées (sans manifeste,
/// ou dont le contenu ne donne pas l'empreinte annoncée) sont rendues à part.
pub(super) fn verify_packs(
    raw: BTreeMap<(PackKind, PackVersion), RawPack>,
) -> (BTreeMap<PackVersion, PackFiles>, Vec<PackVersion>) {
    let mut packs = BTreeMap::new();
    let mut discarded = Vec::new();
    for ((kind, version), RawPack { pack_json, widgets }) in raw {
        let files = pack_json
            .and_then(|p| PackFiles::from_bytes(kind, &p, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice()))))
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

/// Écrit chaque version sous `<sorte>/<hex>/` : son manifeste, puis ses widgets.
pub(super) fn write_packs<W: Write + Seek>(
    zw: &mut zip::ZipWriter<W>,
    packs: &BTreeMap<PackVersion, PackFiles>,
    options: SimpleFileOptions,
) -> Result<(), SourcesInner> {
    for (version, files) in packs {
        let kind = files.kind();
        let (files_dir, manifest) = (kind.files_dir(), kind.manifest_file());
        let dir = format!("{files_dir}/{}", version.hex());
        zw.start_file(format!("{dir}/{manifest}"), options).context(ZipSnafu)?;
        zw.write_all(&files.pack_json().context(JsonSnafu { file: manifest })?).context(IoSnafu)?;
        for (name, source) in files.widgets() {
            zw.start_file(format!("{dir}/{files_dir}/{}", name.file_name()), options).context(ZipSnafu)?;
            zw.write_all(source.as_bytes()).context(IoSnafu)?;
        }
    }
    Ok(())
}
