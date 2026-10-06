//! Les versions de thèmes dans le zip des sources : `themes/<hex>/theme.json`.
//!
//! Une version de thème est un [`PackFiles`] sans widget dont le manifeste
//! est `theme.json` ; elle est vérifiée par son empreinte à la lecture, comme
//! une version de pack. Un chemin invalide (dossier autre que 64 hex
//! minuscules, sous-dossier, fichier autre que `theme.json`) refuse le zip.

use std::collections::BTreeMap;

use snafu::ensure;

use crate::error::{BadNameSnafu, SourcesInner};
use crate::model::Deck;
use crate::pack::{PackFiles, PackVersion};
use crate::validate::Issue;

/// Dossier des versions de thèmes dans le zip.
pub(super) const THEME_VERSIONS_DIR: &str = "themes/";

/// Le manifeste d'une version de thème, dans son dossier.
pub(super) const THEME_JSON: &str = "theme.json";

/// `themes/<hex>/…` : l'empreinte d'un `theme.json`, ou `None` pour une
/// entrée de répertoire nue (`themes/<hex>/`), ni lue ni comptée. Tout autre
/// chemin est refusé.
pub(super) fn theme_entry(name: &str) -> Result<Option<&str>, SourcesInner> {
    let rest = name.strip_prefix(THEME_VERSIONS_DIR).unwrap_or(name);
    let (hex, file) = rest.split_once('/').unwrap_or((rest, ""));
    ensure!(PackVersion::from_hex(hex).is_some(), BadNameSnafu { name });
    match file {
        "" => Ok(None),
        THEME_JSON => Ok(Some(hex)),
        _ => BadNameSnafu { name }.fail(),
    }
}

/// Les `theme.json` lus, par empreinte annoncée : vérifiés, et ceux dont le
/// contenu ne donne pas l'empreinte (ou n'est pas un objet JSON) écartés.
pub(super) fn verify_themes(raw: BTreeMap<String, Vec<u8>>) -> (BTreeMap<PackVersion, PackFiles>, Vec<Issue>) {
    let mut themes = BTreeMap::new();
    let mut mismatched = Vec::new();
    for (hex, json) in raw {
        match PackFiles::verified(&hex, &json, []) {
            Some((version, files)) => {
                themes.insert(version, files);
            }
            None => {
                tracing::warn!(version = %hex, "version de thème altérée écartée");
                mismatched.push(Issue::theme_version_mismatch(&hex));
            }
        }
    }
    (themes, mismatched)
}

/// La version de thème du deck manque-t-elle dans les sources ?
pub(super) fn theme_issues(deck: &Deck, themes: &BTreeMap<PackVersion, PackFiles>) -> Vec<Issue> {
    match &deck.theme {
        Some(r) if !themes.contains_key(&r.version) => vec![Issue::theme_version_missing(&r.version)],
        _ => Vec::new(),
    }
}
