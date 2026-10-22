//! Éditer un thème depuis le mode deck, sans passer par l'édition d'un pack.
//!
//! `edit_theme` part de la version courante d'un thème, y applique des ops de
//! jetons (vérifiées par le schéma, comme le brouillon du mode pack) et un
//! nouveau nom, puis enregistre le résultat : nouvelle version courante d'un
//! thème de l'utilisateur, ou copie d'un thème intégré (nouvel id, nommée
//! « (copie) » sans autre nom). Un deck qui utilise ce thème passe à la
//! nouvelle version en une étape annulable ; la bibliothèque, elle, garde la
//! version (comme tout enregistrement de thème). L'agent s'en sert : l'écran
//! reste sur les slides.
//!
//! Un thème du deck seul (`PackScope::Deck` : le thème généré par l'import)
//! part de la version du deck ; la nouvelle version, du deck seul aussi, est
//! embarquée et adoptée sans jamais entrer dans la bibliothèque.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{DECK_VERSION, Deck, Op, PackVersion, ThemeRef};

use crate::error::{EditingPackSnafu, OpSnafu, ThemeJsonSnafu, UnknownThemeSnafu};
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
    /// Les jetons propres de cette version, ceux que lit [`Session::theme_tokens_of`].
    pub tokens: Map<String, Value>,
}

/// Un deck vide qui ne porte que `tokens` : on y applique des ops de jetons.
pub(crate) fn token_deck(tokens: Value) -> Deck {
    Deck {
        version: DECK_VERSION,
        theme: None,
        params: Default::default(),
        vars: Default::default(),
        tokens,
        widgets: Default::default(),
        layouts: Default::default(),
        slides: Vec::new(),
    }
}

impl Session {
    /// La version d'où part l'édition du thème `id`, et son manifeste.
    ///
    /// Celle du deck pour le thème du deck seul qu'il porte (sans lire la
    /// bibliothèque), sinon la version courante de la bibliothèque, sinon pour
    /// un thème du deck seul embarqué celle qu'on reprendrait (`deck_only_theme_ref`).
    fn theme_base(&self, id: Uuid) -> Result<(PackVersion, ThemeManifest)> {
        if let Some(r) = self.deck_theme().filter(|r| r.id == id)
            && let Some(m) = self.theme_manifest(r).filter(|m| !m.scope.is_library())
        {
            return Ok((r.version.clone(), m));
        }
        if let Some(current) = self.themes.current_version(id) {
            return Ok((current.version, current.manifest));
        }
        let r = self.deck_only_theme_ref(id).context(UnknownThemeSnafu { id })?;
        let manifest = self.theme_manifest(&r).context(UnknownThemeSnafu { id })?;
        Ok((r.version, manifest))
    }

    /// Le thème `id` dans la version d'où partirait son édition.
    ///
    /// Relu dans la bibliothèque, ou dans le deck pour un thème du deck seul (`deck_only`).
    pub fn theme_info(&self, id: Uuid) -> Result<ThemeInfo> {
        let (version, manifest) = self.theme_base(id)?;
        Ok(ThemeInfo {
            id,
            version,
            name: manifest.name,
            builtin: ThemeLibrary::is_builtin(id),
            default: id == crate::theme::DEFAULT_THEME,
            deck_only: !manifest.scope.is_library(),
        })
    }

    /// Les jetons propres du thème `id`, dans la version d'où partirait son édition.
    ///
    /// Ceux que changent les ops de [`Self::edit_theme`] ; une clé absente
    /// vient des valeurs par défaut.
    pub fn theme_tokens_of(&self, id: Uuid) -> Result<Map<String, Value>> {
        Ok(self.theme_base(id)?.1.tokens)
    }

    /// Édite le thème `id` : ops de jetons et nom.
    ///
    /// Part de sa version courante, ou de celle du deck pour un thème du deck seul.
    ///
    /// Refusé hors du mode deck. Un nom vide garde le nom actuel. Sans
    /// changement, rien n'est enregistré.
    pub fn edit_theme(&mut self, id: Uuid, name: Option<String>, ops: Vec<TokenOp>) -> Result<EditedTheme> {
        ensure!(matches!(self.mode, Mode::Deck), EditingPackSnafu);
        let (current, base) = self.theme_base(id)?;
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
            let (name, changed, adopted) = (base.name, false, false);
            return Ok(EditedTheme { id, version: current, name, changed, adopted, tokens });
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
            parent: Some(current),
            created: now_created(),
            scope: base.scope,
            tokens: tokens.clone(),
        };
        let files = manifest.to_files().context(ThemeJsonSnafu { id: new_id })?;
        let version = files.version();
        let r = ThemeRef { id: new_id, version: version.clone() };
        let adopted = self.deck_theme().is_some_and(|t| t.id == id);
        if !manifest.scope.is_library() {
            // Du deck seul : embarquée puis adoptée, jamais dans la bibliothèque. Un deck
            // passé à un autre thème le reprend, comme en le choisissant (t-903).
            let known = self.doc.sources.themes.contains_key(&version);
            self.doc.sources.add_theme(files);
            if let Err(e) = self.adopt_edited_theme(r) {
                if !known {
                    self.doc.sources.themes.remove(&version);
                }
                return Err(e);
            }
            return Ok(EditedTheme { id: new_id, version, name, changed: true, adopted: true, tokens });
        }
        self.themes.add_version(new_id, &files)?;
        self.themes.set_current(new_id, &version)?;
        self.theme_list = self.themes.themes();
        if adopted {
            self.adopt_edited_theme(r)?;
        }
        Ok(EditedTheme { id: new_id, version, name, changed: true, adopted, tokens })
    }

    /// Le deck passe à la version éditée `r`, en une étape annulable.
    fn adopt_edited_theme(&mut self, r: ThemeRef) -> Result<()> {
        // `apply` embarque la version, prise dans la bibliothèque si le deck ne l'a pas.
        let label = self.localizer.tr("session-undo-theme");
        self.apply(&label, vec![Op::SetTheme { theme: Some(r) }], None)
    }
}

#[cfg(test)]
#[path = "deck_theme_edit_tests.rs"]
mod deck_theme_edit_tests;
