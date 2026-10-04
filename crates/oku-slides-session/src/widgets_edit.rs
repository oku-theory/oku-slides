//! Widgets du pack en cours : statut, source et modifications.
//!
//! Le pack en cours : en mode modèles, la dernière version enregistrée du
//! pack édité (`TemplateMode::base`) ; sinon celle qu'embarque le deck.
//! Chaque modification produit une version selon la règle de t-328 (une
//! version par séance) : `adopt_session_version` en mode modèles,
//! `commit_deck_widgets` (`widgets_deck.rs`) en mode deck.
//!
//! Limites (`[render]`) : `max_widget_kib` par fichier, `max_widgets`
//! fichiers par version, `_commun` compris. Un fichier vide vaut absent :
//! l'écrire le supprime.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackRef, PackVersion, Slide, WidgetFile, WidgetRegistry, WidgetSchema, param_value};
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Diagnostic, FileProbe, PackWidgets, Severity};

use crate::error::{
    InHistorySnafu, InvalidWidgetNameSnafu, PackJsonSnafu, PackVersionMissingSnafu, RegistrySnafu, TooManyWidgetsSnafu,
    UnknownWidgetSnafu, WidgetExistsSnafu, WidgetHasNulSnafu, WidgetInUseSnafu, WidgetTooLargeSnafu,
};
use crate::pack::{PackManifest, now_created};
use crate::{Mode, Result, Session};

/// Bornes des fichiers de widgets (`[render]`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct WidgetLimits {
    max_bytes: usize,
    max_kib: usize,
    max_files: usize,
}

impl WidgetLimits {
    pub(crate) fn of(r: &RenderSettings) -> Self {
        Self { max_bytes: r.max_widget_bytes(), max_kib: r.max_widget_kib, max_files: r.max_widgets }
    }
}

/// Un message de Typst, ou du contrôle, sur un fichier de widget.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetDiagnostic {
    /// Fichier du pack où il se trouve ; `None` hors du pack (bibliothèque).
    pub file: Option<WidgetFile>,
    /// Ligne et colonne, à partir de 1, si elles sont connues.
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// Une erreur (sinon un avertissement).
    pub error: bool,
    /// Le message (en anglais, comme ceux de Typst) et ses indices.
    pub message: String,
}

impl WidgetDiagnostic {
    /// Depuis un diagnostic du rendu.
    pub(crate) fn of(d: &Diagnostic) -> Self {
        let mut message = d.message.clone();
        for hint in &d.hints {
            message.push('\n');
            message.push_str(hint);
        }
        Self {
            file: d.widget_file(),
            line: d.at.as_ref().map(|a| a.line),
            column: d.at.as_ref().map(|a| a.column),
            error: d.severity == Severity::Error,
            message,
        }
    }
}

/// Un fichier du pack et son statut.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetStatus {
    pub name: WidgetFile,
    /// Le schéma du widget, s'il est valide ; `None` pour `_commun`.
    pub schema: Option<WidgetSchema>,
    pub diagnostics: Vec<WidgetDiagnostic>,
}

impl WidgetStatus {
    /// Valide : aucune erreur.
    pub fn is_valid(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.error)
    }
}

/// Ce que rend une écriture : le fichier, et les widgets qu'elle a cassés.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetChange {
    pub status: WidgetStatus,
    /// Widgets valides avant, cassés après (un `_commun` modifié).
    pub broken: Vec<WidgetStatus>,
    /// L'écriture est enregistrée.
    ///
    /// Faux en mode deck quand le rendu échoue : rien n'est enregistré.
    pub saved: bool,
}

/// Statuts après une écriture, et diagnostics d'un rendu en échec.
type Committed = (Vec<WidgetStatus>, Vec<WidgetDiagnostic>);

/// Diagnostics d'un rendu en échec ; sans diagnostic Typst, l'erreur elle-même.
fn render_diagnostics(e: &crate::SessionError) -> Vec<WidgetDiagnostic> {
    let diagnostics = e.render_error().map(|re| re.diagnostics()).unwrap_or_default();
    if diagnostics.is_empty() {
        let message = e.one_line();
        return vec![WidgetDiagnostic { file: None, line: None, column: None, error: true, message }];
    }
    diagnostics.iter().map(WidgetDiagnostic::of).collect()
}

/// Ajoute à `statuses` les diagnostics `render`, chacun à son fichier.
///
/// Un diagnostic hors des fichiers listés va au fichier `name` (écrit ou renommé).
fn with_render(mut statuses: Vec<WidgetStatus>, render: Vec<WidgetDiagnostic>, name: &WidgetFile) -> Vec<WidgetStatus> {
    for d in render {
        let owner = d.file.clone().filter(|f| statuses.iter().any(|s| s.name == *f)).unwrap_or_else(|| name.clone());
        match statuses.iter_mut().find(|s| s.name == owner) {
            Some(s) => s.diagnostics.push(d),
            None => statuses.push(WidgetStatus { name: owner, schema: None, diagnostics: vec![d] }),
        }
    }
    statuses
}

/// L'essai `value` vaut-il encore pour le paramètre `param` de `schema` ?
pub(crate) fn trial_fits(schema: &WidgetSchema, param: &str, value: &Value) -> bool {
    schema.param(param).is_some_and(|p| param_value(&p.ty, param, value.clone()).is_ok())
}

/// Le pack dont on édite les widgets : id, version et fichiers.
pub(crate) struct WidgetTarget {
    pub(crate) id: Uuid,
    pub(crate) version: PackVersion,
    pub(crate) files: PackFiles,
}

/// Lit un nom de fichier de widget (`InvalidWidgetName` sinon).
pub(crate) fn parse_name(name: &str) -> Result<WidgetFile> {
    Ok(WidgetFile::parse(name).context(InvalidWidgetNameSnafu { name })?)
}

/// Le statut d'un fichier sondé, schéma lu dans le registre assemblé.
fn status_of(name: &WidgetFile, probe: &FileProbe, registry: &WidgetRegistry) -> WidgetStatus {
    let schema = (probe.is_valid() && !name.is_common()).then(|| registry.get(name.as_str()).cloned()).flatten();
    WidgetStatus {
        name: name.clone(),
        schema,
        diagnostics: probe.diagnostics.iter().map(WidgetDiagnostic::of).collect(),
    }
}

impl Session {
    /// Le pack dont on édite les widgets.
    pub(crate) fn widget_target(&self) -> Result<WidgetTarget> {
        let (r, files) = match &self.mode {
            Mode::Templates { edit, .. } => {
                let files = self
                    .doc
                    .sources
                    .packs
                    .get(&edit.base)
                    .cloned()
                    .context(PackVersionMissingSnafu { id: edit.set, version: edit.base.clone() })?;
                (PackRef { id: edit.set, version: edit.base.clone() }, files)
            }
            Mode::Deck | Mode::History { .. } => self.deck_pack_files()?,
        };
        Ok(WidgetTarget { id: r.id, version: r.version, files })
    }

    /// Statut de chaque fichier de `files` (version `version`), dans l'ordre des noms.
    pub(crate) fn statuses(
        &self,
        version: &PackVersion,
        widgets: &BTreeMap<WidgetFile, String>,
    ) -> Result<Vec<WidgetStatus>> {
        if widgets.is_empty() {
            return Ok(Vec::new());
        }
        let pack = PackWidgets::new(version.hex(), widgets.clone()).context(RegistrySnafu)?;
        let probe = self.worker.probe_pack(&pack).context(RegistrySnafu)?;
        Ok(probe.files.iter().map(|(name, p)| status_of(name, p, &probe.registry)).collect())
    }

    /// Statut des fichiers du pack en cours, `_commun` en tête.
    pub fn widget_status(&self) -> Result<Vec<WidgetStatus>> {
        let t = self.widget_target()?;
        self.statuses(&t.version, t.files.widgets())
    }

    /// Le schéma de `name` si son fichier est valide, lu dans les statuts.
    pub(crate) fn valid_schema(&self, name: &WidgetFile) -> Result<Option<WidgetSchema>> {
        let t = self.widget_target()?;
        ensure!(t.files.widget(name).is_some(), UnknownWidgetSnafu { name: name.as_str() });
        let statuses = self.statuses(&t.version, t.files.widgets())?;
        Ok(statuses.into_iter().find(|s| &s.name == name).filter(WidgetStatus::is_valid).and_then(|s| s.schema))
    }

    /// Source du fichier `name` du pack en cours.
    pub fn widget_source(&self, name: &str) -> Result<String> {
        let name = parse_name(name)?;
        let t = self.widget_target()?;
        Ok(t.files.widget(&name).context(UnknownWidgetSnafu { name: name.as_str() })?.to_owned())
    }

    /// Crée ou remplace le fichier `name` ; une source vide le supprime.
    ///
    /// Rend son statut et celui des widgets que l'écriture a cassés. Un
    /// rendu du deck en échec ne change rien en mode deck, et laisse la
    /// version écrite en mode modèles : ses diagnostics sont dans le statut.
    pub fn set_widget(&mut self, name: &str, source: String) -> Result<WidgetChange> {
        self.ensure_editable()?;
        let name = parse_name(name)?;
        let removing = source.is_empty();
        let t = self.widget_target()?;
        let mut widgets = t.files.widgets().clone();
        if removing {
            ensure!(widgets.contains_key(&name), UnknownWidgetSnafu { name: name.as_str() });
            self.ensure_unused(&t, &name)?;
            widgets.remove(&name);
        } else {
            let limits = self.widget_limits;
            ensure!(source.len() <= limits.max_bytes, WidgetTooLargeSnafu { name: name.as_str(), max: limits.max_kib });
            // L'octet `0` sépare les fichiers du contenu canonique d'une version.
            ensure!(!source.contains('\0'), WidgetHasNulSnafu { name: name.as_str() });
            ensure!(
                widgets.contains_key(&name) || widgets.len() < limits.max_files,
                TooManyWidgetsSnafu { max: limits.max_files }
            );
            widgets.insert(name.clone(), source);
        }
        let before = self.statuses(&t.version, t.files.widgets())?;
        let (after, saved) = match self.commit_widgets(&t, widgets.clone()) {
            Ok((after, render)) => (with_render(after, render, &name), true),
            // Mode deck : rien n'a changé ; les statuts sont ceux de l'essai.
            Err(e) if e.render_error().is_some() => {
                let render = render_diagnostics(&e);
                (with_render(self.statuses(&t.version, &widgets)?, render, &name), false)
            }
            Err(e) => return Err(e),
        };
        let status = after.iter().find(|s| s.name == name).cloned().unwrap_or_else(|| WidgetStatus {
            name: name.clone(),
            schema: None,
            diagnostics: Vec::new(),
        });
        let was_valid = |n: &WidgetFile| before.iter().any(|b| b.name == *n && b.is_valid());
        let broken = after.into_iter().filter(|s| s.name != name && !s.is_valid() && was_valid(&s.name)).collect();
        let change = WidgetChange { status, broken, saved };
        if change.saved
            && let Some(scratch) = self.scratch_mut()
        {
            if removing {
                scratch.removed(&name);
            } else if let Some(schema) = &change.status.schema {
                // Un essai d'un paramètre retiré, ou dont le type a changé, ne vaut plus.
                scratch.pruned(&name, |param, value| trial_fits(schema, param, value));
            }
            // Fichier invalide (pas de schéma) : les essais restent, `preview_inputs` les filtre par schéma.
        }
        Ok(change)
    }

    /// Écriture de l'agent (outil `set_widget`) : écrite, persistée, marquée.
    pub fn set_widget_by_agent(&mut self, name: &str, source: String) -> Result<WidgetChange> {
        let removing = source.is_empty();
        let change = self.set_widget(name, source)?;
        if change.saved
            && !removing
            && let Some(scratch) = self.scratch_mut()
        {
            scratch.mark_agent(&change.status.name);
        }
        self.persist(true);
        Ok(change)
    }

    /// Écriture enregistrée de l'utilisateur sur un widget de l'agent : l'agent l'apprendra.
    pub(crate) fn noted_user_write(&mut self, name: &str, saved: bool) {
        let Ok(file) = parse_name(name) else { return };
        if saved && self.scratch_mut().is_some_and(|s| s.take_agent_mark(&file)) {
            self.note(format!("user edited widget {} after you", file.as_str()));
        }
    }

    /// Supprime le fichier `name` ; refusé si un modèle du pack l'utilise.
    pub fn remove_widget(&mut self, name: &str) -> Result<Vec<WidgetStatus>> {
        let name = parse_name(name)?;
        let t = self.widget_target()?;
        ensure!(t.files.widget(&name).is_some(), UnknownWidgetSnafu { name: name.as_str() });
        self.ensure_unused(&t, &name)?;
        let mut widgets = t.files.widgets().clone();
        widgets.remove(&name);
        let statuses = self.commit_widgets(&t, widgets).map(|s| with_render(s.0, s.1, &name))?;
        if let Some(scratch) = self.scratch_mut() {
            scratch.removed(&name);
        }
        Ok(statuses)
    }

    /// Renomme `from` en `to` dans la version éditée ; le nom déclaré suit.
    ///
    /// Refusé pour `_commun`, vers un nom pris, ou si un modèle utilise `from`.
    pub fn rename_widget(&mut self, from: &str, to: &str) -> Result<Vec<WidgetStatus>> {
        let (from, to) = (parse_name(from)?, parse_name(to)?);
        ensure!(!from.is_common(), InvalidWidgetNameSnafu { name: from.as_str() });
        ensure!(!to.is_common(), InvalidWidgetNameSnafu { name: to.as_str() });
        let t = self.widget_target()?;
        let source = t.files.widget(&from).context(UnknownWidgetSnafu { name: from.as_str() })?;
        ensure!(t.files.widget(&to).is_none(), WidgetExistsSnafu { name: to.as_str() });
        self.ensure_unused(&t, &from)?;
        let renamed = source.replacen(&format!("widget(\"{from}\""), &format!("widget(\"{to}\""), 1);
        let mut widgets = t.files.widgets().clone();
        widgets.remove(&from);
        widgets.insert(to.clone(), renamed);
        let statuses = self.commit_widgets(&t, widgets).map(|s| with_render(s.0, s.1, &to))?;
        if let Some(scratch) = self.scratch_mut() {
            scratch.renamed(&from, &to);
        }
        Ok(statuses)
    }

    /// Refuse si un modèle du pack (enregistré, ou en cours d'édition) utilise `name`.
    fn ensure_unused(&self, t: &WidgetTarget, name: &WidgetFile) -> Result<()> {
        let uses = |s: &Slide| s.kind == name.as_str() || s.walk().any(|l| l.element.kind == name.as_str());
        let manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        let mut templates: Vec<String> =
            manifest.templates.iter().filter(|tpl| uses(&tpl.slide)).map(|tpl| tpl.name.clone()).collect();
        if matches!(self.mode, Mode::Templates { .. }) {
            for s in self.doc.sources.deck().slides.iter().filter(|s| uses(s)) {
                let shown = s.name.clone().unwrap_or_else(|| s.id.clone());
                if !templates.contains(&shown) {
                    templates.push(shown);
                }
            }
        }
        ensure!(templates.is_empty(), WidgetInUseSnafu { name: name.as_str(), templates });
        Ok(())
    }

    /// Enregistre `widgets` dans la version de la séance ; les statuts d'après.
    ///
    /// Mêmes fichiers : rien n'est enregistré. Rend aussi les diagnostics du
    /// rendu qui a échoué après l'enregistrement (mode modèles). L'avis des
    /// widgets écartés se tait pendant l'écriture : le panneau et l'agent
    /// ont les statuts.
    fn commit_widgets(&mut self, t: &WidgetTarget, widgets: BTreeMap<WidgetFile, String>) -> Result<Committed> {
        self.ensure_editable()?;
        if widgets == *t.files.widgets() {
            return Ok((self.statuses(&t.version, &widgets)?, Vec::new()));
        }
        let mut manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        manifest.created = now_created();
        self.widget_edit = true;
        let committed = self.commit_files(t, manifest, widgets);
        self.widget_edit = false;
        let render = committed?;
        Ok((self.widget_status()?, render))
    }

    /// La version produite, selon le mode ; les diagnostics d'un rendu en échec.
    ///
    /// Mode deck : tout échec, rendu compris, ne change rien (`Err`). Mode
    /// modèles : la version est enregistrée avant le rendu des modèles ; s'il
    /// échoue, elle reste (comme un modèle fautif) et ses diagnostics sont rendus.
    fn commit_files(
        &mut self,
        t: &WidgetTarget,
        mut manifest: PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
    ) -> Result<Vec<WidgetDiagnostic>> {
        match &self.mode {
            Mode::Templates { edit, .. } => {
                let edit = edit.clone();
                manifest.parent = edit.parent.clone();
                let files = manifest.to_files(widgets).context(PackJsonSnafu { id: t.id })?;
                let deck = self.doc.sources.deck().clone();
                self.adopt_session_version(deck, files, &edit, false)?;
                match self.rerender() {
                    Ok(()) => Ok(Vec::new()),
                    Err(e) if e.render_error().is_some() => Ok(render_diagnostics(&e)),
                    Err(e) => Err(e),
                }
            }
            Mode::Deck => self.commit_deck_widgets(t, manifest, widgets).map(|()| Vec::new()),
            Mode::History { .. } => InHistorySnafu.fail()?,
        }
    }
}

#[cfg(test)]
#[path = "widgets_edit_tests.rs"]
mod widgets_edit_tests;
