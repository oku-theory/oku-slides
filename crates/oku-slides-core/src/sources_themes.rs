//! Les versions de thèmes dans le zip des sources : `themes/<hex>/theme.json`.
//!
//! Une version de thème est un [`PackFiles`] sans widget dont le manifeste
//! est `theme.json` ; elle est vérifiée par son empreinte à la lecture, comme
//! une version de pack. Un chemin invalide (dossier autre que 64 hex
//! minuscules, sous-dossier, fichier autre que `theme.json`) refuse le zip.
//! Une version écartée est signalée une seule fois, comme celles des packs.

use std::collections::BTreeMap;
use std::io::{Seek, Write};

use snafu::{OptionExt, ResultExt};
use zip::write::SimpleFileOptions;

use crate::error::{BadNameSnafu, IoSnafu, JsonSnafu, SourcesInner, ZipSnafu};
use crate::model::Deck;
use crate::pack::{PackFiles, PackVersion};
use crate::validate::Issue;

/// Dossier des versions de thèmes dans le zip.
pub(super) const THEME_VERSIONS_DIR: &str = "themes/";

/// Le manifeste d'une version de thème, dans son dossier.
pub(super) const THEME_JSON: &str = "theme.json";

/// `themes/<hex>/…` : la version annoncée d'un `theme.json`, ou `None` pour
/// une entrée de répertoire nue (`themes/<hex>/`), ni lue ni comptée. Tout
/// autre chemin est refusé.
pub(super) fn theme_entry(name: &str) -> Result<Option<PackVersion>, SourcesInner> {
    let rest = name.strip_prefix(THEME_VERSIONS_DIR).unwrap_or(name);
    let (hex, file) = rest.split_once('/').unwrap_or((rest, ""));
    let version = PackVersion::from_hex(hex).context(BadNameSnafu { name })?;
    match file {
        "" => Ok(None),
        THEME_JSON => Ok(Some(version)),
        _ => BadNameSnafu { name }.fail(),
    }
}

/// Les `theme.json` lus, par version annoncée : vérifiés ; ceux dont le
/// contenu ne donne pas l'empreinte (ou n'est pas un objet JSON) sont rendus à part.
pub(super) fn verify_themes(
    raw: BTreeMap<PackVersion, Vec<u8>>,
) -> (BTreeMap<PackVersion, PackFiles>, Vec<PackVersion>) {
    let mut themes = BTreeMap::new();
    let mut discarded = Vec::new();
    for (version, json) in raw {
        match PackFiles::from_bytes(&json, []).filter(|f| f.matches(&version)) {
            Some(files) => {
                themes.insert(version, files);
            }
            None => {
                tracing::warn!(%version, "version de thème altérée écartée");
                discarded.push(version);
            }
        }
    }
    (themes, discarded)
}

/// Signale chaque version de thème écartée une seule fois : altérée, et non
/// plus manquante pour le deck qui la référence.
pub(super) fn report_discarded_themes(issues: &mut Vec<Issue>, discarded: &[PackVersion]) {
    for version in discarded {
        let missing = Issue::theme_version_missing(version);
        issues.retain(|i| *i != missing);
        issues.push(Issue::theme_version_mismatch(version.hex()));
    }
}

/// Écrit chaque version sous `themes/<hex>/theme.json`.
pub(super) fn write_themes<W: Write + Seek>(
    zw: &mut zip::ZipWriter<W>,
    themes: &BTreeMap<PackVersion, PackFiles>,
    options: SimpleFileOptions,
) -> Result<(), SourcesInner> {
    for (version, files) in themes {
        zw.start_file(format!("{THEME_VERSIONS_DIR}{}/{THEME_JSON}", version.hex()), options).context(ZipSnafu)?;
        zw.write_all(&files.pack_json().context(JsonSnafu { file: THEME_JSON })?).context(IoSnafu)?;
    }
    Ok(())
}

/// La version de thème du deck manque-t-elle dans les sources ?
pub(super) fn theme_issues(deck: &Deck, themes: &BTreeMap<PackVersion, PackFiles>) -> Vec<Issue> {
    match &deck.theme {
        Some(r) if !themes.contains_key(&r.version) => vec![Issue::theme_version_missing(&r.version)],
        _ => Vec::new(),
    }
}
