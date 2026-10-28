//! Les packs du deck et leurs actions, pour le panneau `host-packs`.
//!
//! La vue ([`DeckPacksView`]) ne porte aucun texte traduit : les refus de
//! retrait restent des `OpError` (ceux de `remove_pack_refusal`, que l'op
//! applique aussi), les versions des `Relation` ; le hub la traduit
//! (`deck_packs_panel`). Chaque action se refuse hors du mode deck et revérifie
//! ce que le panneau propose : un contexte forgé est refusé, jamais cru.
//! Les changements du deck sont une étape d'annulation chacun ; faire de la
//! version du deck ma version courante ne change que la bibliothèque.
//!
//! `pack_updates` compte les épinglages (packs et thème) plus anciens que la
//! bibliothèque ; gardé sur la session, il est recalculé quand les épinglages
//! ou la bibliothèque changent, jamais à chaque instantané.

use std::collections::BTreeMap;

use oku_slides_core::{Namespace, Op, OpError, PackKind, PackPin, remove_pack_refusal};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use crate::agent::bounds::label;
use crate::error::{
    BuiltinPackSnafu, DeckOnlySnafu, InPackTabSnafu, NamespaceTakenSnafu, NotPinnedSnafu, OpSnafu,
    PackAlreadyCurrentSnafu, PackNotMakeCurrentSnafu, PackOpenInTabSnafu, ThemeNotOutdatedSnafu, UnknownNamespaceSnafu,
    UnknownPackSnafu,
};
use crate::library::draft_lock;
use crate::library_index::library_writes;
use crate::pack::{PackInfo, PackManifest, Relation};
use crate::pack_lease::pack_leased;
use crate::pack_receive::is_deck_only;
use crate::theme::default_namespace;
use crate::theme_schema::token_paths;
use crate::{Mode, PackLibrary, Result, Session};

/// Les packs du deck et ce qu'on peut en faire, sans texte traduit.
#[derive(Debug)]
pub struct DeckPacksView {
    /// Les packs de layouts épinglés, par espace de noms.
    pub layouts: Vec<PinnedPackView>,
    /// Les packs de widgets épinglés, par espace de noms.
    pub widgets: Vec<PinnedPackView>,
    /// Par sorte, les packs de la bibliothèque que le deck peut ajouter : espace de noms et nom.
    pub addable: BTreeMap<PackKind, Vec<(Namespace, String)>>,
    /// Le thème du deck.
    pub theme: ThemeSectionView,
}

/// Une version épinglée par rapport à ma version courante, quand ce n'est pas elle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    /// Plus ancienne : ma version courante en descend.
    Older,
    /// Plus récente : elle descend de ma version courante.
    Newer,
    /// Parallèle : aucune ne descend de l'autre.
    Parallel,
}

impl Drift {
    /// L'écart que dit `relation` ; `None` pour la même version.
    fn of(relation: Relation) -> Option<Self> {
        match relation {
            Relation::Same => None,
            Relation::Ancestor => Some(Self::Older),
            Relation::Descendant => Some(Self::Newer),
            Relation::Sibling => Some(Self::Parallel),
        }
    }
}

/// Un pack épinglé : identité, relation à la bibliothèque, refus de retrait.
#[derive(Debug)]
pub struct PinnedPackView {
    /// La sorte du pack.
    pub kind: PackKind,
    /// Son espace de noms dans le deck.
    pub namespace: Namespace,
    /// Son nom, lu dans la version embarquée.
    pub name: String,
    /// La version épinglée, en hexadécimal.
    pub version: String,
    /// La version du deck par rapport à ma version courante ; `None` si c'est elle ou sans bibliothèque.
    pub drift: Option<Drift>,
    /// Un pack intégré à l'app.
    pub builtin: bool,
    /// Un pack du deck seul, que la bibliothèque n'a jamais.
    pub deck_only: bool,
    /// La bibliothèque a la version épinglée.
    pub library_has_pinned: bool,
    /// Pourquoi le pack ne peut pas quitter le deck ; `None` s'il le peut.
    pub remove_refusal: Option<OpError>,
}

/// Le thème du deck dans le panneau.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSectionView {
    /// The id of the deck's theme pin; `None` without one (no "Open" button).
    pub id: Option<Uuid>,
    /// L'espace de noms du thème avec lequel le deck se rend.
    pub active: Namespace,
    /// Son nom.
    pub name: String,
    /// Les noms des thèmes qu'il étend, dans l'ordre.
    pub extends: Vec<String>,
    /// Le nombre de jetons que le deck surcharge.
    pub overrides: usize,
    /// Les thèmes de la bibliothèque : espace de noms et nom.
    pub choices: Vec<(Namespace, String)>,
    /// La version épinglée est plus ancienne que ma version courante.
    pub outdated: bool,
}

impl Session {
    /// The view of the packs panel; rejected outside deck mode.
    ///
    /// # Errors
    ///
    /// Fails outside deck mode (help, pack, history).
    pub fn deck_packs(&self) -> Result<DeckPacksView> {
        self.ensure_deck_mode()?;
        let deck = self.deck();
        let pinned = |kind: PackKind| -> Vec<PinnedPackView> {
            deck.pins(kind).iter().map(|(ns, pin)| self.pinned_view(kind, ns, pin)).collect()
        };
        let addable = PackKind::ALL.into_iter().map(|kind| (kind, self.addable(kind))).collect();
        Ok(DeckPacksView {
            layouts: pinned(PackKind::Layouts),
            widgets: pinned(PackKind::Widgets),
            addable,
            theme: self.theme_section()?,
        })
    }

    /// Adds the current version of a library pack to the deck.
    ///
    /// Its missing requirements join the same step (`Session::apply`).
    /// Rejected for a namespace already pinned (under the other kind, by the
    /// core), or that the library does not give to a pack of this kind.
    ///
    /// # Errors
    ///
    /// Fails outside deck mode, for a namespace already taken (`NamespaceTaken`) or unknown to the
    /// library, or if the transaction is rejected.
    pub fn add_library_pack(&mut self, kind: PackKind, ns: &Namespace) -> Result<()> {
        self.ensure_deck_mode()?;
        let deck = self.deck();
        if let Some(pin) = deck.pins(kind).get(ns) {
            return NamespaceTakenSnafu { namespace: ns.as_str(), owner: pin.id.to_string() }.fail()?;
        }
        if let Some(pin) = deck.pins(other(kind)).get(ns) {
            // Le cœur dit pourquoi (`PackKindConflict`) ; sur un brouillon, le deck ne change pas.
            let op = Op::AddPack { kind, namespace: ns.clone(), pin: pin.clone() };
            oku_slides_core::apply_all(&mut deck.clone(), vec![op], &self.registry).context(OpSnafu)?;
            return NamespaceTakenSnafu { namespace: ns.as_str(), owner: pin.id.to_string() }.fail()?;
        }
        let stored = self.library.owned_current(kind, ns).context(UnknownNamespaceSnafu { namespace: ns.as_str() })?;
        ensure!(stored.manifest.scope.is_library(), DeckOnlySnafu { id: stored.manifest.id });
        let pin = PackPin { id: stored.manifest.id, version: stored.version, requires: stored.manifest.requires };
        let label = self.localizer.tr_with("packs-undo-add", &[("name", stored.manifest.name.as_str().into())]);
        self.apply(&label, vec![Op::AddPack { kind, namespace: ns.clone(), pin }], None)
    }

    /// Removes a pack from the deck, in one undo step.
    ///
    /// The core rejects what `remove_pack_refusal` rejects: nothing changes then.
    ///
    /// # Errors
    ///
    /// Fails outside deck mode, or if the core rejects the removal.
    pub fn remove_deck_pack(&mut self, kind: PackKind, ns: &Namespace) -> Result<()> {
        self.ensure_deck_mode()?;
        self.deck().pins(kind).get(ns).map_or(Ok(()), |pin| self.refuse_linked(pin.id))?;
        let name = self.deck().pins(kind).get(ns).map_or_else(|| ns.to_string(), |pin| self.pack_name(ns, pin));
        let label = self.localizer.tr_with("packs-undo-remove", &[("name", name.as_str().into())]);
        self.apply(&label, vec![Op::RemovePack { kind, namespace: ns.clone() }], None)
    }

    /// The deck moves to the user's current version of a pack.
    ///
    /// One undo step. Rejected for a pack the deck does not pin, a deck-only
    /// pack, or one already at the user's current version (`PackAlreadyCurrent`).
    ///
    /// # Errors
    ///
    /// Fails outside deck mode, for an unpinned pack (`NotPinned`), a deck-only one (`DeckOnly`), an
    /// unknown one (`UnknownPack`) or one already current (`PackAlreadyCurrent`).
    pub fn pack_use_mine(&mut self, kind: PackKind, ns: &Namespace) -> Result<()> {
        self.ensure_deck_mode()?;
        let pin = self.deck().pins(kind).get(ns).cloned().context(NotPinnedSnafu { namespace: ns.as_str() })?;
        ensure!(!self.embeds_deck_only(&pin), DeckOnlySnafu { id: pin.id });
        let current = self.library.current_version(pin.id).context(UnknownPackSnafu { id: pin.id })?;
        ensure!(current.version != pin.version, PackAlreadyCurrentSnafu { pack: ns.as_str() });
        let pin = PackPin { id: pin.id, version: current.version, requires: current.manifest.requires };
        let label = self.localizer.tr("packs-undo-use-mine");
        self.apply(&label, vec![Op::AddPack { kind, namespace: ns.clone(), pin }], None)
    }

    /// The deck's version becomes the user's current version; the deck does not change.
    ///
    /// The version comes from the deck's pin alone. Rejected (`PackNotMakeCurrent`)
    /// for a built-in pack, a deck-only pack, or one whose version the library lacks.
    ///
    /// # Errors
    ///
    /// Fails outside deck mode, for an unpinned pack (`NotPinned`), a rejected one
    /// (`PackNotMakeCurrent`), one another tab edits (`PackOpenInTab`), or if the library does
    /// not write the current version.
    pub fn pack_make_current(&mut self, kind: PackKind, ns: &Namespace) -> Result<()> {
        self.ensure_deck_mode()?;
        let pin = self.deck().pins(kind).get(ns).cloned().context(NotPinnedSnafu { namespace: ns.as_str() })?;
        let allowed = !PackLibrary::is_builtin(pin.id)
            && !self.embeds_deck_only(&pin)
            && self.library.version(pin.id, &pin.version).is_some();
        ensure!(allowed, PackNotMakeCurrentSnafu { pack: ns.as_str() });
        let name = label(&self.pack_name(ns, &pin));
        // The lease check and the write under one `DRAFTS` scope: no editor leases it in between.
        let lock = draft_lock();
        let written = self.refuse_held(pin.id, &name).and_then(|()| self.library.set_current(pin.id, &pin.version));
        drop(lock);
        written?;
        self.pack_cache.clear();
        self.seed_cache.clear();
        self.refresh_pack_updates();
        Ok(())
    }

    /// The deck moves to the user's current version of its theme.
    ///
    /// One undo step. Rejected (`ThemeNotOutdated`) if the pinned version is
    /// not older than the current one, or without a pinned theme.
    ///
    /// # Errors
    ///
    /// Fails outside deck mode, without a readable active theme, if the theme is not outdated
    /// (`ThemeNotOutdated`), or if the transaction is rejected.
    pub fn update_deck_theme(&mut self) -> Result<()> {
        self.ensure_deck_mode()?;
        let ns = self.active_theme()?;
        let outdated = self.deck().theme.is_some() && self.theme_relation() == Some(Drift::Older);
        ensure!(outdated, ThemeNotOutdatedSnafu { theme: ns.as_str() });
        let name = self.themes.by_namespace(&ns).map_or_else(|| ns.to_string(), |s| s.manifest.name);
        let label = self.localizer.tr_with("packs-undo-theme-update", &[("name", name.as_str().into())]);
        self.apply(&label, vec![Op::SetTheme { theme: Some(ns), themes: BTreeMap::new() }], None)
    }

    /// Deletes the user's pack `pack`, every version included; the packs of its kind after.
    ///
    /// # Errors
    ///
    /// `BuiltinPack` for a built-in, `InPackTab` for the pack this tab edits,
    /// `PackOpenInTab` while another editor holds it, and the library's delete errors.
    ///
    /// # Blocking
    ///
    /// Reads and writes the library.
    pub(crate) fn delete_pack(&mut self, pack: Uuid) -> Result<Vec<PackInfo>> {
        ensure!(!PackLibrary::is_builtin(pack), BuiltinPackSnafu { id: pack.to_string() });
        // Leased or not, the edited pack never disappears under its own editor.
        ensure!(!matches!(&self.mode, Mode::Pack { edit, .. } if edit.pack == pack), InPackTabSnafu);
        let kind = self.library.kind_of(pack);
        // The name is read before `DRAFTS`: no version load under it.
        let name = self.library.current_version(pack).map_or_else(|| pack.to_string(), |s| label(&s.manifest.name));
        let lock = draft_lock();
        let deleted = self.refuse_held(pack, &name).and_then(|()| self.library.delete(pack));
        drop(lock);
        deleted?;
        self.pack_cache.clear();
        self.seed_cache.clear();
        self.refresh_pack_updates();
        Ok(self.library.packs(kind))
    }

    /// Refuses a library write of `pack` while an editor holds its lease (`PackOpenInTab`).
    ///
    /// The caller holds `DRAFTS`, so no lease is taken before its write.
    fn refuse_held(&self, pack: Uuid, name: &str) -> Result<()> {
        match self.library.lease_scope().and_then(|scope| pack_leased(scope, pack)) {
            Some(tab) => PackOpenInTabSnafu { pack: name, tab }.fail()?,
            None => Ok(()),
        }
    }

    /// Les épinglages plus anciens que la bibliothèque (cache).
    pub(crate) fn pack_updates(&self) -> u32 {
        self.pack_updates
    }

    /// Recounts the pins older than the library.
    ///
    /// Only the `Older` drift counts (the deck is older): a newer or parallel
    /// version shows in the panel, without a badge. The deck adopts its pins
    /// first; the library write count is then read before the library: a
    /// write landing during the count is seen next time.
    pub(crate) fn refresh_pack_updates(&mut self) {
        // Les épinglages ont changé : le deck adopte ce qu'il épingle avant qu'on le compare.
        self.adopt_pins();
        self.recount_pack_updates();
    }

    /// Counts the pins older than the library, keeping the key it was counted for; adopts nothing.
    fn recount_pack_updates(&mut self) {
        self.pack_updates_pins = Some(self.pins_key());
        let older = |r: Option<Drift>| r == Some(Drift::Older);
        let packs = self.user_deck().all_pins().filter(|(_, _, pin)| older(self.pin_relation(pin))).count();
        let count = packs + usize::from(older(self.theme_relation()));
        // Saturé : plus de 2³² épinglages n'existe pas, la pastille reste allumée.
        self.pack_updates = u32::try_from(count).unwrap_or(u32::MAX);
    }

    /// Recounts the pins older than the library if the pins or the library changed since.
    ///
    /// The count depends only on the pins and the library: a step that
    /// changes neither does not read the library, whose read blocks (t-1112:
    /// it panicked on a thread running a Tokio runtime; every caller runs on
    /// blocking code). Changed pins adopt first (`refresh_pack_updates`); a
    /// library write of this process alone only recounts, without `DRAFTS`.
    pub(crate) fn refresh_pack_updates_if_pins_changed(&mut self) {
        let key = self.pins_key();
        match &self.pack_updates_pins {
            Some(seen) if *seen == key => {}
            Some((packs, theme, _)) if *packs == key.0 && *theme == key.1 => self.recount_pack_updates(),
            _ => self.refresh_pack_updates(),
        }
    }

    /// The pins of the user's deck `pack_updates` depends on: packs, theme, and the library writes seen.
    fn pins_key(&self) -> PinsKey {
        let packs = self.user_deck().all_pins().map(|(kind, ns, pin)| (kind, ns.clone(), pin.clone())).collect();
        (packs, self.deck_theme_pin().cloned(), library_writes())
    }

    /// La version épinglée `pin` par rapport à ma version courante de ce pack.
    ///
    /// `None` quand c'est elle, pour un pack du deck seul ou inconnu de la bibliothèque.
    pub(crate) fn pin_relation(&self, pin: &PackPin) -> Option<Drift> {
        if self.embeds_deck_only(pin) {
            return None;
        }
        let current = self.library.current(pin.id)?;
        Drift::of(self.library.relation(pin.id, &pin.version, &current))
    }

    /// La version du thème du deck par rapport à ma version courante de ce thème (voir [`Self::pin_relation`]).
    fn theme_relation(&self) -> Option<Drift> {
        let pin = self.deck_theme_pin()?;
        let current = self.themes.current(pin.id)?;
        Drift::of(self.themes.relation(pin.id, &pin.version, &current))
    }

    /// Vrai si la version épinglée `pin` est embarquée en version du deck seul.
    fn embeds_deck_only(&self, pin: &PackPin) -> bool {
        self.user_blobs().packs.get(&pin.version).is_some_and(is_deck_only)
    }

    /// Le nom du pack `pin` épinglé sous `ns`, lu dans sa version embarquée.
    ///
    /// Une version absente ou illisible (déjà signalée au rendu) se nomme par son espace de noms.
    fn pack_name(&self, ns: &Namespace, pin: &PackPin) -> String {
        let files = self.user_blobs().packs.get(&pin.version);
        match files.map(PackManifest::from_files) {
            Some(Ok(m)) => m.name,
            Some(Err(e)) => {
                tracing::warn!(pack = %ns, error = %e, "manifeste épinglé illisible : nommé par son espace de noms");
                ns.to_string()
            }
            None => {
                tracing::warn!(pack = %ns, "version épinglée non embarquée : nommée par son espace de noms");
                ns.to_string()
            }
        }
    }

    /// Le pack `pin` de sorte `kind` épinglé sous `ns`, dans la vue.
    fn pinned_view(&self, kind: PackKind, ns: &Namespace, pin: &PackPin) -> PinnedPackView {
        let deck_only = self.embeds_deck_only(pin);
        PinnedPackView {
            kind,
            namespace: ns.clone(),
            name: self.pack_name(ns, pin),
            version: pin.version.hex().to_owned(),
            drift: self.pin_relation(pin),
            builtin: PackLibrary::is_builtin(pin.id),
            deck_only,
            library_has_pinned: !deck_only && self.library.version(pin.id, &pin.version).is_some(),
            remove_refusal: remove_pack_refusal(self.deck(), kind, ns),
        }
    }

    /// Les packs de sorte `kind` que le deck peut ajouter : espace de noms épinglé
    /// sous aucune sorte, détenu selon R15, de la bibliothèque.
    fn addable(&self, kind: PackKind) -> Vec<(Namespace, String)> {
        let deck = self.deck();
        let pinned = |ns: &Namespace| deck.widgets.contains_key(ns) || deck.layouts.contains_key(ns);
        self.library
            .entries(kind)
            .into_iter()
            .filter(|s| PackLibrary::owns(s, &s.manifest.namespace) && s.manifest.scope.is_library())
            .filter(|s| !pinned(&s.manifest.namespace))
            .map(|s| (s.manifest.namespace, s.manifest.name))
            .collect()
    }

    /// La section du thème : actif, parents, surcharges, choix, retard.
    fn theme_section(&self) -> Result<ThemeSectionView> {
        let deck = self.deck();
        let active = deck.theme.clone().map_or_else(default_namespace, Ok)?;
        let named = |ns: &Namespace| {
            let embedded = deck.themes.get(ns).and_then(|pin| self.theme_manifest(pin)).map(|m| m.name);
            embedded.or_else(|| self.themes.by_namespace(ns).map(|s| s.manifest.name)).unwrap_or_else(|| {
                tracing::warn!(theme = %ns, "thème introuvable : nommé par son espace de noms");
                ns.to_string()
            })
        };
        let extends = deck.themes.get(&active).map(|pin| pin.extends.iter().map(named).collect()).unwrap_or_default();
        let overrides = deck.tokens.as_object().map_or(0, |t| token_paths(&self.token_schema(deck), t).len());
        Ok(ThemeSectionView {
            id: self.deck_theme_pin().map(|pin| pin.id),
            name: named(&active),
            active,
            extends,
            overrides,
            choices: self.theme_list.iter().map(|t| (t.namespace.clone(), t.name.clone())).collect(),
            outdated: self.theme_relation() == Some(Drift::Older),
        })
    }
}

/// A deck's pins (kind, namespace, pin), its theme's, and the library writes of this process.
pub(crate) type PinsKey = (Vec<(PackKind, Namespace, PackPin)>, Option<oku_slides_core::ThemePin>, u64);

/// L'autre sorte que `kind`.
fn other(kind: PackKind) -> PackKind {
    match kind {
        PackKind::Widgets => PackKind::Layouts,
        PackKind::Layouts => PackKind::Widgets,
    }
}

#[cfg(test)]
#[path = "deck_packs_tests.rs"]
pub(crate) mod deck_packs_tests;

#[cfg(test)]
#[path = "deck_packs_updates_tests.rs"]
mod deck_packs_updates_tests;
