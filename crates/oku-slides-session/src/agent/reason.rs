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
    TooLong {
        /// Location of the faulty value in the arguments.
        at: String,
        /// Number of characters received.
        count: usize,
        /// Number of characters allowed.
        max: usize,
    },
    /// Contrôle invisible ou bidirectionnel dans un texte.
    HiddenCharacters {
        /// Location of the faulty text in the arguments.
        at: String,
    },
    /// `count` entrées hors de `min..=max`.
    Count {
        /// Location of the faulty list in the arguments.
        at: String,
        /// Minimum number of entries.
        min: usize,
        /// Maximum number of entries.
        max: usize,
        /// Number of entries received.
        count: usize,
    },
    /// Plus de `max` entrées.
    TooMany {
        /// Location of the faulty list in the arguments.
        at: String,
        /// Maximum number of entries.
        max: usize,
        /// Number of entries received.
        count: usize,
    },
    /// Plus de `max` octets.
    TooLarge {
        /// Location of the faulty value in the arguments.
        at: String,
        /// Maximum size in bytes.
        max: usize,
        /// Size received in bytes.
        size: usize,
    },
    /// Valeur vide (titre blanc, aucun fait).
    Empty {
        /// Location of the empty value in the arguments.
        at: String,
    },
    /// Valeur absente.
    Missing {
        /// Location where the value is missing in the arguments.
        at: String,
    },
    /// Identifiant donné deux fois.
    DuplicateId {
        /// Location of the duplicate in the arguments.
        at: String,
        /// Repeated id.
        id: String,
    },
    /// Ni `#rgb`, ni `#rgba`, ni `#rrggbb`, ni `#rrggbbaa`.
    NotHexColor {
        /// Location of the color in the arguments.
        at: String,
        /// Value received instead of a color.
        value: String,
    },
    /// Valeur d'un champ riche qui n'est pas une liste de paragraphes.
    NotParagraphs {
        /// Location of the value in the arguments.
        at: String,
    },
    /// Borne d'un curseur infinie ou NaN.
    NotFinite {
        /// Location of the bound in the arguments.
        at: String,
    },
    /// `min` n'est pas inférieur à `max`.
    BoundsInverted {
        /// Location of the slider in the arguments.
        at: String,
        /// Lower bound received.
        min: f64,
        /// Upper bound received.
        max: f64,
    },
    /// JSON d'une autre forme que celle attendue ; `detail` : le message de serde.
    Malformed {
        /// Location of the JSON in the arguments.
        at: String,
        /// Serde message describing the mismatch.
        detail: String,
    },
    /// Opérations refusées par le deck.
    Refused {
        /// Location of the operations in the arguments.
        at: String,
        /// Transaction error returned by the deck.
        source: Box<TransactionError>,
    },
    /// Une variante supprime la slide qu'elle devait montrer.
    SlideRemoved {
        /// Location of the variant in the arguments.
        at: String,
        /// Id of the deleted slide.
        slide: String,
    },
    /// `name` n'existe pas dans l'outil `tool`.
    NotAvailable {
        /// Location of the name in the arguments.
        at: String,
        /// Requested name, unknown to the tool.
        name: String,
        /// MCP tool that does not know it.
        tool: &'static str,
    },
    /// Un champ de réglage dans `render` : c'est le rôle de `show_params`.
    SettingInRender {
        /// Location of the component in `render`.
        at: String,
        /// Name of the settings component.
        name: String,
    },
    /// Composant réservé à l'hôte.
    Reserved {
        /// Location of the component in the arguments.
        at: String,
        /// Name of the reserved component.
        name: String,
    },
    /// Styles `primary` et `danger`, réservés à l'hôte.
    ReservedStyle {
        /// Location of the style in the arguments.
        at: String,
    },
    /// Liaison `{path}` au lieu d'une valeur littérale.
    NotLiteral {
        /// Location of the binding in the arguments.
        at: String,
    },
    /// Propriété inconnue du composant.
    UnknownProperty {
        /// Location of the property in the arguments.
        at: String,
        /// Name of the unknown property.
        name: String,
    },
    /// Action ailleurs que sur un bouton, autre que `suggest`, ou dont la consigne n'est pas le libellé.
    BadAction {
        /// Location of the action in the arguments.
        at: String,
    },
    /// Aucun composant `root`.
    NoRoot,
    /// Arbre plus profond que `max` niveaux (`id` : un composant au niveau `level`).
    TooDeep {
        /// Maximum depth allowed.
        max: usize,
        /// Level reached by component `id`.
        level: usize,
        /// Component nested too deep.
        id: String,
    },
    /// Composant visité deux fois : partagé ou dans un cycle.
    SharedComponent {
        /// Component visited twice.
        id: String,
    },
    /// Enfant qui ne désigne aucun composant.
    UnknownChild {
        /// Child id without a component.
        id: String,
    },
    /// Composant hors de l'arbre de `root`.
    Unreachable {
        /// Component that `root` does not reach.
        id: String,
    },
    /// La session de l'onglet n'a pas été appelée (faute de l'hôte).
    SessionNotCalled {
        /// Tab whose session is missing.
        tab: String,
    },
}

impl fmt::Display for AgentFormReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AgentFormReason::{
            BadAction, BoundsInverted, Count, DuplicateId, Empty, HiddenCharacters, Malformed, Missing, NoRoot,
            NotAvailable, NotFinite, NotHexColor, NotLiteral, NotParagraphs, Refused, Reserved, ReservedStyle,
            SessionNotCalled, SettingInRender, SharedComponent, SlideRemoved, TooDeep, TooLarge, TooLong, TooMany,
            UnknownChild, UnknownProperty, Unreachable,
        };
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
        use AgentFormReason::{
            BadAction, BoundsInverted, Count, DuplicateId, Empty, HiddenCharacters, Malformed, Missing, NoRoot,
            NotAvailable, NotFinite, NotHexColor, NotLiteral, NotParagraphs, Refused, Reserved, ReservedStyle,
            SessionNotCalled, SettingInRender, SharedComponent, SlideRemoved, TooDeep, TooLarge, TooLong, TooMany,
            UnknownChild, UnknownProperty, Unreachable,
        };
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
