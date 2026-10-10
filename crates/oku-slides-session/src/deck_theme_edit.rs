//! Éditer un thème depuis le mode deck, sans passer par l'édition des modèles.
//!
//! `edit_theme` part de la version courante d'un thème, y applique des ops de
//! jetons (vérifiées par le schéma, comme le brouillon du mode modèles) et un
//! nouveau nom, puis enregistre le résultat : nouvelle version courante d'un
//! thème de l'utilisateur, ou copie d'un thème intégré (nouvel id, nommée
//! « (copie) » sans autre nom). Un deck qui utilise ce thème passe à la
//! nouvelle version en une étape annulable ; la bibliothèque, elle, garde la
//! version (comme tout enregistrement de thème). L'agent s'en sert : l'écran
//! reste sur les slides.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{DECK_VERSION, Deck, Op, PackVersion, ThemeRef};

use crate::error::{EditingTemplatesSnafu, OpSnafu, ThemeJsonSnafu, UnknownThemeSnafu};
use crate::pack::now_created;
use crate::theme::{ThemeInfo, ThemeLibrary, ThemeManifest};
use crate::theme_schema::default_tokens;
use crate::{Mode, Result, Session};

/// Une op sur les jetons d'un thème (même forme que l'op du deck).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TokenOp {
    /// Jeton au chemin donné (`["color", "accent"]`) ; `null` est posé tel quel.
    SetToken { path: Vec<String>, value: Value },
    /// Retire le jeton au chemin donné (retour aux valeurs par défaut).
    UnsetToken { path: Vec<String> },
}

impl From<TokenOp> for Op {
    fn from(op: TokenOp) -> Self {
        match op {
            TokenOp::SetToken { path, value } => Op::SetToken { path, value },
            TokenOp::UnsetToken { path } => Op::UnsetToken { path },
        }
    }
}

/// Le thème après `edit_theme`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EditedTheme {
    pub id: Uuid,
    pub version: PackVersion,
    pub name: String,
    /// Une version a été enregistrée (sinon, rien ne changeait).
    pub changed: bool,
    /// Le deck est passé à cette version.
    pub adopted: bool,
}

/// Un deck vide qui ne porte que `tokens` : on y applique des ops de jetons.
pub(crate) fn token_deck(tokens: Value) -> Deck {
    Deck {
        version: DECK_VERSION,
        theme: None,
        params: Default::default(),
        vars: Default::default(),
        tokens,
        templates: None,
        slides: Vec::new(),
    }
}

impl Session {
    /// Le thème `id` dans sa version courante (relu dans la bibliothèque).
    pub fn theme_info(&self, id: Uuid) -> Result<ThemeInfo> {
        let current = self.themes.current_version(id).context(UnknownThemeSnafu { id })?;
        Ok(ThemeInfo {
            id,
            version: current.version,
            name: current.manifest.name,
            builtin: ThemeLibrary::is_builtin(id),
            default: id == crate::theme::DEFAULT_THEME,
        })
    }

    /// Édite le thème `id` (version courante) : ops de jetons et nom.
    ///
    /// Refusé hors du mode deck. Un nom vide garde le nom actuel. Sans
    /// changement, rien n'est enregistré.
    pub fn edit_theme(&mut self, id: Uuid, name: Option<String>, ops: Vec<TokenOp>) -> Result<EditedTheme> {
        ensure!(matches!(self.mode, Mode::Deck), EditingTemplatesSnafu);
        let current = self.themes.current_version(id).context(UnknownThemeSnafu { id })?;
        let base = current.manifest;
        let ops: Vec<Op> = ops.into_iter().map(Op::from).collect();
        let start = Value::Object(base.tokens.clone());
        self.check_token_ops_on(default_tokens(), &start, &ops, true)?;
        let mut scratch = token_deck(start);
        oku_slides_core::apply_all(&mut scratch, ops, &self.registry).context(OpSnafu)?;
        let tokens = match scratch.tokens {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        let renamed = name.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty() && *n != base.name);
        if tokens == base.tokens && renamed.is_none() {
            return Ok(EditedTheme { id, version: current.version, name: base.name, changed: false, adopted: false });
        }
        let builtin = ThemeLibrary::is_builtin(id);
        let name = match renamed {
            Some(n) => n,
            None if builtin => self.localizer.tr_with("theme-copy-name", &[("name", base.name.as_str().into())]),
            None => base.name.clone(),
        };
        let new_id = if builtin { Uuid::new_v4() } else { id };
        let manifest = ThemeManifest {
            id: new_id,
            name: name.clone(),
            description: base.description,
            parent: Some(current.version),
            created: now_created(),
            tokens,
        };
        let files = manifest.to_files().context(ThemeJsonSnafu { id: new_id })?;
        let version = files.version();
        self.themes.add_version(new_id, &files)?;
        self.themes.set_current(new_id, &version)?;
        self.theme_list = self.themes.themes();
        let r = ThemeRef { id: new_id, version: version.clone() };
        let adopted = self.deck_theme().is_some_and(|t| t.id == id);
        if adopted {
            // `apply` embarque la version, prise dans la bibliothèque.
            let label = self.localizer.tr("session-undo-theme");
            self.apply(&label, vec![Op::SetTheme { theme: Some(r) }], None)?;
        }
        Ok(EditedTheme { id: new_id, version, name, changed: true, adopted })
    }
}

#[cfg(test)]
#[path = "deck_theme_edit_tests.rs"]
mod deck_theme_edit_tests;
