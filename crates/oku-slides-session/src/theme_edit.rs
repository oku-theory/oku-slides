//! A theme tab's draft: the specimen deck carries its tokens.
//!
//! The specimen deck has no theme (`theme: None`). Editing a user theme
//! starts from its own tokens; editing a built-in starts from an empty
//! draft, that of a child (`ThemeEdit::child`) that will extend it. The
//! specimen deck renders over the layer of the themes the draft extends
//! (`ThemeEdit::extends`, first its base's parents, or the built-in itself
//! for a child; `theme_draft`), under the draft its tokens hold. Each setting
//! is a `SetToken` / `UnsetToken`, checked by the specimen deck's composed
//! schema (`theme_schema`, `theme_composed`). A tab never changes the theme
//! it edits; changing the `extends` list is an undo step under a rank
//! (`theme_extends`), which the draft follows on undo (`ThemeSwitch`).
//! Renaming the theme is a plain state change, without a step.

use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::OptionExt;

use oku_slides_core::{Deck, Namespace, Op, ThemePin};
use uuid::Uuid;

use crate::deck_theme_edit::token_deck;
use crate::error::UnknownThemeSnafu;
use crate::theme::{ThemeLibrary, ThemeManifest};
use crate::theme_closure::pin_of;
use crate::theme_composed::without_unpinned_components;
use crate::theme_schema::{
    Control, TokenSpec, default_tokens, ensure_unused, lookup, merge, spec, validate, validate_tree,
};
use crate::{Mode, Result, Session};

/// The theme a theme tab edits: its base version, name and parents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeEdit {
    /// La version de base du brouillon : id, version et parents.
    pub base: ThemePin,
    /// L'espace de noms de la base, lu à la création de l'édition.
    pub namespace: Namespace,
    /// Le nom que prendra le thème enregistré.
    pub name: String,
    /// Les thèmes qu'étend le brouillon, dans l'ordre (le dernier l'emporte).
    pub extends: Vec<Namespace>,
}

impl ThemeEdit {
    /// L'édition de la version `base` d'espace de noms `namespace`, nommée `name`, étendant `extends`.
    pub(crate) fn of(base: ThemePin, namespace: Namespace, name: String, extends: Vec<Namespace>) -> Self {
        Self { base, namespace, name, extends }
    }

    /// L'édition de la version `base`, de manifeste `manifest`, partant de ses parents de départ.
    pub(crate) fn starting(base: ThemePin, manifest: &ThemeManifest) -> Self {
        let mut edit = Self::of(base, manifest.namespace.clone(), manifest.name.clone(), Vec::new());
        edit.extends = edit.starting_extends(manifest);
        edit
    }

    /// Les parents de départ : l'intégré lui-même pour un enfant, sinon ceux de la base `base`.
    pub(crate) fn starting_extends(&self, base: &ThemeManifest) -> Vec<Namespace> {
        if self.child() { vec![self.namespace.clone()] } else { base.extends.clone() }
    }

    /// La base est intégrée : le brouillon part vide et s'enregistre en un enfant qui l'étend.
    ///
    /// Tiré de la base, jamais gardé : un état de reprise ne peut le contredire.
    #[must_use]
    pub fn child(&self) -> bool {
        ThemeLibrary::is_builtin(self.base.id)
    }
}

/// Le thème édité, vu par l'interface.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeEditView {
    /// La version de base du brouillon : id, version et parents.
    pub base: ThemePin,
    /// L'espace de noms de la base.
    pub namespace: String,
    /// Le nom que prendra le thème enregistré.
    pub name: String,
    /// La base est intégrée : l'enregistrer en crée un enfant qui l'étend.
    pub child: bool,
    /// Les thèmes qu'étend le brouillon, dans l'ordre.
    pub extends: Vec<ThemeParentView>,
    /// Le brouillon diffère de son point de départ (jetons, nom ou parents).
    pub dirty: bool,
    /// Empreinte des jetons du brouillon : change avec eux, quelle que soit
    /// la voie (réglage, annulation, reprise). Les aperçus des widgets s'y
    /// règlent : deux réglages du même jeton ont le même libellé d'annulation.
    pub revision: String,
}

/// Un thème qu'étend le brouillon, vu par l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeParentView {
    /// Son espace de noms.
    pub namespace: String,
    /// Son nom ; son espace de noms s'il est introuvable.
    pub name: String,
}

/// A change of the draft's parents (or of its base, on a save), under its rank:
/// undoing or redoing it brings back the draft that goes with it.
#[derive(Debug, Clone)]
pub(crate) struct ThemeSwitch {
    /// Le rang écrit dans [`DRAFT_SWITCH_VAR`] par ce changement.
    pub(crate) rank: u64,
    pub(crate) from: ThemeEdit,
    pub(crate) to: ThemeEdit,
}

/// Variable of the specimen deck that holds the rank of the last change of
/// the draft's parents (absent: none). Each change writes it, in one step,
/// under a new rank: above the deck's and every rank of the session
/// ([`next_switch_rank`]). The step thus always changes the deck, even when
/// the tokens do not, and undo or redo brings back a rank the draft is read
/// from. A rank with no known change (before the first change, or before a
/// resume, which keeps no changes) is the draft before the first one. No
/// template names it: it never shows in the render. Only the session's own
/// rank steps write it: a caller's `SetVar` of it is refused (`WrongDocument`).
pub(crate) const DRAFT_SWITCH_VAR: &str = "oku-slides-theme-switch";

/// Le rang du changement de base que porte `deck` (0 : aucun).
pub(crate) fn switch_rank(deck: &Deck) -> u64 {
    deck.vars.get(DRAFT_SWITCH_VAR).and_then(Value::as_u64).unwrap_or(0)
}

/// Un rang neuf : au-delà de celui de `deck` et de ceux de `switches`.
pub(crate) fn next_switch_rank(deck: &Deck, switches: &[ThemeSwitch]) -> u64 {
    switches.iter().map(|s| s.rank).fold(switch_rank(deck), u64::max) + 1
}

/// Les jetons d'un deck, en objet (aucun : vide).
fn tokens_of(v: &Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

/// True when the draft `edit` of tokens `tokens` differs from its base `base` (tokens, name or parents).
pub(crate) fn draft_differs(edit: &ThemeEdit, base: &ThemeManifest, tokens: &Map<String, Value>) -> bool {
    let origin = if edit.child() { Map::new() } else { base.tokens.clone() };
    base.name != edit.name || edit.extends != edit.starting_extends(base) || origin != *tokens
}

/// `tokens` en valeur de deck : un objet vide n'en est pas une.
pub(crate) fn tokens_value(tokens: Map<String, Value>) -> Value {
    if tokens.is_empty() { Value::Null } else { Value::Object(tokens) }
}

/// True for an op that touches the tokens.
pub(crate) fn is_token_op(op: &Op) -> bool {
    matches!(op, Op::SetToken { .. } | Op::UnsetToken { .. } | Op::SetTokens { .. })
}

/// Le jeton `path` existe dans `tokens`.
fn has_path(tokens: &Map<String, Value>, path: &[String]) -> bool {
    lookup(tokens, path).is_some()
}

/// A token op the schema checks outside a theme tab: the value of a color or
/// a palette (rule t-194).
fn checked_outside_drafts(schema: &[TokenSpec], path: &[String]) -> bool {
    spec(schema, path).is_some_and(|s| matches!(s.control, Control::Color | Control::Palette))
}

impl Session {
    /// The working deck's tokens: the theme draft in a theme tab.
    pub(crate) fn edited_tokens(&self) -> Map<String, Value> {
        tokens_of(&self.doc.sources.deck().tokens)
    }

    /// The deck whose style the previews (layouts, widgets) take: the working
    /// deck in pack mode (the deck's theme and overrides) and in a theme tab
    /// (the draft); otherwise the user's deck.
    pub(crate) fn style_deck(&self) -> &Deck {
        match &self.mode {
            Mode::Pack { .. } | Mode::Theme { .. } => self.doc.sources.deck(),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => self.user_deck(),
        }
    }

    /// Le manifeste de la version `pin` : embarquée, sinon dans la bibliothèque.
    pub(crate) fn theme_version(&self, pin: &ThemePin) -> Option<ThemeManifest> {
        self.theme_manifest(pin).or_else(|| ThemeManifest::from_files(&self.themes.version(pin.id, &pin.version)?).ok())
    }

    /// Embeds the draft's base version in the specimen deck if no source holds it.
    ///
    /// The base is then read in memory, without the library (the `host-theme`
    /// surface and the snapshot read it often).
    pub(crate) fn embed_theme_base(&mut self) {
        let Mode::Theme { edit, .. } = &self.mode else { return };
        let base = edit.draft.base.clone();
        if self.theme_manifest(&base).is_some() {
            return;
        }
        if let Some(files) = self.themes.version(base.id, &base.version) {
            self.doc.sources.add_theme(files);
        }
    }

    /// The manifest of the draft's base version.
    pub(crate) fn base_manifest(&self) -> Option<ThemeManifest> {
        let Mode::Theme { edit, .. } = &self.mode else { return None };
        self.theme_version(&edit.draft.base)
    }

    /// The tokens the draft starts from: none for a child, else its base's
    /// (none outside a theme tab). A token brought back (↺) takes its value here.
    pub(crate) fn base_tokens(&self) -> Map<String, Value> {
        match &self.mode {
            Mode::Theme { edit, .. } if !edit.draft.child() => {
                self.base_manifest().map(|m| m.tokens).unwrap_or_default()
            }
            _ => Map::new(),
        }
    }

    /// The draft a theme tab on `theme` starts from, at its current library version, and its tokens.
    ///
    /// A built-in gives an empty draft (`child`).
    pub(crate) fn initial_theme_edit(&self, theme: Uuid) -> Result<(ThemeEdit, Map<String, Value>)> {
        let current = self.themes.current_version(theme).context(UnknownThemeSnafu { theme: theme.to_string() })?;
        let edit = ThemeEdit::starting(pin_of(&current.manifest, current.version), &current.manifest);
        let tokens = if edit.child() { Map::new() } else { current.manifest.tokens };
        Ok((edit, tokens))
    }

    /// The hash of the working deck's tokens (stable within one app version).
    fn draft_revision(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.doc.sources.deck().tokens.to_string().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// True when the draft `edit` differs from its starting point (tokens, name or parents).
    fn theme_dirty(&self, edit: &ThemeEdit) -> bool {
        let Some(base) = self.base_manifest() else { return true };
        draft_differs(edit, &base, &self.edited_tokens())
    }

    /// The draft `edit` as the interface sees it.
    pub(crate) fn theme_edit_view(&self, edit: &ThemeEdit) -> ThemeEditView {
        ThemeEditView {
            base: edit.base.clone(),
            namespace: edit.namespace.to_string(),
            name: edit.name.clone(),
            child: edit.child(),
            extends: self.draft_parents(edit),
            dirty: self.theme_dirty(edit),
            revision: self.draft_revision(),
        }
    }

    /// The theme tab's draft becomes `draft`.
    pub(crate) fn set_theme_edit(&mut self, draft: ThemeEdit) {
        if let Mode::Theme { edit, .. } = &mut self.mode {
            edit.draft = draft;
        }
    }

    /// Renames the edited theme; an empty name takes back its base's name.
    ///
    /// A state change, with no undo step: the name is not in the deck.
    ///
    /// # Errors
    ///
    /// Fails outside a theme tab (`WrongDocument`).
    pub fn rename_edited_theme(&mut self, name: &str) -> Result<()> {
        self.ensure_theme_tab()?;
        let name = match name.trim() {
            "" => self.base_manifest().map(|m| m.name).unwrap_or_default(),
            trimmed => trimmed.to_owned(),
        };
        if let Mode::Theme { edit, .. } = &mut self.mode {
            edit.draft.name = name;
        }
        self.doc.dirty = true;
        Ok(())
    }

    /// After undo or redo: the draft follows the rank ([`DRAFT_SWITCH_VAR`]) the specimen deck brings back.
    ///
    /// A change of the session's rank: the draft it leads to; any other rank
    /// (none, or from before the session): the draft before its first change.
    /// Without a change in the session (after a resume the history starts
    /// empty), nothing moves.
    pub(crate) fn realign_theme_edit(&mut self) {
        let rank = switch_rank(self.doc.sources.deck());
        let Mode::Theme { edit, switches, .. } = &mut self.mode else { return };
        let reached = switches.iter().find(|s| s.rank == rank).map(|s| &s.to);
        if let Some(draft) = reached.or_else(|| switches.first().map(|s| &s.from)) {
            edit.draft = draft.clone();
        }
        self.embed_theme_base();
    }

    /// Checks the token ops of `ops` before they apply.
    ///
    /// In a theme tab (the draft): each `SetToken` goes through the specimen
    /// deck's composed schema, and an `UnsetToken` that removes a token still
    /// named elsewhere is refused. Elsewhere: only the colors and palettes of
    /// the deck's overrides are checked (t-194). Each op is checked in the
    /// effective tokens (defaults, theme, overrides) the previous ones left; a
    /// `SetTokens` leaf by leaf, by the same rule.
    pub(crate) fn check_token_ops(&self, ops: &[Op]) -> Result<()> {
        let drafting = matches!(self.mode, Mode::Theme { .. });
        if !ops.iter().any(is_token_op) {
            return Ok(());
        }
        let deck = self.doc.sources.deck();
        let mut lower = default_tokens().clone();
        merge(&mut lower, &self.theme_layer(deck)?);
        let schema = (self.token_schema(deck), self.schema_namespaces(deck));
        self.check_token_ops_on(&schema, &lower, &deck.tokens, ops, drafting)
    }

    /// Vérifie les ops de jetons de `ops`, appliquées une à une à `tokens`
    /// au-dessus de `lower`, selon le schéma composé `schema` et les espaces de
    /// noms de ses packs (voir `check_token_ops`) : un `SetTokens` strict
    /// accepte tel quel un sous-arbre `component.<ns>` d'un pack qu'ils ne
    /// tiennent pas (`without_unpinned_components`). `drafting` : les règles du brouillon d'un thème
    /// (tout `SetToken` passe par le schéma, un jeton retiré ne doit plus être
    /// nommé) ; sinon, couleurs et palettes seules.
    pub(crate) fn check_token_ops_on(
        &self,
        (schema, pinned): &(Vec<TokenSpec>, BTreeSet<Namespace>),
        lower: &Map<String, Value>,
        tokens: &Value,
        ops: &[Op],
        drafting: bool,
    ) -> Result<()> {
        let effective = |own: &Value| {
            let mut tokens = lower.clone();
            merge(&mut tokens, &tokens_of(own));
            tokens
        };
        let mut own = token_deck(tokens.clone());
        for op in ops {
            if !is_token_op(op) {
                continue;
            }
            match op {
                Op::SetToken { path, value } if drafting || checked_outside_drafts(schema, path) => {
                    validate(schema, path, value, &effective(&own.tokens))?;
                }
                Op::SetTokens { tokens } => {
                    let checked = without_unpinned_components(&tokens_of(tokens), pinned);
                    validate_tree(schema, &checked, &effective(tokens), drafting)?;
                }
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
                    ensure_unused(schema, path, &after)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "theme_edit_tests.rs"]
pub(crate) mod theme_edit_tests;

#[cfg(test)]
#[path = "theme_edit_switch_tests.rs"]
mod theme_edit_switch_tests;
