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

use crate::error::{
    DeckOnlySnafu, NamespaceTakenSnafu, NotPinnedSnafu, OpSnafu, PackAlreadyCurrentSnafu, PackNotMakeCurrentSnafu,
    ThemeNotOutdatedSnafu, UnknownNamespaceSnafu, UnknownPackSnafu,
};
use crate::pack::{PackManifest, Relation};
use crate::pack_receive::is_deck_only;
use crate::theme::default_namespace;
use crate::theme_schema::token_paths;
use crate::{PackLibrary, Result, Session};

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
    /// La vue du panneau des packs ; refusée hors du mode deck.
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

    /// Ajoute au deck la version courante d'un pack de la bibliothèque.
    ///
    /// Ses besoins absents entrent dans la même étape (`Session::apply`).
    /// Refusé pour un espace de noms déjà épinglé (sous l'autre sorte, par le
    /// cœur), ou que la bibliothèque ne donne pas à un pack de cette sorte.
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

    /// Retire un pack du deck, en une étape d'annulation.
    ///
    /// Le cœur refuse ce que `remove_pack_refusal` refuse : rien ne change alors.
    pub fn remove_deck_pack(&mut self, kind: PackKind, ns: &Namespace) -> Result<()> {
        self.ensure_deck_mode()?;
        let name = self.deck().pins(kind).get(ns).map_or_else(|| ns.to_string(), |pin| self.pack_name(ns, pin));
        let label = self.localizer.tr_with("packs-undo-remove", &[("name", name.as_str().into())]);
        self.apply(&label, vec![Op::RemovePack { kind, namespace: ns.clone() }], None)
    }

    /// Le deck passe à ma version courante d'un pack.
    ///
    /// Une étape d'annulation. Refusé pour un pack que le deck n'épingle pas,
    /// du deck seul, ou déjà dans ma version courante (`PackAlreadyCurrent`).
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

    /// La version du deck devient ma version courante ; le deck ne change pas.
    ///
    /// La version vient de l'épinglage du deck seul. Refusé (`PackNotMakeCurrent`)
    /// pour un pack intégré, du deck seul, ou dont la bibliothèque n'a pas la version.
    pub fn pack_make_current(&mut self, kind: PackKind, ns: &Namespace) -> Result<()> {
        self.ensure_deck_mode()?;
        let pin = self.deck().pins(kind).get(ns).cloned().context(NotPinnedSnafu { namespace: ns.as_str() })?;
        let allowed = !PackLibrary::is_builtin(pin.id)
            && !self.embeds_deck_only(&pin)
            && self.library.version(pin.id, &pin.version).is_some();
        ensure!(allowed, PackNotMakeCurrentSnafu { pack: ns.as_str() });
        self.library.set_current(pin.id, &pin.version)?;
        self.pack_cache.clear();
        self.seed_cache.clear();
        self.refresh_pack_updates();
        Ok(())
    }

    /// Le deck passe à ma version courante de son thème.
    ///
    /// Une étape d'annulation. Refusé (`ThemeNotOutdated`) si la version
    /// épinglée n'est pas plus ancienne que la courante, ou sans thème épinglé.
    pub fn update_deck_theme(&mut self) -> Result<()> {
        self.ensure_deck_mode()?;
        let ns = self.active_theme()?;
        let outdated = self.deck().theme.is_some() && self.theme_relation() == Some(Drift::Older);
        ensure!(outdated, ThemeNotOutdatedSnafu { theme: ns.as_str() });
        let name = self.themes.by_namespace(&ns).map_or_else(|| ns.to_string(), |s| s.manifest.name);
        let label = self.localizer.tr_with("packs-undo-theme-update", &[("name", name.as_str().into())]);
        self.apply(&label, vec![Op::SetTheme { theme: Some(ns), themes: BTreeMap::new() }], None)
    }

    /// Les épinglages plus anciens que la bibliothèque (cache).
    pub(crate) fn pack_updates(&self) -> u32 {
        self.pack_updates
    }

    /// Recalcule le nombre d'épinglages en retard.
    ///
    /// Seul l'écart `Older` compte (le deck est plus ancien) : une
    /// version plus récente ou parallèle se lit dans le panneau, sans pastille.
    pub(crate) fn refresh_pack_updates(&mut self) {
        self.pack_updates_pins = Some(self.pins_key());
        let older = |r: Option<Drift>| r == Some(Drift::Older);
        let packs = self.user_deck().all_pins().filter(|(_, _, pin)| older(self.pin_relation(pin))).count();
        let count = packs + usize::from(older(self.theme_relation()));
        // Saturé : plus de 2³² épinglages n'existe pas, la pastille reste allumée.
        self.pack_updates = u32::try_from(count).unwrap_or(u32::MAX);
    }

    /// Recalcule le nombre d'épinglages en retard si les épinglages ont changé depuis le dernier calcul.
    ///
    /// Le nombre ne dépend que des épinglages et de la bibliothèque : une
    /// étape (annuler, rétablir, op, restauration) qui ne change aucun
    /// épinglage ne relit pas la bibliothèque, dont la lecture est bloquante
    /// (t-1112 : elle paniquait sur un fil qui fait tourner un runtime Tokio).
    pub(crate) fn refresh_pack_updates_if_pins_changed(&mut self) {
        if self.pack_updates_pins.as_ref() != Some(&self.pins_key()) {
            self.refresh_pack_updates();
        }
    }

    /// Les épinglages du deck de l'utilisateur dont dépend `pack_updates` : packs, puis thème.
    fn pins_key(&self) -> PinsKey {
        let packs = self.user_deck().all_pins().map(|(kind, ns, pin)| (kind, ns.clone(), pin.clone())).collect();
        (packs, self.deck_theme_pin().cloned())
    }

    /// La version épinglée `pin` par rapport à ma version courante de ce pack.
    ///
    /// `None` quand c'est elle, pour un pack du deck seul ou inconnu de la bibliothèque.
    fn pin_relation(&self, pin: &PackPin) -> Option<Drift> {
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
            name: named(&active),
            active,
            extends,
            overrides,
            choices: self.theme_list.iter().map(|t| (t.namespace.clone(), t.name.clone())).collect(),
            outdated: self.theme_relation() == Some(Drift::Older),
        })
    }
}

/// Les épinglages d'un deck (sorte, espace de noms, épinglage) et celui de son thème.
pub(crate) type PinsKey = (Vec<(PackKind, Namespace, PackPin)>, Option<oku_slides_core::ThemePin>);

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
