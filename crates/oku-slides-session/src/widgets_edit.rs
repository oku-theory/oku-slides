//! Widgets du pack en cours : statut, source et modifications.
//!
//! Le pack en cours : en mode pack, la dernière version enregistrée du
//! pack édité (`PackMode::base`) ; en mode deck, le pack épinglé sous
//! l'espace de noms demandé, sinon le pack propre du deck de la catégorie du
//! widget écrit (`deck` pour un élément, `deck-layouts` pour une slide), créé
//! du deck seul à la première écriture (`widgets_deck.rs`).
//! Chaque modification produit une version selon la règle de t-328 (une
//! version par séance) : `adopt_session_version` en mode pack (une étape
//! d'annulation du document du pack), `commit_deck_widgets`
//! (`widgets_deck.rs`) en mode deck.
//!
//! Limites (`[render]`) : `max_widget_kib` par fichier, `max_widgets`
//! fichiers par version, `_commun` compris. Un fichier vide vaut absent :
//! l'écrire le supprime.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{
    Category, Namespace, PackFiles, PackKind, PackVersion, SchemaFault, Slide, WidgetError, WidgetFile, WidgetRegistry,
    WidgetSchema, param_value, qualify, text_fits,
};
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Diagnostic, FileProbe, PackWidgets, RegistryError, Severity};

use crate::error::{
    InHistorySnafu, InvalidWidgetNameSnafu, PackJsonSnafu, PackManifestSnafu, PackVersionMissingSnafu, ReadOnlySnafu,
    RegistrySnafu, TooManyWidgetsSnafu, UnknownWidgetSnafu, WidgetExistsSnafu, WidgetHasNulSnafu, WidgetInUseSnafu,
    WidgetTooLargeSnafu,
};
use crate::pack::{PackManifest, now_created};
use crate::pack_mode::{example_slide, layout_slide_id, realign_layouts};
use crate::pack_mode_preview::realign_widget_previews;
use crate::pack_mode_save::AdoptStep;
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

/// La valeur d'exemple `value` vaut-elle encore pour le paramètre `param` de `schema` ?
///
/// Elle doit se lire dans son type et tenir dans ses bornes de taille
/// (`text_fits`) : une valeur reçue dans un pack importé n'est pas passée
/// par `checked_value` (t-524).
pub(crate) fn trial_fits(schema: &WidgetSchema, param: &str, value: &Value) -> bool {
    schema.param(param).is_some_and(|p| match param_value(&p.ty, param, value.clone()) {
        Ok(Some(v)) => text_fits(&p.ty, &v),
        Ok(None) => true,
        Err(_) => false,
    })
}

/// Le pack dont on édite les widgets : sorte, espace de noms, id, version et fichiers.
pub(crate) struct WidgetTarget {
    pub(crate) kind: PackKind,
    pub(crate) namespace: Namespace,
    pub(crate) id: Uuid,
    pub(crate) version: PackVersion,
    pub(crate) files: PackFiles,
}

/// Lit un nom de fichier de widget (`InvalidWidgetName` sinon).
pub(crate) fn parse_name(name: &str) -> Result<WidgetFile> {
    Ok(WidgetFile::parse(name).context(InvalidWidgetNameSnafu { name })?)
}

/// Le statut d'un fichier sondé du pack `ns`, schéma lu dans le registre assemblé.
fn status_of(ns: &Namespace, name: &WidgetFile, probe: &FileProbe, registry: &WidgetRegistry) -> WidgetStatus {
    let qualified = qualify(ns, name.as_str());
    let schema = (probe.is_valid() && !name.is_common()).then(|| registry.get(&qualified).cloned()).flatten();
    WidgetStatus {
        name: name.clone(),
        schema,
        diagnostics: probe.diagnostics.iter().map(WidgetDiagnostic::of).collect(),
    }
}

/// La source du widget `from` renommé `to` : son nom déclaré suit.
pub(crate) fn renamed_source(source: &str, from: &str, to: &str) -> String {
    source.replacen(&format!("widget(\"{from}\""), &format!("widget(\"{to}\""), 1)
}

impl Session {
    /// Le pack dont on édite les widgets.
    ///
    /// En mode pack, le pack édité (`pack` n'y compte pas) ; sinon le pack
    /// épinglé sous `pack`, ou, sans `pack`, le pack propre du deck (`deck`),
    /// à créer à la première écriture s'il n'existe pas.
    pub(crate) fn widget_target(&self, pack: Option<&Namespace>) -> Result<WidgetTarget> {
        self.widget_target_for(pack, None)
    }

    /// [`Self::widget_target`] pour écrire un widget de catégorie `category`, si elle est connue.
    ///
    /// En mode deck, sans `pack`, une slide va au pack `deck-layouts` (voir `deck_widget_target`).
    fn widget_target_for(&self, pack: Option<&Namespace>, category: Option<Category>) -> Result<WidgetTarget> {
        let Mode::Pack { edit, .. } = &self.mode else { return self.deck_widget_target(pack, category) };
        let files = self
            .doc
            .sources
            .packs
            .get(&edit.base)
            .cloned()
            .context(PackVersionMissingSnafu { id: edit.pack, version: edit.base.clone() })?;
        let namespace = match self.edited_pin() {
            Some((ns, _)) => ns.clone(),
            None => PackManifest::from_files(&files).context(PackManifestSnafu { id: edit.pack })?.namespace,
        };
        Ok(WidgetTarget { kind: files.kind(), namespace, id: edit.pack, version: edit.base.clone(), files })
    }

    /// Statut de chaque fichier de `widgets` (version `version` du pack `ns`, de sorte `kind`), dans l'ordre des noms.
    pub(crate) fn statuses(
        &self,
        (kind, ns): (PackKind, &Namespace),
        version: &PackVersion,
        widgets: &BTreeMap<WidgetFile, String>,
    ) -> Result<Vec<WidgetStatus>> {
        if widgets.is_empty() {
            return Ok(Vec::new());
        }
        let pack = PackWidgets::new(version.hex(), ns.clone(), kind, widgets.clone()).context(RegistrySnafu)?;
        let probe = self.worker.probe_pack(&pack).context(RegistrySnafu)?;
        Ok(probe.files.iter().map(|(name, p)| status_of(ns, name, p, &probe.registry)).collect())
    }

    /// Statut des fichiers du pack en cours (voir `pack` à [`Self::set_widget`]), `_commun` en tête.
    pub fn widget_status(&self, pack: Option<&Namespace>) -> Result<Vec<WidgetStatus>> {
        let t = self.widget_target(pack)?;
        self.statuses((t.kind, &t.namespace), &t.version, t.files.widgets())
    }

    /// Le schéma de `name` si son fichier est valide, lu dans les statuts.
    pub(crate) fn valid_schema(&self, name: &WidgetFile) -> Result<Option<WidgetSchema>> {
        let t = self.widget_target(None)?;
        ensure!(t.files.widget(name).is_some(), UnknownWidgetSnafu { name: name.as_str() });
        let statuses = self.statuses((t.kind, &t.namespace), &t.version, t.files.widgets())?;
        Ok(statuses.into_iter().find(|s| &s.name == name).filter(WidgetStatus::is_valid).and_then(|s| s.schema))
    }

    /// Source du fichier `name` du pack en cours (voir `pack` à [`Self::set_widget`]).
    pub fn widget_source(&self, pack: Option<&Namespace>, name: &str) -> Result<String> {
        let name = parse_name(name)?;
        let t = self.widget_target(pack)?;
        Ok(t.files.widget(&name).context(UnknownWidgetSnafu { name: name.as_str() })?.to_owned())
    }

    /// Crée ou remplace le fichier `name` ; une source vide le supprime.
    ///
    /// Le pack : en mode pack, le pack édité ; en mode deck, celui que le
    /// deck épingle sous `pack` (d'une autre sorte que la catégorie du widget
    /// écrit : refusé), sinon son pack propre de cette catégorie (`deck` ou
    /// `deck-layouts`), créé du deck seul à la première écriture. Rend son statut et celui des widgets que
    /// l'écriture a cassés. Un rendu du deck en échec ne change rien en mode
    /// deck, et laisse la version écrite en mode pack : ses diagnostics
    /// sont dans le statut.
    pub fn set_widget(&mut self, pack: Option<&Namespace>, name: &str, source: String) -> Result<WidgetChange> {
        self.ensure_editable()?;
        let name = parse_name(name)?;
        let removing = source.is_empty();
        // En mode deck, la catégorie du widget écrit choisit le pack propre du deck.
        let category = match &self.mode {
            Mode::Deck if !removing && !name.is_common() => self.written_category(pack, &name, &source)?,
            _ => None,
        };
        let t = self.widget_target_for(pack, category)?;
        // Un pack nommé d'une autre sorte que le widget écrit : refusé, rien n'est écrit.
        if category.is_some_and(|c| c != t.kind.category()) {
            let widget = WidgetError::bad_schema(&qualify(&t.namespace, name.as_str()), SchemaFault::PackCategory);
            return Err(RegistryError::schema(widget)).context(RegistrySnafu)?;
        }
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
        let before = self.statuses((t.kind, &t.namespace), &t.version, t.files.widgets())?;
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        if removing {
            manifest.examples.remove(&name);
        }
        let (after, saved) = match self.commit_manifest(&t, manifest, widgets.clone()) {
            Ok((after, render)) => (with_render(after, render, &name), true),
            // Mode deck : rien n'a changé ; les statuts sont ceux de l'essai.
            Err(e) if e.render_error().is_some() => {
                let render = render_diagnostics(&e);
                (with_render(self.statuses((t.kind, &t.namespace), &t.version, &widgets)?, render, &name), false)
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
        // Un widget modifié garde ses valeurs d'exemple : une valeur que le schéma n'admet plus est
        // ignorée à l'aperçu, et retirée à la prochaine écriture du widget (un widget supprimé perd les siennes).
        if change.saved
            && removing
            && let Some(scratch) = self.scratch_mut()
        {
            scratch.removed(&name);
        }
        Ok(change)
    }

    /// Écriture de l'agent (outil `set_widget`) : écrite, persistée, marquée.
    pub fn set_widget_by_agent(
        &mut self,
        pack: Option<&Namespace>,
        name: &str,
        source: String,
    ) -> Result<WidgetChange> {
        let removing = source.is_empty();
        let change = self.set_widget(pack, name, source)?;
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

    /// Supprime le fichier `name` du pack `pack` (voir [`Self::set_widget`]) ;
    /// refusé si un exemple du pack ou le deck l'utilise.
    pub fn remove_widget(&mut self, pack: Option<&Namespace>, name: &str) -> Result<Vec<WidgetStatus>> {
        let name = parse_name(name)?;
        let t = self.widget_target(pack)?;
        ensure!(t.files.widget(&name).is_some(), UnknownWidgetSnafu { name: name.as_str() });
        self.ensure_unused(&t, &name)?;
        let mut widgets = t.files.widgets().clone();
        widgets.remove(&name);
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        manifest.examples.remove(&name);
        let statuses = self.commit_manifest(&t, manifest, widgets).map(|s| with_render(s.0, s.1, &name))?;
        if let Some(scratch) = self.scratch_mut() {
            scratch.removed(&name);
        }
        Ok(statuses)
    }

    /// Renomme `from` en `to` dans la version éditée ; le nom déclaré suit.
    ///
    /// Refusé pour `_commun`, vers un nom pris, ou si un exemple du pack ou le deck utilise `from`.
    pub fn rename_widget(&mut self, pack: Option<&Namespace>, from: &str, to: &str) -> Result<Vec<WidgetStatus>> {
        let (from, to) = (parse_name(from)?, parse_name(to)?);
        ensure!(!from.is_common(), InvalidWidgetNameSnafu { name: from.as_str() });
        ensure!(!to.is_common(), InvalidWidgetNameSnafu { name: to.as_str() });
        let t = self.widget_target(pack)?;
        let source = t.files.widget(&from).context(UnknownWidgetSnafu { name: from.as_str() })?;
        ensure!(t.files.widget(&to).is_none(), WidgetExistsSnafu { name: to.as_str() });
        self.ensure_unused(&t, &from)?;
        let renamed = renamed_source(source, from.as_str(), to.as_str());
        // Avant le renommage : un widget de ce nom dans un fichier connu du pack (collision).
        let to_existed = self.widget_known(t.id, to.as_str());
        let mut widgets = t.files.widgets().clone();
        widgets.remove(&from);
        widgets.insert(to.clone(), renamed);
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        // L'exemple suit le widget.
        if let Some(examples) = manifest.examples.remove(&from).filter(|e| !e.is_empty()) {
            manifest.examples.insert(to.clone(), examples);
        }
        let statuses = self.commit_manifest(&t, manifest, widgets).map(|s| with_render(s.0, s.1, &to))?;
        if let Some(scratch) = self.scratch_mut() {
            scratch.renamed(&from, &to);
        }
        self.conversation_renamed(t.id, from.as_str(), to.as_str(), to_existed);
        Ok(statuses)
    }

    /// Fichiers connus du pack `pack` : version éditée, pack du deck, version courante.
    pub(crate) fn pack_files_known(&self, pack: Uuid) -> Vec<PackFiles> {
        let mut out = self.pack_files_held(pack);
        out.extend(self.library.current_version(pack).map(|v| v.files));
        out
    }

    /// Fichiers du pack `pack` tenus par la session, sans lire la bibliothèque :
    /// version éditée, pack épinglé par le deck.
    pub(crate) fn pack_files_held(&self, pack: Uuid) -> Vec<PackFiles> {
        let mut out = Vec::new();
        if let Mode::Pack { edit, .. } = &self.mode
            && edit.pack == pack
        {
            out.extend(self.doc.sources.packs.get(&edit.base).cloned());
        }
        if let Some((_, _, pin)) = self.user_deck().all_pins().find(|(_, _, pin)| pin.id == pack) {
            out.extend(self.user_blobs().packs.get(&pin.version).cloned());
        }
        out
    }

    /// Le widget `name` est-il dans un fichier connu du pack `pack` ?
    pub(crate) fn widget_known(&self, pack: Uuid, name: &str) -> bool {
        let Ok(file) = parse_name(name) else { return false };
        self.pack_files_known(pack).iter().any(|f| f.widget(&file).is_some())
    }

    /// Refuse si un exemple du pack ou, en mode deck, une slide du deck utilise `name`, sous son nom qualifié.
    ///
    /// Les exemples sont nommés par leur widget ; en mode pack, la slide d'un
    /// layout est son exemple en cours. Celui de `name` ne compte pas : il part avec lui.
    fn ensure_unused(&self, t: &WidgetTarget, name: &WidgetFile) -> Result<()> {
        let kind = qualify(&t.namespace, name.as_str());
        let uses = |s: &Slide| s.kind == kind || s.walk().any(|l| l.element.kind == kind);
        let manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        let working = &self.doc.sources.deck().slides;
        let pack_mode = self.in_pack_mode();
        // L'exemple de `e` : sa slide en mode pack, sinon celui du manifeste.
        let example = |e: &WidgetFile| -> Option<Slide> {
            let id = layout_slide_id(e);
            let live = working.iter().find(|s| pack_mode && s.id == id).cloned();
            live.or_else(|| manifest.examples.contains_key(e).then(|| example_slide(&manifest, e)))
        };
        let examples: BTreeSet<&WidgetFile> = manifest.examples.keys().chain(t.files.widgets().keys()).collect();
        let mut users: Vec<String> = examples
            .into_iter()
            .filter(|e| *e != name && !e.is_common() && example(e).is_some_and(|s| uses(&s)))
            .map(|e| e.as_str().to_owned())
            .collect();
        // Mode deck : les slides du deck, par leur id.
        if !pack_mode {
            users.extend(working.iter().filter(|s| uses(s)).map(|s| s.id.clone()));
        }
        ensure!(users.is_empty(), WidgetInUseSnafu { name: kind, uses: users });
        Ok(())
    }

    /// Enregistre `manifest` et `widgets` dans la version de la séance ; les statuts d'après.
    ///
    /// Mêmes fichiers et même manifeste (date de création mise à part) :
    /// rien n'est enregistré. Rend aussi les diagnostics du rendu qui a
    /// échoué après l'enregistrement (mode pack). L'avis des widgets
    /// écartés se tait pendant l'écriture : le panneau et l'agent ont les
    /// statuts.
    pub(crate) fn commit_manifest(
        &mut self,
        t: &WidgetTarget,
        mut manifest: PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
    ) -> Result<Committed> {
        self.ensure_editable()?;
        let before = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        if widgets == *t.files.widgets() && manifest == before {
            return Ok((self.statuses((t.kind, &t.namespace), &t.version, &widgets)?, Vec::new()));
        }
        manifest.created = now_created();
        self.widget_edit = true;
        let committed = self.commit_files(t, manifest, widgets);
        self.widget_edit = false;
        let render = committed?;
        Ok((self.widget_status(Some(&t.namespace))?, render))
    }

    /// La version produite, selon le mode ; les diagnostics d'un rendu en échec.
    ///
    /// Mode deck : tout échec, rendu compris, ne change rien (`Err`). Mode
    /// pack : la version est enregistrée avant le rendu du deck de travail ; s'il
    /// échoue, elle reste (comme un layout fautif) et ses diagnostics sont rendus.
    fn commit_files(
        &mut self,
        t: &WidgetTarget,
        mut manifest: PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
    ) -> Result<Vec<WidgetDiagnostic>> {
        match &self.mode {
            Mode::Pack { edit, .. } => {
                let edit = edit.clone();
                manifest.parent = edit.parent.clone();
                let label = self.localizer.tr_with("widgets-edit-label", &[("name", manifest.name.as_str().into())]);
                let files = manifest.to_files(t.kind, widgets).context(PackJsonSnafu { id: t.id })?;
                // Un layout écrit ajoute sa slide au deck de travail, un layout supprimé retire la sienne ;
                // les slides d'aperçu d'un pack de widgets suivent ses widgets.
                let mut deck = self.doc.sources.deck().clone();
                realign_layouts(&mut deck, &manifest, &files);
                realign_widget_previews(&mut deck, &manifest, &files);
                self.adopt_session_version(deck, files, &edit, false, AdoptStep::Session(&label))?;
                match self.rerender() {
                    Ok(()) => Ok(Vec::new()),
                    Err(e) if e.render_error().is_some() => Ok(render_diagnostics(&e)),
                    Err(e) => Err(e),
                }
            }
            Mode::Deck => self.commit_deck_widgets(t, manifest, widgets).map(|()| Vec::new()),
            Mode::Help { .. } => ReadOnlySnafu.fail()?,
            Mode::History { .. } => InHistorySnafu.fail()?,
        }
    }
}

#[cfg(test)]
#[path = "widgets_edit_tests.rs"]
mod widgets_edit_tests;
