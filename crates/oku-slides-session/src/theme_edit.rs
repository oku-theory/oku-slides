//! Brouillon du thème en mode pack : le deck de travail porte ses jetons.
//!
//! À l'entrée du mode pack, le deck de travail perd son thème (`theme:
//! None`). Éditer un thème de l'utilisateur part de ses propres jetons ;
//! éditer un thème intégré part d'un brouillon vide, celui d'un enfant
//! (`ThemeEdit::child`) qui l'étendra. Le deck de travail se rend sur le
//! calque des thèmes qu'étend le brouillon (`ThemeEdit::extends`, au départ
//! les parents de sa base, ou l'intégré lui-même pour un enfant ;
//! `theme_draft`), sous le brouillon qu'il porte dans ses jetons. Chaque
//! réglage est un `SetToken` / `UnsetToken`, vérifié par le schéma composé du
//! deck de travail (`theme_schema`, `theme_composed`) ; choisir un autre thème
//! remplace les jetons en une étape annulable (`SetTokens`), et la base suit
//! l'annulation (`ThemeSwitch`). Changer la liste `extends` est aussi une
//! étape sous un rang (`theme_extends`). Renommer le thème est un simple
//! changement d'état, sans étape.
//!
//! L'enregistrement (`theme_draft`) fait d'un brouillon qui diffère de son
//! point de départ une nouvelle version d'un thème de l'utilisateur, ou un
//! enfant de l'intégré édité.

use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::OptionExt;

use oku_slides_core::{Deck, Namespace, Op, ThemePin};

use crate::deck_theme_edit::token_deck;
use crate::error::{NotEditingPackSnafu, ThemeChoiceAloneSnafu, ThemeVersionMissingSnafu, UnknownThemeSnafu};
use crate::theme::{ThemeLibrary, ThemeManifest, default_namespace};
use crate::theme_closure::pin_of;
use crate::theme_composed::without_unpinned_components;
use crate::theme_schema::{
    Control, TokenSpec, default_tokens, ensure_unused, lookup, merge, spec, validate, validate_tree,
};
use crate::views::EditingView;
use crate::{Mode, Result, Session};

/// Le thème édité en mode pack : sa version de base, son nom et ses parents.
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

/// Un changement de base (`set_edited_theme`) ou de parents, sous son rang : l'annuler ou
/// le rétablir ramène la base qui va avec.
#[derive(Debug, Clone)]
pub(crate) struct ThemeSwitch {
    /// Le rang écrit dans [`DRAFT_SWITCH_VAR`] par ce changement.
    pub(crate) rank: u64,
    pub(crate) from: ThemeEdit,
    pub(crate) to: ThemeEdit,
}

/// Variable du deck de travail qui porte le rang du dernier changement de
/// base fait (absente : aucun). Chaque choix d'un thème l'écrit avec les
/// jetons, en une étape, sous un rang neuf : plus grand que celui du deck et
/// que tous ceux de la séance ([`next_switch_rank`]). L'étape change donc
/// toujours le deck, même quand les jetons ne changent pas (d'un intégré à
/// l'autre), et annuler ou rétablir ramène un rang d'où se relit la base. Un
/// rang sans changement connu (celui d'avant le premier changement, ou
/// d'avant une reprise, qui ne garde pas les changements) est la base d'avant
/// le premier. Aucun gabarit ne la nomme : elle ne se voit pas au rendu, et le
/// deck de l'utilisateur ne la reçoit jamais.
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

/// `tokens` en valeur de deck : un objet vide n'en est pas une.
pub(crate) fn tokens_value(tokens: Map<String, Value>) -> Value {
    if tokens.is_empty() { Value::Null } else { Value::Object(tokens) }
}

/// Une op qui touche aux jetons.
fn is_token_op(op: &Op) -> bool {
    matches!(op, Op::SetToken { .. } | Op::UnsetToken { .. } | Op::SetTokens { .. })
}

/// Le jeton `path` existe dans `tokens`.
fn has_path(tokens: &Map<String, Value>, path: &[String]) -> bool {
    lookup(tokens, path).is_some()
}

/// Une op de jetons, vérifiée par le schéma en dehors du mode pack : la
/// valeur d'une couleur ou d'une palette (règle t-194).
fn checked_outside_drafts(schema: &[TokenSpec], path: &[String]) -> bool {
    spec(schema, path).is_some_and(|s| matches!(s.control, Control::Color | Control::Palette))
}

impl Session {
    /// Les jetons du deck de travail : le brouillon du thème en mode pack.
    pub(crate) fn edited_tokens(&self) -> Map<String, Value> {
        tokens_of(&self.doc.sources.deck().tokens)
    }

    /// Le deck dont les aperçus (layouts, widgets) prennent le style : en mode
    /// pack, le deck de travail (le brouillon du thème, sans surcharges du
    /// deck) ; sinon le deck de l'utilisateur.
    pub(crate) fn style_deck(&self) -> &Deck {
        match &self.mode {
            Mode::Pack { .. } => self.doc.sources.deck(),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => self.user_deck(),
        }
    }

    /// Le manifeste de la version `pin` : embarquée, sinon dans la bibliothèque.
    pub(crate) fn theme_version(&self, pin: &ThemePin) -> Option<ThemeManifest> {
        self.theme_manifest(pin).or_else(|| ThemeManifest::from_files(&self.themes.version(pin.id, &pin.version)?).ok())
    }

    /// La version courante du thème `ns` de la bibliothèque, épinglée, et son manifeste.
    fn library_theme(&self, ns: &Namespace) -> Result<(ThemePin, ThemeManifest)> {
        let current = self.themes.by_namespace(ns).context(UnknownThemeSnafu { theme: ns.as_str() })?;
        Ok((pin_of(&current.manifest, current.version), current.manifest))
    }

    /// Embarque la version de base du brouillon dans le deck de travail si
    /// aucune source ne la porte : la base se lit ensuite en mémoire, sans
    /// la bibliothèque (la surface `host-theme` et le snapshot la relisent
    /// souvent). Le deck de l'utilisateur n'en reçoit rien : seul un
    /// enregistrement y embarque une version.
    pub(crate) fn embed_theme_base(&mut self) {
        let Mode::Pack { theme, .. } = &self.mode else { return };
        let base = theme.base.clone();
        if self.theme_manifest(&base).is_some() {
            return;
        }
        if let Some(files) = self.themes.version(base.id, &base.version) {
            self.doc.sources.add_theme(files);
        }
    }

    /// Le manifeste de la version de base du brouillon.
    pub(crate) fn base_manifest(&self) -> Option<ThemeManifest> {
        let Mode::Pack { theme, .. } = &self.mode else { return None };
        self.theme_version(&theme.base)
    }

    /// Les jetons d'où part le brouillon : vides pour un enfant, sinon ceux de sa base
    /// (vides hors mode pack). Un jeton ramené (↺) reprend sa valeur ici.
    pub(crate) fn base_tokens(&self) -> Map<String, Value> {
        match &self.mode {
            Mode::Pack { theme, .. } if !theme.child() => self.base_manifest().map(|m| m.tokens).unwrap_or_default(),
            _ => Map::new(),
        }
    }

    /// Le thème à éditer en entrant en mode pack, et ses jetons : celui du
    /// deck de l'utilisateur, sinon la version courante du thème par défaut.
    /// Un intégré donne un brouillon vide (`child`).
    pub(crate) fn initial_theme_edit(&self) -> Result<(ThemeEdit, Map<String, Value>)> {
        let base = match self.deck_theme_pin() {
            Some(pin) => pin.clone(),
            None => self.library_theme(&default_namespace()?)?.0,
        };
        let manifest = self
            .theme_version(&base)
            .context(ThemeVersionMissingSnafu { id: base.id, version: base.version.clone() })?;
        let edit = ThemeEdit::starting(base, &manifest);
        let tokens = if edit.child() { Map::new() } else { manifest.tokens };
        Ok((edit, tokens))
    }

    /// L'empreinte des jetons du deck de travail (stable dans une même version de l'app).
    fn draft_revision(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.doc.sources.deck().tokens.to_string().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Le brouillon diffère-t-il de son point de départ (jetons, nom ou parents) ?
    fn theme_dirty(&self, edit: &ThemeEdit) -> bool {
        let Some(base) = self.base_manifest() else { return true };
        base.name != edit.name
            || edit.extends != edit.starting_extends(&base)
            || self.base_tokens() != self.edited_tokens()
    }

    /// Pack mode for the snapshot: the pack, the theme draft and the lease flags.
    pub(crate) fn editing_view(&self) -> Option<EditingView<'_>> {
        let Mode::Pack { edit, theme, .. } = &self.mode else { return None };
        let view = ThemeEditView {
            base: theme.base.clone(),
            namespace: theme.namespace.to_string(),
            name: theme.name.clone(),
            child: theme.child(),
            extends: self.draft_parents(theme),
            dirty: self.theme_dirty(theme),
            revision: self.draft_revision(),
        };
        let (read_only, leased) = self.edit_flags();
        let display_name = crate::agent::bounds::label(&edit.name);
        Some(EditingView { mode: edit, theme: view, read_only, leased, display_name })
    }

    /// The draft starts from theme `ns`, at its current version in the library.
    ///
    /// Its tokens (none for a built-in, whose draft will be a child) replace
    /// those of the working deck in one undoable step, which also carries the
    /// change's rank ([`DRAFT_SWITCH_VAR`]): a choice that does not change the
    /// tokens (from one built-in to another) gets its own step.
    ///
    /// # Errors
    ///
    /// Fails outside pack mode (`NotEditingPack`), for a theme unknown to the library, or if the
    /// transaction is rejected.
    pub fn set_edited_theme(&mut self, ns: &Namespace) -> Result<()> {
        let Mode::Pack { theme, switches, .. } = &self.mode else {
            return NotEditingPackSnafu.fail()?;
        };
        let from = theme.clone();
        let rank = next_switch_rank(self.doc.sources.deck(), switches);
        let (base, manifest) = self.library_theme(ns)?;
        let label = self.localizer.tr_with("session-undo-edited-theme", &[("name", manifest.name.as_str().into())]);
        let to = ThemeEdit::starting(base, &manifest);
        let tokens = if to.child() { Map::new() } else { manifest.tokens };
        // La base change avant `apply`, qui écrit l'état de reprise : il la
        // garde avec les jetons. Un échec la remet.
        self.set_theme_edit(to.clone());
        self.embed_theme_base();
        let ops = vec![
            Op::SetTokens { tokens: tokens_value(tokens) },
            Op::SetVar { key: DRAFT_SWITCH_VAR.to_owned(), value: Value::from(rank) },
        ];
        if let Err(e) = self.apply(&label, ops, None) {
            self.set_theme_edit(from);
            return Err(e);
        }
        if let Mode::Pack { switches, .. } = &mut self.mode {
            switches.push(ThemeSwitch { rank, from, to });
        }
        Ok(())
    }

    /// Le thème édité devient `edit` (en mode pack).
    pub(crate) fn set_theme_edit(&mut self, edit: ThemeEdit) {
        if let Mode::Pack { theme, .. } = &mut self.mode {
            *theme = edit;
        }
    }

    /// Renames the edited theme; an empty name takes back its base's name.
    ///
    /// A state change, with no undo step: the name is not in the deck.
    ///
    /// # Errors
    ///
    /// Fails outside pack mode (`NotEditingPack`).
    pub fn rename_edited_theme(&mut self, name: &str) -> Result<()> {
        if !matches!(self.mode, Mode::Pack { .. }) {
            return NotEditingPackSnafu.fail()?;
        }
        let name = match name.trim() {
            "" => self.base_manifest().map(|m| m.name).unwrap_or_default(),
            trimmed => trimmed.to_owned(),
        };
        if let Mode::Pack { theme, .. } = &mut self.mode {
            theme.name = name;
        }
        self.doc.dirty = true;
        Ok(())
    }

    /// Après annuler ou rétablir : la base suit le rang du changement de
    /// thème ([`DRAFT_SWITCH_VAR`]) que ramène le deck de travail.
    ///
    /// Le rang d'un changement de la séance : la base où il mène ; tout autre
    /// rang (aucun, ou d'avant la séance) : la base d'avant son premier
    /// changement. Sans changement dans la séance (après un changement de pack
    /// ou une reprise, l'historique repart vide), rien ne bouge.
    pub(crate) fn realign_theme_edit(&mut self) {
        let rank = switch_rank(self.doc.sources.deck());
        let Mode::Pack { theme, switches, .. } = &mut self.mode else { return };
        let reached = switches.iter().find(|s| s.rank == rank).map(|s| &s.to);
        let edit = reached.or_else(|| switches.first().map(|s| &s.from));
        if let Some(edit) = edit {
            *theme = edit.clone();
        }
        self.embed_theme_base();
    }

    /// En mode pack, une transaction faite du seul choix d'un thème : son
    /// espace de noms (`null` : le thème par défaut), dont le brouillon part
    /// alors (`set_edited_theme`). `None` ailleurs ou pour toute autre transaction.
    pub(crate) fn drafted_theme_choice(&self, ops: &[Op]) -> Option<Result<Namespace>> {
        match (&self.mode, ops) {
            (Mode::Pack { .. }, [Op::SetTheme { theme, .. }]) => Some(theme.clone().map_or_else(default_namespace, Ok)),
            _ => None,
        }
    }

    /// Vérifie les ops de jetons de `ops` avant leur application.
    ///
    /// En mode pack (brouillon du thème) : chaque `SetToken` passe par le
    /// schéma composé du deck de travail, et un `UnsetToken` qui retire un jeton encore nommé ailleurs
    /// est refusé. Ailleurs : seules les couleurs et palettes des surcharges
    /// du deck sont vérifiées (t-194). Chaque op est vérifiée dans les jetons
    /// effectifs (défauts, thème, surcharges) laissés par les précédentes ; un
    /// `SetTokens` l'est feuille à feuille, selon la même règle. En mode
    /// pack, un `SetTheme` mêlé à d'autres ops est refusé : le thème s'y
    /// choisit seul (`drafted_theme_choice`), ses jetons par `SetToken`.
    pub(crate) fn check_token_ops(&self, ops: &[Op]) -> Result<()> {
        let drafting = matches!(self.mode, Mode::Pack { .. });
        if drafting && ops.iter().any(|op| matches!(op, Op::SetTheme { .. })) {
            return ThemeChoiceAloneSnafu.fail()?;
        }
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
#[path = "theme_edit_save_tests.rs"]
mod theme_edit_save_tests;

#[cfg(test)]
#[path = "theme_edit_switch_tests.rs"]
mod theme_edit_switch_tests;
