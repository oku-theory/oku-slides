//! Brouillon du thème en mode modèles : le deck de travail porte ses jetons.
//!
//! À l'entrée du mode modèles, le deck de travail perd son thème (`theme:
//! None`) et reçoit **tous** les jetons du thème du deck mis de côté (sa
//! base) ; il se rend alors sur les seuls jetons par défaut (voir
//! `Session::theme_tokens`), comme tout deck qui utiliserait ce thème. Chaque
//! réglage est un `SetToken` / `UnsetToken`, vérifié par le schéma
//! (`theme_schema`) ; choisir un autre thème remplace les jetons en une étape
//! annulable (`SetTokens`), et la base suit l'annulation (`ThemeSwitch`).
//! Renommer le thème est un simple changement d'état, sans étape.
//!
//! À l'enregistrement des modèles, un brouillon qui diffère de sa base
//! (jetons ou nom) devient une version : nouvelle version courante d'un
//! thème de l'utilisateur, ou copie d'un thème intégré (nouvel id, nommée
//! « (copie) » si le nom n'a pas changé). Sinon, une base autre que le thème
//! du deck est adoptée telle quelle. Le deck mis de côté l'embarque et y
//! passe, dans l'étape d'annulation du pack.

use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{DECK_VERSION, Deck, Op, ThemeRef};

use crate::error::{
    NotEditingTemplatesSnafu, SourcesSnafu, ThemeChoiceAloneSnafu, ThemeJsonSnafu, ThemeVersionMissingSnafu,
    UnknownThemeSnafu,
};
use crate::library_kind::VersionManifest;
use crate::pack::now_created;
use crate::templates_mode::adopt_merge_key;
use crate::theme::{DEFAULT_THEME, ThemeLibrary, ThemeManifest};
use crate::theme_schema::{Control, default_tokens, ensure_unused, lookup, merge, spec, validate, validate_tree};
use crate::views::EditingView;
use crate::{Mode, Result, Session};

/// Le thème édité en mode modèles : sa version de base et son nom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeEdit {
    pub base: ThemeRef,
    pub name: String,
}

/// Le thème édité, vu par l'interface.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeEditView {
    pub base: ThemeRef,
    pub name: String,
    /// Thème intégré : l'enregistrer en crée une copie.
    pub builtin: bool,
    /// Le brouillon diffère de sa base (jetons ou nom).
    pub dirty: bool,
    /// Empreinte des jetons du brouillon : change avec eux, quelle que soit
    /// la voie (réglage, annulation, reprise). Les aperçus des widgets s'y
    /// règlent : deux réglages du même jeton ont le même libellé d'annulation.
    pub revision: String,
}

/// Un changement de base (`set_edited_theme`) et les jetons avant et après :
/// l'annuler ou le rétablir ramène la base qui va avec.
#[derive(Debug, Clone)]
pub(crate) struct ThemeSwitch {
    from: ThemeEdit,
    to: ThemeEdit,
    before: Map<String, Value>,
    after: Map<String, Value>,
}

/// Les jetons d'un deck, en objet (aucun : vide).
fn tokens_of(v: &Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

/// `tokens` en valeur de deck : un objet vide n'en est pas une.
pub(crate) fn tokens_value(tokens: Map<String, Value>) -> Value {
    if tokens.is_empty() { Value::Null } else { Value::Object(tokens) }
}

/// Le jeton `path` existe dans `tokens`.
fn has_path(tokens: &Map<String, Value>, path: &[String]) -> bool {
    lookup(tokens, path).is_some()
}

/// Une op de jetons, vérifiée par le schéma en dehors du mode modèles : la
/// valeur d'une couleur ou d'une palette (règle t-194).
fn checked_outside_drafts(path: &[String]) -> bool {
    spec(path).is_some_and(|s| matches!(s.control, Control::Color | Control::Palette))
}

impl Session {
    /// Les jetons du deck de travail : le brouillon du thème en mode modèles.
    pub(crate) fn edited_tokens(&self) -> Map<String, Value> {
        tokens_of(&self.doc.sources.deck().tokens)
    }

    /// Le deck dont les aperçus (modèles, widgets) prennent le style : en mode
    /// modèles, le deck de travail (le brouillon du thème, sans surcharges du
    /// deck) ; sinon le deck de l'utilisateur.
    pub(crate) fn style_deck(&self) -> &Deck {
        match &self.mode {
            Mode::Templates { .. } => self.doc.sources.deck(),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => self.user_deck(),
        }
    }

    /// Le manifeste de la version `r` : embarquée, sinon dans la bibliothèque.
    fn theme_version(&self, r: &ThemeRef) -> Option<ThemeManifest> {
        self.theme_manifest(r).or_else(|| ThemeManifest::from_files(&self.themes.version(r.id, &r.version)?).ok())
    }

    /// Embarque la version de base du brouillon dans le deck de travail si
    /// aucune source ne la porte : la base se lit ensuite en mémoire, sans
    /// la bibliothèque (la surface `host-theme` et le snapshot la relisent
    /// souvent). Le deck de l'utilisateur n'en reçoit rien : seul un
    /// enregistrement y embarque une version.
    pub(crate) fn embed_theme_base(&mut self) {
        let Mode::Templates { theme, .. } = &self.mode else { return };
        let base = theme.base.clone();
        if self.theme_manifest(&base).is_some() {
            return;
        }
        if let Some(files) = self.themes.version(base.id, &base.version) {
            self.doc.sources.add_theme(files);
        }
    }

    /// Le manifeste de la version de base du brouillon.
    fn base_manifest(&self) -> Option<ThemeManifest> {
        let Mode::Templates { theme, .. } = &self.mode else { return None };
        self.theme_version(&theme.base)
    }

    /// Les jetons de la version de base du brouillon (vides hors mode modèles).
    pub(crate) fn base_tokens(&self) -> Map<String, Value> {
        self.base_manifest().map(|m| m.tokens).unwrap_or_default()
    }

    /// Le thème à éditer en entrant en mode modèles, et ses jetons : celui du
    /// deck de l'utilisateur, sinon la version courante du thème par défaut.
    pub(crate) fn initial_theme_edit(&self) -> Result<(ThemeEdit, Map<String, Value>)> {
        let base = match self.deck_theme() {
            Some(r) => r.clone(),
            None => ThemeRef {
                id: DEFAULT_THEME,
                version: self.themes.current(DEFAULT_THEME).context(UnknownThemeSnafu { id: DEFAULT_THEME })?,
            },
        };
        let manifest = self
            .theme_version(&base)
            .context(ThemeVersionMissingSnafu { id: base.id, version: base.version.clone() })?;
        Ok((ThemeEdit { base, name: manifest.name }, manifest.tokens))
    }

    /// L'empreinte des jetons du deck de travail (stable dans une même version de l'app).
    fn draft_revision(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.doc.sources.deck().tokens.to_string().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Le brouillon diffère-t-il de sa base (jetons ou nom) ?
    fn theme_dirty(&self, edit: &ThemeEdit) -> bool {
        let Some(base) = self.base_manifest() else { return true };
        base.name != edit.name || base.tokens != self.edited_tokens()
    }

    /// Le mode modèles pour le snapshot : le pack et le brouillon du thème.
    pub(crate) fn editing_view(&self) -> Option<EditingView<'_>> {
        let Mode::Templates { edit, theme, .. } = &self.mode else { return None };
        let view = ThemeEditView {
            base: theme.base.clone(),
            name: theme.name.clone(),
            builtin: ThemeLibrary::is_builtin(theme.base.id),
            dirty: self.theme_dirty(theme),
            revision: self.draft_revision(),
        };
        Some(EditingView { mode: edit, theme: view })
    }

    /// Le brouillon part du thème `id`, dans sa version courante : ses jetons
    /// remplacent ceux du deck de travail en une étape annulable.
    pub fn set_edited_theme(&mut self, id: Uuid) -> Result<()> {
        let Mode::Templates { theme, .. } = &self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let from = theme.clone();
        let version = self.themes.current(id).context(UnknownThemeSnafu { id })?;
        let base = ThemeRef { id, version };
        let manifest =
            self.theme_version(&base).context(ThemeVersionMissingSnafu { id, version: base.version.clone() })?;
        let before = self.edited_tokens();
        let label = self.localizer.tr_with("session-undo-edited-theme", &[("name", manifest.name.as_str().into())]);
        let to = ThemeEdit { base, name: manifest.name };
        // La base change avant `apply`, qui écrit l'état de reprise : il la
        // garde avec les jetons. Un échec la remet.
        self.set_theme_edit(to.clone());
        self.embed_theme_base();
        if let Err(e) = self.apply(&label, vec![Op::SetTokens { tokens: tokens_value(manifest.tokens) }], None) {
            self.set_theme_edit(from);
            return Err(e);
        }
        let after = self.edited_tokens();
        if let Mode::Templates { switches, .. } = &mut self.mode {
            switches.push(ThemeSwitch { from, to, before, after });
        }
        Ok(())
    }

    /// Le thème édité devient `edit` (en mode modèles).
    fn set_theme_edit(&mut self, edit: ThemeEdit) {
        if let Mode::Templates { theme, .. } = &mut self.mode {
            *theme = edit;
        }
    }

    /// Renomme le thème édité ; un nom vide reprend celui de sa base.
    ///
    /// Un changement d'état, sans étape d'annulation : le nom n'est pas dans le deck.
    pub fn rename_edited_theme(&mut self, name: String) -> Result<()> {
        if !matches!(self.mode, Mode::Templates { .. }) {
            return NotEditingTemplatesSnafu.fail()?;
        }
        let name = match name.trim() {
            "" => self.base_manifest().map(|m| m.name).unwrap_or_default(),
            trimmed => trimmed.to_owned(),
        };
        if let Mode::Templates { theme, .. } = &mut self.mode {
            theme.name = name;
        }
        self.doc.dirty = true;
        Ok(())
    }

    /// Après annuler ou rétablir : la base suit les jetons ramenés par un
    /// changement de thème (`set_edited_theme`) défait ou refait.
    pub(crate) fn realign_theme_edit(&mut self) {
        let tokens = self.edited_tokens();
        let Mode::Templates { theme, switches, .. } = &mut self.mode else { return };
        let undone = switches.iter().rev().find(|s| s.to.base == theme.base && s.before == tokens).map(|s| &s.from);
        let redone = || switches.iter().rev().find(|s| s.from.base == theme.base && s.after == tokens).map(|s| &s.to);
        if let Some(edit) = undone.or_else(redone) {
            *theme = edit.clone();
        }
        self.embed_theme_base();
    }

    /// Enregistre le brouillon du thème ; le `ThemeRef` que le deck mis de
    /// côté doit prendre, embarqué dans ses sources (`None` : il garde le sien).
    ///
    /// Un brouillon qui diffère de sa base devient une version courante :
    /// du même thème pour un thème de l'utilisateur, d'une copie pour un
    /// intégré. Sinon, une base autre que le thème du deck est retenue.
    pub(crate) fn save_theme_edit(&mut self) -> Result<Option<ThemeRef>> {
        let Mode::Templates { theme, .. } = &self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let edit = theme.clone();
        let base = self
            .theme_version(&edit.base)
            .context(ThemeVersionMissingSnafu { id: edit.base.id, version: edit.base.version.clone() })?;
        let tokens = self.edited_tokens();
        let r = if tokens != base.tokens || edit.name != base.name {
            let (id, name) = match ThemeLibrary::is_builtin(edit.base.id) {
                true if edit.name == base.name => {
                    (Uuid::new_v4(), self.localizer.tr_with("theme-copy-name", &[("name", base.name.as_str().into())]))
                }
                true => (Uuid::new_v4(), edit.name),
                false => (edit.base.id, edit.name),
            };
            let manifest = ThemeManifest {
                id,
                name,
                description: base.description,
                parent: Some(edit.base.version.clone()),
                created: now_created(),
                tokens,
            };
            let files = manifest.to_files().context(ThemeJsonSnafu { id })?;
            self.themes.add_version(id, &files)?;
            self.themes.set_current(id, &files.version())?;
            self.theme_list = self.themes.themes();
            ThemeRef { id, version: files.version() }
        } else if self.deck_theme() != Some(&edit.base) {
            edit.base
        } else {
            return Ok(None);
        };
        let files = self.theme_files(&r).context(ThemeVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
        let name = ThemeManifest::from_files(&files).context(ThemeJsonSnafu { id: r.id })?.name;
        if let Mode::Templates { stash, theme, switches, .. } = &mut self.mode {
            stash.sources.add_theme(files);
            let saved = ThemeEdit { base: r.clone(), name };
            // L'historique du deck de travail reste : les changements de base
            // qui menaient à l'ancienne base mènent à la version enregistrée.
            for s in switches.iter_mut() {
                for edit in [&mut s.from, &mut s.to] {
                    if edit.base == theme.base {
                        *edit = saved.clone();
                    }
                }
            }
            *theme = saved;
        }
        Ok(Some(r))
    }

    /// Le deck mis de côté passe au thème `r` (déjà embarqué), dans l'étape
    /// d'annulation de l'enregistrement du pack qui vient d'avoir lieu.
    pub(crate) fn stash_adopt_theme(&mut self, r: ThemeRef) -> Result<()> {
        let Mode::Templates { stash, edit, .. } = &mut self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", edit.name.as_str().into())]);
        let merge_key = adopt_merge_key(edit.set, edit.parent.as_ref());
        let mut user = stash.sources.deck().clone();
        user.theme = Some(r);
        stash.sources.apply(user, &label, Some(&merge_key)).context(SourcesSnafu)?;
        stash.dirty = true;
        Ok(())
    }

    /// En mode modèles, une transaction faite du seul choix d'un thème : son
    /// id (`null` : le thème par défaut), dont le brouillon part alors
    /// (`set_edited_theme`). `None` ailleurs ou pour toute autre transaction.
    pub(crate) fn drafted_theme_choice(&self, ops: &[Op]) -> Option<Uuid> {
        match (&self.mode, ops) {
            (Mode::Templates { .. }, [Op::SetTheme { theme }]) => Some(theme.as_ref().map_or(DEFAULT_THEME, |r| r.id)),
            _ => None,
        }
    }

    /// Vérifie les ops de jetons de `ops` avant leur application.
    ///
    /// En mode modèles (brouillon du thème) : chaque `SetToken` passe par le
    /// schéma, et un `UnsetToken` qui retire un jeton encore nommé ailleurs
    /// est refusé. Ailleurs : seules les couleurs et palettes des surcharges
    /// du deck sont vérifiées (t-194). Chaque op est vérifiée dans les jetons
    /// effectifs (défauts, thème, surcharges) laissés par les précédentes ; un
    /// `SetTokens` l'est feuille à feuille, selon la même règle. En mode
    /// modèles, un `SetTheme` mêlé à d'autres ops est refusé : le thème s'y
    /// choisit seul (`drafted_theme_choice`), ses jetons par `SetToken`.
    pub(crate) fn check_token_ops(&self, ops: &[Op]) -> Result<()> {
        let drafting = matches!(self.mode, Mode::Templates { .. });
        if drafting && ops.iter().any(|op| matches!(op, Op::SetTheme { .. })) {
            return ThemeChoiceAloneSnafu.fail()?;
        }
        let token_op = |op: &Op| matches!(op, Op::SetToken { .. } | Op::UnsetToken { .. } | Op::SetTokens { .. });
        if !ops.iter().any(token_op) {
            return Ok(());
        }
        let deck = self.doc.sources.deck();
        let mut lower = default_tokens().clone();
        if let Some(theme) = self.theme_tokens(deck) {
            merge(&mut lower, &theme);
        }
        let effective = |own: &Value| {
            let mut tokens = lower.clone();
            merge(&mut tokens, &tokens_of(own));
            tokens
        };
        let mut own = Deck {
            version: DECK_VERSION,
            theme: None,
            params: Default::default(),
            vars: Default::default(),
            tokens: deck.tokens.clone(),
            templates: None,
            slides: Vec::new(),
        };
        for op in ops {
            if !token_op(op) {
                continue;
            }
            match op {
                Op::SetToken { path, value } if drafting || checked_outside_drafts(path) => {
                    validate(path, value, &effective(&own.tokens))?
                }
                Op::SetTokens { tokens } => validate_tree(&tokens_of(tokens), &effective(tokens), drafting)?,
                _ => {}
            }
            // Une op mal formée est refusée par l'application elle-même.
            if oku_slides_core::apply_all(&mut own, vec![op.clone()], &self.registry).is_err() {
                return Ok(());
            }
            if let Op::UnsetToken { path } = op
                && drafting
            {
                let after = effective(&own.tokens);
                if !has_path(&after, path) {
                    ensure_unused(path, &after)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "theme_edit_tests.rs"]
mod theme_edit_tests;
