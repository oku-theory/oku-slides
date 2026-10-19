//! Pourquoi une surface de l'agent est refusée (`SessionErrorKind::AgentForm`).
//!
//! Chaque raison se dit dans la langue de l'utilisateur (`session-agent-form-*`, Fluent) ;
//! `Display` reste l'anglais des journaux. Les arguments sont ceux que l'agent a envoyés
//! (identifiants, noms, valeurs), des indices et des bornes : jamais un chemin de l'hôte.
//! `at` situe la valeur fautive dans les arguments de l'outil (`fields[2].options`).

use std::fmt;

use oku_slides_core::TransactionError;
use oku_slides_i18n::{FluentValue, Localize, Localizer};

/// La raison d'un refus `AgentForm`, regroupée par nature plutôt que par appel.
#[derive(Debug)]
#[non_exhaustive]
pub enum AgentFormReason {
    /// Texte de `count` caractères pour `max` permis.
    TooLong { at: String, count: usize, max: usize },
    /// Contrôle invisible ou bidirectionnel dans un texte.
    HiddenCharacters { at: String },
    /// `count` entrées hors de `min..=max`.
    Count { at: String, min: usize, max: usize, count: usize },
    /// Plus de `max` entrées.
    TooMany { at: String, max: usize, count: usize },
    /// Plus de `max` octets.
    TooLarge { at: String, max: usize, size: usize },
    /// Valeur vide (titre blanc, aucun fait).
    Empty { at: String },
    /// Valeur absente.
    Missing { at: String },
    /// Identifiant donné deux fois.
    DuplicateId { at: String, id: String },
    /// Ni `#rgb`, ni `#rgba`, ni `#rrggbb`, ni `#rrggbbaa`.
    NotHexColor { at: String, value: String },
    /// Valeur d'un champ riche qui n'est pas une liste de paragraphes.
    NotParagraphs { at: String },
    /// Borne d'un curseur infinie ou NaN.
    NotFinite { at: String },
    /// `min` n'est pas inférieur à `max`.
    BoundsInverted { at: String, min: f64, max: f64 },
    /// JSON d'une autre forme que celle attendue ; `detail` : le message de serde.
    Malformed { at: String, detail: String },
    /// Opérations refusées par le deck.
    Refused { at: String, source: Box<TransactionError> },
    /// Une variante supprime la slide qu'elle devait montrer.
    SlideRemoved { at: String, slide: String },
    /// `name` n'existe pas dans l'outil `tool`.
    NotAvailable { at: String, name: String, tool: &'static str },
    /// Un champ de réglage dans `render` : c'est le rôle de `show_params`.
    SettingInRender { at: String, name: String },
    /// Composant réservé à l'hôte.
    Reserved { at: String, name: String },
    /// Styles `primary` et `danger`, réservés à l'hôte.
    ReservedStyle { at: String },
    /// Liaison `{path}` au lieu d'une valeur littérale.
    NotLiteral { at: String },
    /// Propriété inconnue du composant.
    UnknownProperty { at: String, name: String },
    /// Action ailleurs que sur un bouton, autre que `suggest`, ou dont la consigne n'est pas le libellé.
    BadAction { at: String },
    /// Aucun composant `root`.
    NoRoot,
    /// Arbre plus profond que `max` niveaux (`id` : un composant au niveau `level`).
    TooDeep { max: usize, level: usize, id: String },
    /// Composant visité deux fois : partagé ou dans un cycle.
    SharedComponent { id: String },
    /// Enfant qui ne désigne aucun composant.
    UnknownChild { id: String },
    /// Composant hors de l'arbre de `root`.
    Unreachable { id: String },
    /// La session de l'onglet n'a pas été appelée (faute de l'hôte).
    SessionNotCalled { tab: String },
}

impl fmt::Display for AgentFormReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AgentFormReason::*;
        match self {
            TooLong { at, count, max } => write!(f, "{at}: {count} characters, at most {max}"),
            HiddenCharacters { at } => {
                write!(f, "{at}: invisible or bidirectional control characters are not allowed")
            }
            Count { at, min, max, count } => write!(f, "{at}: expected {min} to {max} entries, got {count}"),
            TooMany { at, max, count } => write!(f, "{at}: at most {max} entries, got {count}"),
            TooLarge { at, max, size } => write!(f, "{at}: at most {max} bytes, got {size}"),
            Empty { at } => write!(f, "{at}: must not be empty"),
            Missing { at } => write!(f, "{at}: missing"),
            DuplicateId { at, id } => write!(f, "{at}: duplicate id {id:?}"),
            NotHexColor { at, value } => write!(f, "{at}: {value} is not a hex color"),
            NotParagraphs { at } => write!(f, "{at}: a rich value must be a list of paragraphs"),
            NotFinite { at } => write!(f, "{at}: min/max must be finite"),
            BoundsInverted { at, min, max } => write!(f, "{at}: min ({min}) must be < max ({max})"),
            Malformed { at, detail } => write!(f, "{at}: {detail}"),
            Refused { at, source } => write!(f, "{at}: {source}: {}", source.op_error()),
            SlideRemoved { at, slide } => write!(f, "{at}: slide {slide} removed"),
            NotAvailable { at, name, tool } => write!(f, "{at}: {name} is not available in {tool}"),
            SettingInRender { at, name } => {
                write!(f, "{at}: {name} is not available in render (use show_params for settings)")
            }
            Reserved { at, name } => write!(f, "{at}: {name} is reserved to Oku Slides"),
            ReservedStyle { at } => write!(f, "{at}: primary/danger/icon styles are reserved to Oku Slides"),
            NotLiteral { at } => write!(f, "{at}: values must be literal (no {{path}} bindings)"),
            UnknownProperty { at, name } => write!(f, "{at}: unknown property {name:?}"),
            BadAction { at } => {
                write!(f, "{at}: only a button may carry an action: suggest, with its own label as literal prompt")
            }
            NoRoot => write!(f, "no component with id \"root\""),
            TooDeep { max, level, id } => write!(f, "deeper than {max} levels (level {level}, at {id:?})"),
            SharedComponent { id } => {
                write!(f, "component {id:?} is used more than once (the components must form a tree)")
            }
            UnknownChild { id } => write!(f, "unknown child {id:?}"),
            Unreachable { id } => write!(f, "component {id:?} is not reachable from \"root\""),
            SessionNotCalled { tab } => write!(f, "{tab}: session not called"),
        }
    }
}

impl Localize for AgentFormReason {
    fn localize(&self, l: &Localizer) -> String {
        use AgentFormReason::*;
        let s = |v: &str| FluentValue::from(v.to_owned());
        let n = |v: usize| FluentValue::from(v);
        let (key, args): (&str, Vec<(&str, FluentValue<'_>)>) = match self {
            TooLong { at, count, max } => ("too-long", vec![("at", s(at)), ("count", n(*count)), ("max", n(*max))]),
            HiddenCharacters { at } => ("hidden-characters", vec![("at", s(at))]),
            Count { at, min, max, count } => {
                ("count", vec![("at", s(at)), ("min", n(*min)), ("max", n(*max)), ("count", n(*count))])
            }
            TooMany { at, max, count } => ("too-many", vec![("at", s(at)), ("max", n(*max)), ("count", n(*count))]),
            TooLarge { at, max, size } => ("too-large", vec![("at", s(at)), ("max", n(*max)), ("size", n(*size))]),
            Empty { at } => ("empty", vec![("at", s(at))]),
            Missing { at } => ("missing", vec![("at", s(at))]),
            DuplicateId { at, id } => ("duplicate-id", vec![("at", s(at)), ("id", s(id))]),
            NotHexColor { at, value } => ("not-hex-color", vec![("at", s(at)), ("value", s(value))]),
            NotParagraphs { at } => ("not-paragraphs", vec![("at", s(at))]),
            NotFinite { at } => ("not-finite", vec![("at", s(at))]),
            BoundsInverted { at, min, max } => {
                ("bounds-inverted", vec![("at", s(at)), ("min", (*min).into()), ("max", (*max).into())])
            }
            Malformed { at, detail } => ("malformed", vec![("at", s(at)), ("detail", s(detail))]),
            Refused { at, source } => ("refused", vec![("at", s(at)), ("reason", source.localize(l).into())]),
            SlideRemoved { at, slide } => ("slide-removed", vec![("at", s(at)), ("slide", s(slide))]),
            NotAvailable { at, name, tool } => {
                ("not-available", vec![("at", s(at)), ("name", s(name)), ("tool", s(tool))])
            }
            SettingInRender { at, name } => ("setting-in-render", vec![("at", s(at)), ("name", s(name))]),
            Reserved { at, name } => ("reserved", vec![("at", s(at)), ("name", s(name))]),
            ReservedStyle { at } => ("reserved-style", vec![("at", s(at))]),
            NotLiteral { at } => ("not-literal", vec![("at", s(at))]),
            UnknownProperty { at, name } => ("unknown-property", vec![("at", s(at)), ("name", s(name))]),
            BadAction { at } => ("bad-action", vec![("at", s(at))]),
            NoRoot => ("no-root", vec![]),
            TooDeep { max, level, id } => ("too-deep", vec![("max", n(*max)), ("level", n(*level)), ("id", s(id))]),
            SharedComponent { id } => ("shared-component", vec![("id", s(id))]),
            UnknownChild { id } => ("unknown-child", vec![("id", s(id))]),
            Unreachable { id } => ("unreachable", vec![("id", s(id))]),
            SessionNotCalled { tab } => ("session-not-called", vec![("tab", s(tab))]),
        };
        l.tr_with(&format!("session-agent-form-{key}"), &args)
    }
}

#[cfg(test)]
#[path = "reason_tests.rs"]
mod reason_tests;
