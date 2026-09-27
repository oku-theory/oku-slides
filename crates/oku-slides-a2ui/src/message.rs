//! Messages A2UI v0.9 du serveur et composants du catalogue d'Oku Slides.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

/// Catalogue des surfaces d'Oku Slides (panneau et agent), implémenté par l'interface (`ui/src/a2ui`).
pub const CATALOG_ID: &str = "urn:oku-slides:a2ui:catalog:v2";

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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Component {
    pub id: String,
    #[serde(flatten)]
    pub kind: ComponentKind,
}

/// Composants du catalogue [`CATALOG_ID`]. Noms et propriétés reprennent
/// ceux du catalogue de base v0.9 ; `Stepper`, `ChoiceChips`, `ColorField`,
/// `SlideThumb`, `DiffText` sont propres à Oku Slides.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "component")]
pub enum ComponentKind {
    /// Enfants empilés verticalement.
    Column { children: Vec<String> },
    /// Un post-it autour d'un seul enfant.
    Card { child: String },
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
}

/// Style d'un texte (sous-ensemble des variantes v0.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TextVariant {
    H4,
    Body,
    Caption,
}

/// Une option d'un choix ou d'une liste. Aussi argument des outils de
/// l'agent (`ask_choice`, champs `select`) : désérialisable, avec schéma.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ChoiceOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Un morceau de texte gardé, retiré ou ajouté.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiffSegment {
    pub kind: DiffKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Keep,
    Del,
    Ins,
}

/// Une valeur écrite telle quelle, ou lue dans le modèle de données.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Action {
    pub event: Event,
}

/// Un évènement nommé ; l'interface lit les chemins du contexte à l'envoi.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub name: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, Dynamic<Value>>,
}

impl Component {
    pub fn new(id: impl Into<String>, kind: ComponentKind) -> Self {
        Self { id: id.into(), kind }
    }

    /// Un post-it autour de `child`.
    pub fn card(id: impl Into<String>, child: &str) -> Self {
        Self::new(id, ComponentKind::Card { child: child.to_owned() })
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

    pub fn delete_surface(surface_id: impl Into<String>) -> Self {
        Self::v0_9(ServerBody::DeleteSurface(DeleteSurface { surface_id: surface_id.into() }))
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod tests;
