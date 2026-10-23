//! Éditer un thème depuis le mode deck, sans passer par l'édition d'un pack.
//!
//! `edit_theme` part de la version courante d'un thème, y applique des ops de
//! jetons (vérifiées par le schéma composé du deck, au-dessus des défauts de
//! ses packs et des parents du thème, comme le brouillon du mode pack) et un
//! nouveau nom, puis enregistre le résultat : nouvelle version courante d'un
//! thème de l'utilisateur (même espace de noms, mêmes parents), ou enfant d'un
//! thème intégré (nouvel id, espace de noms tiré de son nom, `extends:
//! [<intégré>]`, les seuls jetons édités, nommé « (variante) » sans autre
//! nom). Un deck qui utilise ce thème passe à la nouvelle version en une
//! étape annulable ; la bibliothèque, elle, garde la version (comme tout
//! enregistrement de thème). L'agent s'en sert : l'écran reste sur les slides.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{DECK_VERSION, Deck, Namespace, Op, PackVersion};

use crate::deck_theme::{LayerReading, pack_defaults};
use crate::error::{EditingPackSnafu, OpSnafu, ThemeJsonSnafu, UnknownThemeSnafu};
use crate::library_lookup::find_theme;
use crate::namespace_slug::library_namespace;
use crate::pack::now_created;
use crate::theme::{ThemeInfo, ThemeLibrary, ThemeManifest};
use crate::theme_closure::{chain_tokens, theme_closure};
use crate::theme_schema::{default_tokens, merge};
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
    /// Son espace de noms : celui du thème édité, ou celui de son enfant.
    pub namespace: Namespace,
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
        themes: Default::default(),
        params: Default::default(),
        vars: Default::default(),
        tokens,
        widgets: Default::default(),
        layouts: Default::default(),
        slides: Vec::new(),
    }
}

impl Session {
    /// La version d'où part l'édition du thème `id`, et son manifeste : sa version courante de la bibliothèque.
    fn theme_base(&self, id: Uuid) -> Result<(PackVersion, ThemeManifest)> {
        let current = self.themes.current_version(id).context(UnknownThemeSnafu { theme: id.to_string() })?;
        Ok((current.version, current.manifest))
    }

    /// Le thème `id` dans la version d'où partirait son édition, relu dans la bibliothèque.
    pub fn theme_info(&self, id: Uuid) -> Result<ThemeInfo> {
        let current = self.themes.current_version(id).context(UnknownThemeSnafu { theme: id.to_string() })?;
        Ok(ThemeInfo::of(current))
    }

    /// Le thème désigné par `wanted` (espace de noms, id ou nom, sans casse), dans sa version courante.
    pub fn find_theme_info(&self, wanted: &str) -> Result<ThemeInfo> {
        let ns = find_theme(&self.themes, wanted).context(UnknownThemeSnafu { theme: wanted })?;
        let current = self.themes.by_namespace(&ns).context(UnknownThemeSnafu { theme: wanted })?;
        Ok(ThemeInfo::of(current))
    }

    /// Les jetons propres du thème `id`, dans la version d'où partirait son édition.
    ///
    /// Ceux que changent les ops de [`Self::edit_theme`] ; une clé absente
    /// vient des valeurs par défaut.
    pub fn theme_tokens_of(&self, id: Uuid) -> Result<Map<String, Value>> {
        Ok(self.theme_base(id)?.1.tokens)
    }

    /// Les jetons sous l'édition du thème `ns` : défauts, défauts de composant
    /// des packs du deck, puis la chaîne de ses parents (la sienne pour un
    /// intégré, dont l'édition fait un enfant), dans la bibliothèque.
    fn edit_lower(&self, ns: &Namespace, child: bool) -> Result<Map<String, Value>> {
        let closure = theme_closure(&self.themes, ns)?;
        let chain = chain_tokens(&closure, ns, child)?;
        let packs = |v: &PackVersion| self.pack_version_files(v);
        let defaults = pack_defaults(&packs, self.user_deck(), LayerReading::Lenient)?;
        let layer = oku_slides_core::tokens::theme_layer(defaults.iter().map(|(n, d)| (n, d.clone())), chain.iter());
        let mut lower = default_tokens().clone();
        merge(&mut lower, &layer);
        Ok(lower)
    }

    /// Édite le thème `id` : ops de jetons et nom.
    ///
    /// Part de sa version courante dans la bibliothèque : de ses jetons pour
    /// un thème de l'utilisateur, d'aucun pour un intégré (dont l'édition
    /// enregistre un enfant).
    ///
    /// Refusé hors du mode deck. Un nom vide garde le nom actuel. Sans
    /// changement, rien n'est enregistré.
    pub fn edit_theme(&mut self, id: Uuid, name: Option<String>, ops: Vec<TokenOp>) -> Result<EditedTheme> {
        ensure!(matches!(self.mode, Mode::Deck), EditingPackSnafu);
        let (current, base) = self.theme_base(id)?;
        let child = ThemeLibrary::is_builtin(id);
        let ops: Vec<Op> = ops.into_iter().map(Op::from).collect();
        let start = if child { Map::new() } else { base.tokens.clone() };
        let lower = self.edit_lower(&base.namespace, child)?;
        let schema = (self.token_schema(self.user_deck()), self.schema_namespaces(self.user_deck()));
        self.check_token_ops_on(&schema, &lower, &Value::Object(start.clone()), &ops, true)?;
        let mut scratch = token_deck(Value::Object(start.clone()));
        oku_slides_core::apply_all(&mut scratch, ops, &self.registry).context(OpSnafu)?;
        let tokens = match scratch.tokens {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        let renamed = name.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty() && *n != base.name);
        if tokens == start && renamed.is_none() {
            let (name, namespace, changed, adopted) = (base.name, base.namespace, false, false);
            return Ok(EditedTheme { id, namespace, version: current, name, changed, adopted, tokens: base.tokens });
        }
        let name = match renamed {
            Some(n) => n,
            None if child => self.localizer.tr_with("theme-copy-name", &[("name", base.name.as_str().into())]),
            None => base.name.clone(),
        };
        let manifest = match child {
            true => ThemeManifest {
                id: Uuid::new_v4(),
                namespace: library_namespace(&self.library, &name)?,
                extends: vec![base.namespace.clone()],
                name,
                description: base.description,
                parent: None,
                created: now_created(),
                tokens,
            },
            false => ThemeManifest { name, parent: Some(current), created: now_created(), tokens, ..base },
        };
        let new_id = manifest.id;
        self.library.check_theme(new_id, &manifest.namespace)?;
        let files = manifest.to_files().context(ThemeJsonSnafu { id: new_id })?;
        let version = files.version();
        let adopted = self.deck_theme_pin().is_some_and(|t| t.id == id);
        self.themes.add_version(new_id, &files)?;
        self.themes.set_current(new_id, &version)?;
        self.theme_list = self.themes.themes();
        if adopted {
            self.adopt_edited_theme(&manifest.namespace)?;
        }
        let ThemeManifest { name, namespace, tokens, .. } = manifest;
        Ok(EditedTheme { id: new_id, namespace, version, name, changed: true, adopted, tokens })
    }

    /// Le deck passe au thème édité `ns` (sa version courante et sa fermeture), en une étape annulable.
    fn adopt_edited_theme(&mut self, ns: &Namespace) -> Result<()> {
        // `apply` embarque la fermeture, prise dans la bibliothèque.
        let label = self.localizer.tr("session-undo-theme");
        self.apply(&label, vec![Op::SetTheme { theme: Some(ns.clone()), themes: Default::default() }], None)
    }
}

#[cfg(test)]
#[path = "deck_theme_edit_tests.rs"]
mod deck_theme_edit_tests;
