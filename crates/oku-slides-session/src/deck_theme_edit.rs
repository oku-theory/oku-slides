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

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{DECK_VERSION, Deck, Namespace, Op, PackFiles, PackVersion, ThemePin};

use crate::deck_theme::{LayerReading, pack_defaults};
use crate::document_kind::refused_in_theme_tab;
use crate::error::{EditingPackSnafu, OpSnafu, ThemeJsonSnafu, ThemeVersionMissingSnafu, UnknownThemeSnafu};
use crate::library::draft_lock;
use crate::library_lookup::find_theme;
use crate::namespace_slug::library_namespace;
use crate::pack::now_created;
use crate::pack_lease::refuse_held;
use crate::theme::{ThemeInfo, ThemeLibrary, ThemeManifest};
use crate::theme_closure::{Closure, chain_tokens, closure_with, embedded_or_library, library_theme, pin_of};
use crate::theme_schema::{default_tokens, merge};
use crate::{Mode, Result, Session};

/// An op on a theme's tokens, of the same shape as the deck op.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TokenOp {
    /// Sets the token at the given path (`["color", "accent"]`); `null` is set as is.
    SetToken {
        /// Path of the token, segment by segment.
        path: Vec<String>,
        /// New JSON value of the token.
        value: Value,
    },
    /// Removes the token at the given path, back to the default values.
    UnsetToken {
        /// Path of the token, segment by segment.
        path: Vec<String>,
    },
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
    /// Theme id: that of the edited theme, or that of the created child.
    pub id: Uuid,
    /// Son espace de noms : celui du thème édité, ou celui de son enfant.
    pub namespace: Namespace,
    /// Hash of the current version after the edit.
    pub version: PackVersion,
    /// Theme name, possibly renamed or suffixed as a copy.
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
        themes: BTreeMap::default(),
        params: oku_slides_core::DeckParams::default(),
        vars: BTreeMap::default(),
        tokens,
        widgets: BTreeMap::default(),
        layouts: BTreeMap::default(),
        slides: Vec::new(),
    }
}

impl Session {
    /// La version d'où part l'édition du thème `id`, et son manifeste : sa version courante de la bibliothèque.
    fn theme_base(&self, id: Uuid) -> Result<(PackVersion, ThemeManifest)> {
        let current = self.themes.current_version(id).context(UnknownThemeSnafu { theme: id.to_string() })?;
        Ok((current.version, current.manifest))
    }

    /// Theme `id` at the version its edit would start from, reread from the library.
    ///
    /// # Errors
    ///
    /// Fails (`UnknownTheme`) if the library does not have theme `id`.
    pub fn theme_info(&self, id: Uuid) -> Result<ThemeInfo> {
        let current = self.themes.current_version(id).context(UnknownThemeSnafu { theme: id.to_string() })?;
        Ok(ThemeInfo::of(current))
    }

    /// The theme named by `wanted` (namespace, id or name, case-insensitive), at its current version.
    ///
    /// # Errors
    ///
    /// Fails (`UnknownTheme`) if no theme matches `wanted`.
    pub fn find_theme_info(&self, wanted: &str) -> Result<ThemeInfo> {
        let ns = find_theme(&self.themes, wanted).context(UnknownThemeSnafu { theme: wanted })?;
        let current = self.themes.by_namespace(&ns).context(UnknownThemeSnafu { theme: wanted })?;
        Ok(ThemeInfo::of(current))
    }

    /// The own tokens of theme `id`, at the version its edit would start from.
    ///
    /// Those changed by the ops of [`Self::edit_theme`]; a missing key
    /// comes from the default values.
    ///
    /// # Errors
    ///
    /// Fails if theme `id` is unknown or if its version cannot be read.
    pub fn theme_tokens_of(&self, id: Uuid) -> Result<Map<String, Value>> {
        Ok(self.theme_base(id)?.1.tokens)
    }

    /// Les jetons sous l'édition du thème `ns` : défauts, défauts de composant
    /// des packs du deck, puis la chaîne de ses parents (la sienne pour un
    /// intégré, dont l'édition fait un enfant).
    ///
    /// `ns` est pris dans sa version courante (celle d'où part l'édition) ;
    /// chaque parent dans la version que le deck épingle et embarque, sinon
    /// dans la bibliothèque (t-1106) : un parent que seul le deck a se lit.
    fn edit_lower(&self, ns: &Namespace, child: bool) -> Result<Map<String, Value>> {
        let root = library_theme(&self.themes, ns).context(UnknownThemeSnafu { theme: ns.as_str() })?;
        let closure = self.deck_closure(ns, &root)?;
        let chain = chain_tokens(&closure, ns, child)?;
        let packs = |v: &PackVersion| self.pack_version_files(v);
        let defaults = pack_defaults(&packs, self.user_deck(), LayerReading::Lenient)?;
        let layer = oku_slides_core::tokens::theme_layer(defaults.iter().map(|(n, d)| (n, d.clone())), chain.iter());
        let mut lower = default_tokens().clone();
        merge(&mut lower, &layer);
        Ok(lower)
    }

    /// La fermeture du thème `ns` de version `root` : ses parents dans les
    /// versions que le deck embarque, sinon dans la bibliothèque (t-1106).
    fn deck_closure(&self, ns: &Namespace, root: &(ThemePin, PackFiles)) -> Result<Closure> {
        let sources = [self.user_blobs()];
        closure_with(ns, |n| if n == ns { Some(root.clone()) } else { embedded_or_library(&self.themes, &sources, n) })
    }

    /// Edits theme `id`: token ops and name.
    ///
    /// Starts from its current version in the library: from its tokens for
    /// a user theme, from none for a built-in one (whose edit saves a
    /// child). A user theme's version it builds on is frozen first.
    ///
    /// Rejected outside deck mode. An empty name keeps the current name.
    /// Without a change, nothing is saved. A user theme that a theme tab
    /// edits is refused: its tab writes it.
    ///
    /// # Errors
    ///
    /// Fails outside deck mode (`EditingPack`; `WrongDocument` in a theme tab), for an unknown
    /// theme, while a theme tab holds it (`ThemeOpenInTab`), for token ops rejected by the
    /// schema, without the bytes of the version it builds on (`ThemeVersionMissing`), or if the
    /// library does not freeze that version or write the new one.
    ///
    /// # Blocking
    ///
    /// Reads and writes the theme library, under `DRAFTS` for the freeze and the write.
    pub fn edit_theme(&mut self, id: Uuid, name: Option<String>, ops: Vec<TokenOp>) -> Result<EditedTheme> {
        if matches!(self.mode, Mode::Theme { .. }) {
            return refused_in_theme_tab();
        }
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
        let manifest = if child {
            ThemeManifest {
                id: Uuid::new_v4(),
                namespace: library_namespace(&self.library, &name, "pack")?,
                extends: vec![base.namespace.clone()],
                name,
                description: base.description,
                parent: None,
                created: now_created(),
                tokens,
            }
        } else {
            ThemeManifest { name, parent: Some(current.clone()), created: now_created(), tokens, ..base }
        };
        let new_id = manifest.id;
        self.library.check_theme(new_id, &manifest.namespace)?;
        let files = manifest.to_files().context(ThemeJsonSnafu { id: new_id })?;
        let version = files.version();
        let adopted = self.deck_theme_pin().is_some_and(|t| t.id == id);
        // Its parents must be found before anything is written.
        let (pins, _) = self.deck_closure(&manifest.namespace, &(pin_of(&manifest, version.clone()), files.clone()))?;
        // The bytes of the version it builds on are read before `DRAFTS`: no disk read under it.
        let missing = || ThemeVersionMissingSnafu { id, version: current.clone() };
        let built_on = if child { None } else { Some(self.themes.version(id, &current).context(missing())?) };
        // Checked and written under one `DRAFTS` scope: no tab takes the theme in between.
        let lock = draft_lock();
        refuse_held(self.library.theme_scope(), new_id, &manifest.name)?;
        // Frozen first: a sleeping theme tab's next save never replaces this version's parent (t-1223).
        if let Some(files) = &built_on {
            self.themes.adopt(&lock, id, files)?;
        }
        self.themes.add_version(new_id, &files)?;
        self.themes.set_current(new_id, &version)?;
        drop(lock);
        self.theme_list = self.themes.themes();
        if adopted {
            self.adopt_edited_theme(&manifest.namespace, pins)?;
        }
        let ThemeManifest { name, namespace, tokens, .. } = manifest;
        Ok(EditedTheme { id: new_id, namespace, version, name, changed: true, adopted, tokens })
    }

    /// Le deck passe au thème édité `ns` et à sa fermeture `themes`, en une étape annulable.
    fn adopt_edited_theme(&mut self, ns: &Namespace, themes: BTreeMap<Namespace, ThemePin>) -> Result<()> {
        // `apply` embarque les versions qui manquent au deck, prises dans la bibliothèque.
        let label = self.localizer.tr("session-undo-theme");
        self.apply(&label, vec![Op::SetTheme { theme: Some(ns.clone()), themes }], None)
    }
}

#[cfg(test)]
#[path = "deck_theme_edit_tests.rs"]
mod deck_theme_edit_tests;
