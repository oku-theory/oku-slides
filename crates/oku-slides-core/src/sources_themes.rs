//! Les versions de thèmes dans le zip des sources : `themes/<hex>/theme.json`.
//!
//! Une version de thème est un [`PackFiles`] de thème ([`PackFiles::theme`] :
//! sans préfixe canonique ni widget, manifeste `theme.json`) ; elle est
//! vérifiée par son empreinte à la lecture, comme une version de pack. Un chemin invalide (dossier autre que 64 hex
//! minuscules, sous-dossier, fichier autre que `theme.json`) refuse le zip.
//! Une version écartée est signalée une seule fois, comme celles des packs.

use std::collections::BTreeMap;
use std::io::{Seek, Write};

use snafu::{OptionExt, ResultExt};
use zip::write::SimpleFileOptions;

use crate::error::{BadNameSnafu, IoSnafu, JsonSnafu, SourcesInner, ZipSnafu};
use crate::model::Deck;
use crate::pack::{PackFiles, PackVersion, ThemeManifest};
use crate::theme_chain::{ThemeChainError, check_closure};
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
        if let Some(files) = PackFiles::verified_theme(version.hex(), &json) {
            themes.insert(version, files);
        } else {
            tracing::warn!(%version, "version de thème altérée écartée");
            discarded.push(version);
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

/// Chaque thème épinglé par le deck : version embarquée, manifeste conforme
/// (espace de noms, id et `extends` de l'épinglage). Une version absente n'est
/// signalée qu'une fois, même épinglée sous deux espaces de noms. Des
/// épinglages qui ne sont pas la fermeture du thème (fusion concurrente de deux
/// `set_theme`) sont signalés : la session répare la fermeture.
pub(super) fn theme_issues(deck: &Deck, themes: &BTreeMap<PackVersion, PackFiles>) -> Vec<Issue> {
    let mut issues = Vec::new();
    if let Err(e) = check_closure(deck.theme.as_ref(), &deck.themes) {
        let culprit = match &e {
            ThemeChainError::Missing { namespace } | ThemeChainError::Extra { namespace } => Some(namespace),
            ThemeChainError::Cycle { .. } => deck.theme.as_ref(),
        };
        // Un cycle a toujours un thème : `check_closure` sans thème ne rend que `Extra`.
        if let Some(ns) = culprit {
            issues.push(Issue::theme_closure(ns));
        }
    }
    for (ns, pin) in &deck.themes {
        let Some(files) = themes.get(&pin.version) else {
            let missing = Issue::theme_version_missing(&pin.version);
            if !issues.contains(&missing) {
                issues.push(missing);
            }
            continue;
        };
        match ThemeManifest::from_files(files) {
            Ok(m) if m.namespace == *ns && m.id == pin.id && m.extends == pin.extends => {}
            _ => issues.push(Issue::theme_pin_mismatch(ns)),
        }
    }
    issues
}
