//! Sonde d'un fichier de widget, compilé seul : son schéma ou ses
//! diagnostics. Un widget cassé est écarté sans emporter les autres.
//!
//! Règles :
//! - Le fichier est servi sous `/packs/sonde/<dossier>/<nom>.typ` (`widgets`
//!   ou `layouts`, selon la sorte du pack), avec `_commun.typ` : la sonde ne
//!   dépend pas de la version du pack.
//! - Le nom déclaré doit être celui du fichier, sinon le fichier est refusé.
//! - Un message levé hors des fichiers du pack (source de la sonde,
//!   bibliothèque) revient au fichier : à l'appel fautif, sinon sans position.
//! - [`assemble`] forme le registre des valides ; un widget que le registre
//!   refuse (schéma invalide, slot qui accepte un widget inconnu, catégorie
//!   contraire à la sorte du pack) devient écarté.
//! - Un fichier qui importe un autre pack ne compile pas ici : seul son `_commun`
//!   est servi, il est donc écarté comme tout fichier cassé.

use std::collections::BTreeMap;

use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use snafu::IntoError;
use typst::diag::SourceDiagnostic;
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::{Namespace, PackKind, WidgetFile, WidgetRegistry, split_kind};

use crate::diagnostic::{Diagnostic, Severity, SourceAt};
use crate::error::{MissingSnafu, RegistryError, SchemaSnafu};
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

/// Clé de cache : sorte du pack, nom, source et `_commun` (qu'il soit importé ou non).
///
/// Le contenu de `_commun` entre toujours dans la clé : un import peut ne
/// pas être écrit en toutes lettres, et toute modification de `_commun`
/// force la re-sonde de chaque widget. La sorte fixe le chemin du fichier,
/// que ses diagnostics nomment.
pub fn probe_key(kind: PackKind, name: &WidgetFile, source: &str, common: Option<&str>) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(kind.files_dir().as_bytes());
    h.update([0]);
    h.update(name.as_str().as_bytes());
    h.update([0]);
    h.update(source.as_bytes());
    h.update([0]);
    match common {
        Some(c) => {
            h.update([1]);
            h.update(c.as_bytes());
        }
        None => h.update([0]),
    }
    h.finalize().into()
}

/// Chemin d'un fichier d'un pack de sorte `kind` dans le monde de la sonde.
fn path_of(kind: PackKind, name: &WidgetFile) -> String {
    format!("/packs/{PROBE_PACK}/{}/{}", kind.files_dir(), name.file_name())
}

/// Source de la sonde : importe le fichier et émet son schéma (`_commun` : rien).
fn probe_source(kind: PackKind, name: &WidgetFile) -> String {
    let path = path_of(kind, name);
    if name.is_common() {
        format!("#import \"{path}\": *\n")
    } else {
        format!("#import \"{path}\": widget as candidate\n#metadata(candidate.schema)<oku-widget-file>\n")
    }
}

/// Compile `name`, fichier d'un pack de sorte `kind`, seul (avec `_commun`) ;
/// tout échec devient un diagnostic. La catégorie déclarée est contrôlée à
/// l'assemblage ([`assemble_set`]), pas ici.
pub fn probe_file(fonts: &Fonts, kind: PackKind, name: &WidgetFile, source: &str, common: Option<&str>) -> FileProbe {
    let mut world = match OkuSlidesWorld::new(fonts.clone()) {
        Ok(world) => world,
        Err(e) => return FileProbe::broken(name, e.to_string()),
    };
    let mut files = vec![(name.clone(), source)];
    if let Some(c) = common.filter(|_| !name.is_common()) {
        files.push((WidgetFile::common(), c));
    }
    for (file, text) in files {
        if let Err(e) = world.add_source(&path_of(kind, &file), text.to_owned()) {
            return FileProbe::broken(name, e.to_string());
        }
    }
    world.set_main_text(&probe_source(kind, name));
    let result = typst::compile::<PagedDocument>(&world);
    let mut diagnostics: Vec<Diagnostic> =
        result.warnings.iter().map(|d| file_diagnostic(&world, d, kind, name)).collect();
    let doc = match result.output {
        Ok(doc) => doc,
        Err(errors) => {
            diagnostics.splice(0..0, errors.iter().map(|d| file_diagnostic(&world, d, kind, name)));
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
        diagnostics.insert(0, misnamed(kind, name, source, &declared));
        return FileProbe { schema: None, diagnostics };
    }
    FileProbe { schema: Some(schema), diagnostics }
}

/// Un message de la sonde, rapporté aux fichiers du pack : levé ailleurs
/// (source de la sonde, bibliothèque), il prend le premier appel situé dans
/// un fichier du pack, ou à défaut le fichier `name`, sans position.
fn file_diagnostic(world: &OkuSlidesWorld, d: &SourceDiagnostic, kind: PackKind, name: &WidgetFile) -> Diagnostic {
    let mut out = world.diagnostic(d);
    if out.widget_file().is_some() {
        return out;
    }
    let dir = format!("packs/{PROBE_PACK}/{}/", kind.files_dir());
    let in_pack = |at: &SourceAt| at.path.starts_with(&dir);
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
fn misnamed(kind: PackKind, name: &WidgetFile, source: &str, declared: &str) -> Diagnostic {
    let message = format!("the widget declares the name \"{declared}\" but its file is {}", name.file_name());
    let mut d = Diagnostic::widget_error(name, message);
    let found = (!declared.is_empty()).then(|| source.find(&format!("\"{declared}\""))).flatten();
    if let Some(byte) = found {
        let before = &source[..byte];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        d.at = Some(SourceAt { path: path_of(kind, name).trim_start_matches('/').to_owned(), line, column });
    }
    d
}

/// Le registre des packs déjà sondés (`fixed` : sonde brute) et des widgets
/// valides de `files`, rangés sous l'espace de noms `namespace`, de sorte `kind`.
/// Un widget que le registre refuse y devient écarté, avec un diagnostic, et
/// l'assemblage reprend sans lui.
pub fn assemble(
    fixed: &Json,
    namespace: &Namespace,
    kind: PackKind,
    files: &mut BTreeMap<WidgetFile, FileProbe>,
) -> Result<WidgetRegistry, RegistryError> {
    assemble_set(fixed, &mut [(namespace, kind, files)])
}

/// Les fichiers sondés d'un pack, sous son espace de noms et sa sorte.
pub type ProbedPack<'a> = (&'a Namespace, PackKind, &'a mut BTreeMap<WidgetFile, FileProbe>);

/// Le registre des packs de `fixed` et des widgets valides de chaque pack de
/// `packs`, sous son espace de noms. Un widget que le registre refuse devient
/// écarté, avec un diagnostic, et l'assemblage reprend sans lui. Un nom nu dans
/// l'erreur (schéma) désigne le premier pack qui a ce fichier valide ; un nom
/// qualifié (slot, catégorie contraire à la sorte), le fichier de son pack.
///
/// Les packs déjà sondés (`packs` de la sonde `fixed`, comme `base` dans
/// [`crate::base_probe`]) entrent tels quels au registre : leurs widgets ne
/// sont jamais écartés ici.
pub fn assemble_set(fixed: &Json, packs: &mut [ProbedPack<'_>]) -> Result<WidgetRegistry, RegistryError> {
    let fixed: Vec<Json> = fixed
        .get("packs")
        .and_then(Json::as_array)
        .cloned()
        .ok_or_else(|| RegistryError::from(MissingSnafu.build()))?;
    loop {
        let entries: Vec<Json> = fixed
            .iter()
            .cloned()
            .chain(packs.iter().map(|(namespace, kind, files)| {
                let widgets: Vec<Json> = files
                    .iter()
                    .filter(|(n, p)| !n.is_common() && p.is_valid())
                    .filter_map(|(_, p)| p.schema.clone())
                    .collect();
                json!({ "namespace": namespace, "kind": kind, "widgets": widgets })
            }))
            .collect();
        let err = match WidgetRegistry::from_probe(json!({ "packs": entries })) {
            Ok(registry) => return Ok(registry),
            Err(e) => e,
        };
        // Le registre nomme le widget nu (schéma) ou qualifié (slots) : on revient au pack et au fichier.
        let valid = |files: &BTreeMap<WidgetFile, FileProbe>, w: &str| {
            WidgetFile::parse(w).filter(|n| files.get(n).is_some_and(FileProbe::is_valid))
        };
        let culprit = err.widget().and_then(|w| match split_kind(w) {
            Some((ns, name)) => packs
                .iter()
                .position(|(namespace, _, _)| namespace.as_str() == ns)
                .and_then(|i| valid(packs[i].2, name).map(|n| (i, n))),
            None => packs
                .iter()
                .position(|(_, _, files)| valid(files, w).is_some())
                .and_then(|i| valid(packs[i].2, w).map(|n| (i, n))),
        });
        let Some((i, name)) = culprit else { return Err(SchemaSnafu.into_error(Box::new(err)).into()) };
        if let Some(probe) = packs[i].2.get_mut(&name) {
            probe.schema = None;
            probe.diagnostics.insert(0, Diagnostic::widget_error(&name, err.to_string()));
        }
    }
}
