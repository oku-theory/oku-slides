//! Messages A2UI v0.9 du serveur et composants du catalogue d'Oku Slides.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Catalogue des surfaces d'Oku Slides (panneau et agent), implémenté par l'interface (`ui/src/a2ui`).
pub const CATALOG_ID: &str = "urn:oku-slides:a2ui:catalog:v3";

/// Version du protocole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Version {
    #[serde(rename = "v0.9")]
    V0_9,
}

/// Un message du serveur vers l'interface.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServerMessage {
    pub version: Version,
    #[serde(flatten)]
    pub body: ServerBody,
}

/// Contenu d'un [`ServerMessage`] : une seule clé, le nom du message.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerBody {
    CreateSurface(CreateSurface),
    UpdateComponents(UpdateComponents),
    UpdateDataModel(UpdateDataModel),
    DeleteSurface(DeleteSurface),
}

/// Crée une surface vide, rendue avec un catalogue.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSurface {
    pub surface_id: String,
    pub catalog_id: String,
}

/// Ajoute ou remplace des composants (par identifiant).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateComponents {
    pub surface_id: String,
    pub components: Vec<Component>,
}

/// Écrit `value` au chemin `path` du modèle de données (la racine sans
/// chemin) ; sans valeur, efface ce qui s'y trouve.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataModel {
    pub surface_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

/// Retire une surface.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSurface {
    pub surface_id: String,
}

/// Un composant : son identifiant et ce qu'il est. `root` est la racine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    #[serde(flatten)]
    pub kind: ComponentKind,
}

/// Composants du catalogue [`CATALOG_ID`]. Noms et propriétés reprennent
/// ceux du catalogue de base v0.9 ; `Stepper`, `ChoiceChips`, `ColorField`,
/// `SlideThumb`, `DiffText` sont propres à Oku Slides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "component")]
pub enum ComponentKind {
    /// Enfants empilés verticalement.
    Column { children: Vec<String> },
    /// Une carte autour d'un seul enfant ; `tone` la teinte.
    Card {
        child: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<Tone>,
    },
    /// Un texte.
    Text {
        text: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        variant: Option<TextVariant>,
    },
    /// Un champ de texte lié au modèle ; `action` part quand la saisie est
    /// validée (sortie du champ ou Entrée).
    TextField {
        label: Dynamic<String>,
        value: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Un compteur − valeur + lié au modèle ; `action` part à chaque pas.
    Stepper {
        label: Dynamic<String>,
        value: Dynamic<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Enfants côte à côte.
    Row { children: Vec<String> },
    /// Un bouton ; `primary` : l'action principale de la surface ; `danger` :
    /// une action qui engage (non annulable, permission permanente).
    Button {
        label: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        primary: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        danger: Option<bool>,
        action: Action,
    },
    /// Des pastilles, une par option ; choisir écrit l'`id` dans `value` puis envoie `action`.
    ChoiceChips {
        options: Vec<ChoiceOption>,
        value: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Un curseur ; `action` part quand on lâche.
    Slider {
        label: Dynamic<String>,
        value: Dynamic<f64>,
        min: f64,
        max: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Une couleur (hex), choisie dans `palette` ou libre.
    ColorField {
        label: Dynamic<String>,
        value: Dynamic<String>,
        palette: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Une liste déroulante.
    Select {
        label: Dynamic<String>,
        value: Dynamic<String>,
        options: Vec<ChoiceOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Miniature d'une slide rendue par la session (SVG), cliquable.
    SlideThumb {
        svg: String,
        label: Dynamic<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Un texte et ses changements, mot à mot.
    DiffText { segments: Vec<DiffSegment> },
    /// La consigne de l'utilisateur (hôte seul).
    Prompt { text: Dynamic<String> },
    /// La progression d'un tour ; repliée en une ligne `summary` (hôte seul).
    Timeline { steps: Dynamic<Vec<TimelineStep>>, collapsed: Dynamic<bool>, summary: Dynamic<String> },
    /// La conclusion d'un tour : titre, 1 à 4 faits, miniature et couleur facultatives.
    Receipt {
        title: Dynamic<String>,
        facts: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thumb: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        swatch: Option<String>,
    },
    /// Un enfant replié derrière `summary`.
    Disclosure { summary: Dynamic<String>, child: String },
    /// Un compteur ou un statut.
    Badge {
        text: Dynamic<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<Tone>,
    },
    /// Du Markdown (rendu sans HTML brut).
    Markdown { text: Dynamic<String> },
}

/// Teinte d'une carte ou d'un badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    /// Sans teinte.
    Neutral,
    /// Teinte d'accent (reçus).
    Accent,
    /// Filet rouge : réservé à l'hôte dans `render`.
    Danger,
}

/// Statut d'une étape de la timeline d'un tour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Pas commencée.
    Pending,
    /// En cours.
    InProgress,
    /// Finie.
    Completed,
    /// Échouée : reste visible dans une étape d'outils groupés.
    Failed,
}

/// Une étape de la timeline : outils groupés, note de l'agent ou entrée du plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineStep {
    /// Titre du dernier outil de l'étape, note de l'agent ou entrée du plan.
    pub label: String,
    /// Statut agrégé de l'étape.
    pub status: StepStatus,
    /// Nombre d'outils de l'étape ; absent pour une note ou une entrée du plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

/// Style d'un texte (sous-ensemble des variantes v0.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextVariant {
    H4,
    Body,
    Caption,
    /// Une ligne atténuée, coupée.
    Note,
}

/// Une option d'un choix ou d'une liste. Aussi argument des outils de
/// l'agent (`ask_choice`, champs `select`) : désérialisable, avec schéma.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Un morceau de texte gardé, retiré ou ajouté.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffSegment {
    pub kind: DiffKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Keep,
    Del,
    Ins,
}

/// Une valeur écrite telle quelle, ou lue dans le modèle de données.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dynamic<T> {
    /// Chemin JSON Pointer (RFC 6901) dans le modèle de la surface.
    Path {
        path: String,
    },
    Literal(T),
}

impl<T> Dynamic<T> {
    pub fn path(path: impl Into<String>) -> Self {
        Self::Path { path: path.into() }
    }
}

/// Ce qu'un composant envoie quand l'utilisateur agit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub event: Event,
}

/// Un évènement nommé ; l'interface lit les chemins du contexte à l'envoi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, Dynamic<Value>>,
}

impl ComponentKind {
    /// Les identifiants des enfants, dans l'ordre.
    pub fn children(&self) -> Vec<&str> {
        // Sans `_` : un nouveau composant doit dire ici s'il a des enfants (la validation de `render` en dépend).
        match self {
            Self::Column { children } | Self::Row { children } => children.iter().map(String::as_str).collect(),
            Self::Card { child, .. } | Self::Disclosure { child, .. } => vec![child.as_str()],
            Self::Text { .. }
            | Self::TextField { .. }
            | Self::Stepper { .. }
            | Self::Button { .. }
            | Self::ChoiceChips { .. }
            | Self::Slider { .. }
            | Self::ColorField { .. }
            | Self::Select { .. }
            | Self::SlideThumb { .. }
            | Self::DiffText { .. }
            | Self::Prompt { .. }
            | Self::Timeline { .. }
            | Self::Receipt { .. }
            | Self::Badge { .. }
            | Self::Markdown { .. } => Vec::new(),
        }
    }

    /// Les actions que le composant peut envoyer.
    pub fn actions(&self) -> Vec<&Action> {
        match self {
            Self::Button { action, .. } => vec![action],
            Self::TextField { action, .. }
            | Self::Stepper { action, .. }
            | Self::ChoiceChips { action, .. }
            | Self::Slider { action, .. }
            | Self::ColorField { action, .. }
            | Self::Select { action, .. }
            | Self::SlideThumb { action, .. } => action.iter().collect(),
            // Sans `_` : un nouveau composant doit dire ici s'il envoie une action (la validation de `render` en dépend).
            Self::Column { .. }
            | Self::Row { .. }
            | Self::Card { .. }
            | Self::Text { .. }
            | Self::DiffText { .. }
            | Self::Prompt { .. }
            | Self::Timeline { .. }
            | Self::Receipt { .. }
            | Self::Disclosure { .. }
            | Self::Badge { .. }
            | Self::Markdown { .. } => Vec::new(),
        }
    }

    /// Le même composant, en lecture seule ; `None` pour un bouton (sans action, il n'a plus de sens).
    pub fn without_action(self) -> Option<Self> {
        Some(match self {
            Self::Button { .. } => return None,
            Self::TextField { label, value, .. } => Self::TextField { label, value, action: None },
            Self::Stepper { label, value, min, max, .. } => Self::Stepper { label, value, min, max, action: None },
            Self::ChoiceChips { options, value, .. } => Self::ChoiceChips { options, value, action: None },
            Self::Slider { label, value, min, max, step, .. } => {
                Self::Slider { label, value, min, max, step, action: None }
            }
            Self::ColorField { label, value, palette, .. } => Self::ColorField { label, value, palette, action: None },
            Self::Select { label, value, options, .. } => Self::Select { label, value, options, action: None },
            Self::SlideThumb { svg, label, .. } => Self::SlideThumb { svg, label, action: None },
            // Sans `_` : un nouveau composant doit dire ici comment il se fige.
            kind @ (Self::Column { .. }
            | Self::Row { .. }
            | Self::Card { .. }
            | Self::Text { .. }
            | Self::DiffText { .. }
            | Self::Prompt { .. }
            | Self::Timeline { .. }
            | Self::Receipt { .. }
            | Self::Disclosure { .. }
            | Self::Badge { .. }
            | Self::Markdown { .. }) => kind,
        })
    }
}

impl Component {
    /// `false` pour les composants réservés à l'hôte (`Prompt`, `Timeline`, `Receipt`, `SlideThumb`).
    ///
    /// Un reçu passe par l'outil `report` (bornes, miniature rendue par l'hôte) ; une miniature
    /// est toujours rendue par l'hôte.
    pub fn agent_allowed(&self) -> bool {
        !matches!(
            self.kind,
            ComponentKind::Prompt { .. }
                | ComponentKind::Timeline { .. }
                | ComponentKind::Receipt { .. }
                | ComponentKind::SlideThumb { .. }
        )
    }

    pub fn new(id: impl Into<String>, kind: ComponentKind) -> Self {
        Self { id: id.into(), kind }
    }

    /// Une carte autour de `child`.
    pub fn card(id: impl Into<String>, child: &str) -> Self {
        Self::new(id, ComponentKind::Card { child: child.to_owned(), tone: None })
    }

    pub fn column(id: impl Into<String>, children: &[&str]) -> Self {
        Self::new(id, ComponentKind::Column { children: children.iter().map(|c| (*c).to_owned()).collect() })
    }

    pub fn row(id: impl Into<String>, children: &[&str]) -> Self {
        Self::new(id, ComponentKind::Row { children: children.iter().map(|c| (*c).to_owned()).collect() })
    }

    pub fn text(id: impl Into<String>, text: impl Into<String>, variant: Option<TextVariant>) -> Self {
        Self::new(id, ComponentKind::Text { text: Dynamic::Literal(text.into()), variant })
    }

    pub fn button(id: impl Into<String>, label: impl Into<String>, primary: bool, action: Action) -> Self {
        Self::new(
            id,
            ComponentKind::Button {
                label: Dynamic::Literal(label.into()),
                primary: primary.then_some(true),
                danger: None,
                action,
            },
        )
    }

    /// Le même bouton, marqué ou non comme une action qui engage.
    ///
    /// Ne concerne que les boutons : un autre composant est rendu tel quel.
    #[must_use]
    pub fn danger(mut self, on: bool) -> Self {
        debug_assert!(matches!(self.kind, ComponentKind::Button { .. }), "danger() ne vaut que pour un bouton");
        if let ComponentKind::Button { danger, .. } = &mut self.kind {
            *danger = on.then_some(true);
        }
        self
    }
}

impl Action {
    /// L'évènement `name`, avec son contexte.
    pub fn event<'a>(name: impl Into<String>, context: impl IntoIterator<Item = (&'a str, Dynamic<Value>)>) -> Self {
        Self {
            event: Event { name: name.into(), context: context.into_iter().map(|(k, v)| (k.to_owned(), v)).collect() },
        }
    }

    /// L'évènement `name`, sans contexte.
    pub fn named(name: impl Into<String>) -> Self {
        Self { event: Event { name: name.into(), context: BTreeMap::new() } }
    }
}

impl ServerMessage {
    fn v0_9(body: ServerBody) -> Self {
        Self { version: Version::V0_9, body }
    }

    /// `createSurface` avec le catalogue d'Oku Slides.
    pub fn create_surface(surface_id: impl Into<String>) -> Self {
        Self::v0_9(ServerBody::CreateSurface(CreateSurface {
            surface_id: surface_id.into(),
            catalog_id: CATALOG_ID.to_owned(),
        }))
    }

    pub fn update_components(surface_id: impl Into<String>, components: Vec<Component>) -> Self {
        Self::v0_9(ServerBody::UpdateComponents(UpdateComponents { surface_id: surface_id.into(), components }))
    }

    /// Remplace tout le modèle de données.
    pub fn set_data_model(surface_id: impl Into<String>, value: Value) -> Self {
        Self::v0_9(ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: surface_id.into(),
            path: None,
            value: Some(value),
        }))
    }

    /// Écrit `value` au chemin `path` (JSON Pointer) du modèle de données.
    pub fn update_data_model(surface_id: impl Into<String>, path: &str, value: Value) -> Self {
        Self::v0_9(ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: surface_id.into(),
            path: Some(path.to_owned()),
            value: Some(value),
        }))
    }

    pub fn delete_surface(surface_id: impl Into<String>) -> Self {
        Self::v0_9(ServerBody::DeleteSurface(DeleteSurface { surface_id: surface_id.into() }))
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod tests;
