//! A2UI v0.9 server messages: creating, updating and removing surfaces.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::component::Component;

/// Catalog of Oku Slides surfaces (panel and agent), implemented by the UI (`ui/src/a2ui`).
pub const CATALOG_ID: &str = "urn:oku-slides:a2ui:catalog:v4";

/// Protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Version {
    /// A2UI v0.9, the only version the UI speaks.
    #[serde(rename = "v0.9")]
    V0_9,
}

/// A message from the server to the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerMessage {
    /// Protocol version, always v0.9.
    pub version: Version,
    /// The message itself, flattened next to `version`.
    #[serde(flatten)]
    pub body: ServerBody,
}

/// Body of a [`ServerMessage`]: a single key, the message name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerBody {
    /// `createSurface`: opens an empty surface.
    CreateSurface(CreateSurface),
    /// `updateComponents`: adds or replaces components.
    UpdateComponents(UpdateComponents),
    /// `updateDataModel`: writes or clears a model value.
    UpdateDataModel(UpdateDataModel),
    /// `deleteSurface`: removes a surface.
    DeleteSurface(DeleteSurface),
}

/// Creates an empty surface, rendered with a catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSurface {
    /// Surface identifier, reused by the following messages.
    pub surface_id: String,
    /// Catalog that renders its components ([`CATALOG_ID`] for Oku Slides).
    pub catalog_id: String,
}

/// Adds or replaces components (by identifier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateComponents {
    /// The target surface.
    pub surface_id: String,
    /// Components to set, flat; a known identifier replaces the old one.
    pub components: Vec<Component>,
}

/// Writes `value` at `path` in the data model.
///
/// Without a path, it writes the root; without a value, it clears whatever is there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataModel {
    /// The target surface.
    pub surface_id: String,
    /// JSON Pointer path written; absent: the model root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    // Read back: an explicit `null` becomes `None` (clear); the host never sends `Some(Null)`.
    /// Value written; absent: clears whatever is at the path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

/// Removes a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSurface {
    /// The removed surface.
    pub surface_id: String,
}

impl ServerMessage {
    fn v0_9(body: ServerBody) -> Self {
        Self { version: Version::V0_9, body }
    }

    /// `createSurface` with the Oku Slides catalog.
    pub fn create_surface(surface_id: impl Into<String>) -> Self {
        Self::v0_9(ServerBody::CreateSurface(CreateSurface {
            surface_id: surface_id.into(),
            catalog_id: CATALOG_ID.to_owned(),
        }))
    }

    /// `updateComponents`: sets `components` in the surface.
    pub fn update_components(surface_id: impl Into<String>, components: Vec<Component>) -> Self {
        Self::v0_9(ServerBody::UpdateComponents(UpdateComponents { surface_id: surface_id.into(), components }))
    }

    /// Replaces the whole data model.
    pub fn set_data_model(surface_id: impl Into<String>, value: Value) -> Self {
        Self::v0_9(ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: surface_id.into(),
            path: None,
            value: Some(value),
        }))
    }

    /// Writes `value` at `path` (JSON Pointer) in the data model.
    pub fn update_data_model(surface_id: impl Into<String>, path: &str, value: Value) -> Self {
        Self::v0_9(ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: surface_id.into(),
            path: Some(path.to_owned()),
            value: Some(value),
        }))
    }

    /// `deleteSurface`: removes the surface.
    pub fn delete_surface(surface_id: impl Into<String>) -> Self {
        Self::v0_9(ServerBody::DeleteSurface(DeleteSurface { surface_id: surface_id.into() }))
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod tests;
