//! Sonde d'un fichier de widget, compilé seul : son schéma ou ses
//! diagnostics. Un widget cassé est écarté sans emporter les autres.
//!
//! Règles :
//! - Le fichier est servi sous `/packs/sonde/widgets/<nom>.typ`, avec
//!   `_commun.typ` : la sonde ne dépend pas de la version du pack.
//! - Le nom déclaré doit être celui du fichier, sinon le fichier est refusé.
//! - Un message levé hors des fichiers du pack (source de la sonde,
//!   bibliothèque) revient au fichier : à l'appel fautif, sinon sans position.
//! - [`assemble`] forme le registre des valides ; un widget que le registre
//!   refuse (surcharge incompatible, schéma invalide) devient écarté.

use std::collections::BTreeMap;

use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use snafu::IntoError;
use typst::diag::SourceDiagnostic;
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::{WidgetFile, WidgetRegistry};

use crate::diagnostic::{Diagnostic, Severity, SourceAt};
use crate::error::{RegistryError, SchemaSnafu};
use crate::fonts::Fonts;
use crate::world::OkuSlidesWorld;

/// Identifiant de pack des sondes de fichiers.
pub const PROBE_PACK: &str = "sonde";

/// Ce que la sonde dit d'un fichier : son schéma, ou pourquoi il est écarté.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileProbe {
    /// Schéma brut du widget ; `None` pour `_commun` ou un fichier écarté.
    pub schema: Option<Json>,
    /// Erreurs et avertissements de Typst, ou du contrôle du schéma.
    pub diagnostics: Vec<Diagnostic>,
}

impl FileProbe {
    /// Un fichier écarté pour `message`, sans position.
    pub fn broken(name: &WidgetFile, message: impl Into<String>) -> Self {
        Self { schema: None, diagnostics: vec![Diagnostic::widget_error(name, message)] }
    }

    /// Vrai si aucune erreur ne l'écarte.
    pub fn is_valid(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

/// Le fichier importe-t-il `_commun` ? Lecture textuelle, assez pour le cache.
fn imports_common(source: &str) -> bool {
    source.contains("_commun.typ")
}

/// Clé de cache : nom, source, et `_commun` si le fichier l'importe.
pub fn probe_key(name: &WidgetFile, source: &str, common: Option<&str>) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(name.as_str().as_bytes());
    h.update([0]);
    h.update(source.as_bytes());
    h.update([0]);
    match common.filter(|_| imports_common(source)) {
        Some(c) => {
            h.update([1]);
            h.update(c.as_bytes());
        }
        None => h.update([0]),
    }
    h.finalize().into()
}

/// Chemin d'un fichier dans le monde de la sonde.
fn path_of(name: &WidgetFile) -> String {
    format!("/packs/{PROBE_PACK}/widgets/{}", name.file_name())
}

/// Source de la sonde : importe le fichier et émet son schéma (`_commun` : rien).
fn probe_source(name: &WidgetFile) -> String {
    if name.is_common() {
        format!("#import \"{}\": *\n", path_of(name))
    } else {
        format!("#import \"{}\": widget as candidate\n#metadata(candidate.schema)<oku-widget-file>\n", path_of(name))
    }
}

/// Compile `name` seul (avec `_commun`) ; tout échec devient un diagnostic.
pub fn probe_file(fonts: &Fonts, name: &WidgetFile, source: &str, common: Option<&str>) -> FileProbe {
    let mut world = match OkuSlidesWorld::new(fonts.clone()) {
        Ok(world) => world,
        Err(e) => return FileProbe::broken(name, e.to_string()),
    };
    let mut files = vec![(name.clone(), source)];
    if let Some(c) = common.filter(|_| !name.is_common()) {
        files.push((WidgetFile::common(), c));
    }
    for (file, text) in files {
        if let Err(e) = world.add_source(&path_of(&file), text.to_owned()) {
            return FileProbe::broken(name, e.to_string());
        }
    }
    world.set_main_text(&probe_source(name));
    let result = typst::compile::<PagedDocument>(&world);
    let mut diagnostics: Vec<Diagnostic> = result.warnings.iter().map(|d| file_diagnostic(&world, d, name)).collect();
    let doc = match result.output {
        Ok(doc) => doc,
        Err(errors) => {
            diagnostics.splice(0..0, errors.iter().map(|d| file_diagnostic(&world, d, name)));
            return FileProbe { schema: None, diagnostics };
        }
    };
    if name.is_common() {
        return FileProbe { schema: None, diagnostics };
    }
    let schema = Label::new(PicoStr::intern("oku-widget-file"))
        .and_then(|label| {
            doc.introspector().query(&Selector::Label(label)).first().and_then(|c| c.get_by_name("value").ok())
        })
        .and_then(|v| serde_json::to_value(&v).ok());
    let Some(schema) = schema else {
        diagnostics.insert(0, Diagnostic::widget_error(name, "the widget file emits no schema"));
        return FileProbe { schema: None, diagnostics };
    };
    let declared = schema.get("name").and_then(Json::as_str).unwrap_or_default().to_owned();
    if declared != name.as_str() {
        diagnostics.insert(0, misnamed(name, source, &declared));
        return FileProbe { schema: None, diagnostics };
    }
    FileProbe { schema: Some(schema), diagnostics }
}

/// Un message de la sonde, rapporté aux fichiers du pack : levé ailleurs
/// (source de la sonde, bibliothèque), il prend le premier appel situé dans
/// un fichier du pack, ou à défaut le fichier `name`, sans position.
fn file_diagnostic(world: &OkuSlidesWorld, d: &SourceDiagnostic, name: &WidgetFile) -> Diagnostic {
    let mut out = world.diagnostic(d);
    if out.widget_file().is_some() {
        return out;
    }
    let in_pack = |at: &SourceAt| at.path.starts_with(&format!("packs/{PROBE_PACK}/widgets/"));
    match d.trace.iter().find_map(|t| world.source_at(t.span).filter(in_pack)) {
        Some(at) => {
            out.place = Some(format!("{}:{}", at.path, at.line));
            out.at = Some(at);
        }
        None => {
            let hints = std::mem::take(&mut out.hints);
            out = Diagnostic { severity: out.severity, hints, ..Diagnostic::widget_error(name, out.message) };
        }
    }
    out
}

/// Le nom déclaré diffère du nom de fichier : diagnostic à la déclaration.
fn misnamed(name: &WidgetFile, source: &str, declared: &str) -> Diagnostic {
    let message = format!("the widget declares the name \"{declared}\" but its file is {}", name.file_name());
    let mut d = Diagnostic::widget_error(name, message);
    let found = (!declared.is_empty()).then(|| source.find(&format!("\"{declared}\""))).flatten();
    if let Some(byte) = found {
        let before = &source[..byte];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        d.at = Some(SourceAt { path: path_of(name).trim_start_matches('/').to_owned(), line, column });
    }
    d
}

/// Le registre de la base (`base` : sonde brute) surchargée par les widgets
/// valides de `files`. Un widget que le registre refuse y devient écarté,
/// avec un diagnostic, et l'assemblage reprend sans lui.
pub fn assemble(base: &Json, files: &mut BTreeMap<WidgetFile, FileProbe>) -> Result<WidgetRegistry, RegistryError> {
    let base = base.get("base").cloned().unwrap_or_else(|| Json::Array(Vec::new()));
    loop {
        let pack: Vec<Json> = files
            .iter()
            .filter(|(n, p)| !n.is_common() && p.is_valid())
            .filter_map(|(_, p)| p.schema.clone())
            .collect();
        let err = match WidgetRegistry::from_probe(json!({ "base": base, "pack": pack })) {
            Ok(registry) => return Ok(registry),
            Err(e) => e,
        };
        let culprit =
            err.widget().and_then(WidgetFile::parse).filter(|n| files.get(n).is_some_and(FileProbe::is_valid));
        let Some(name) = culprit else { return Err(SchemaSnafu.into_error(Box::new(err)).into()) };
        if let Some(probe) = files.get_mut(&name) {
            probe.schema = None;
            probe.diagnostics.insert(0, Diagnostic::widget_error(&name, err.to_string()));
        }
    }
}
