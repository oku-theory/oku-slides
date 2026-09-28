//! Registre des widgets : compile une sonde Typst qui émet le schéma des
//! widgets de base et, s'il y en a, de ceux d'un pack.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use snafu::{OptionExt, ResultExt};
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::WidgetRegistry;

use crate::error::{BadPackIdSnafu, MissingSnafu, RegistryCompileSnafu, RegistryError, SchemaSnafu, SetupSnafu};
use crate::fonts::Fonts;
use crate::world::OkuSlidesWorld;

/// Les widgets d'un pack de modèles : son identifiant et son `widgets.typ`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackWidgets {
    id: String,
    source: String,
}

impl PackWidgets {
    /// `id` : lettres minuscules ASCII, chiffres, `-` et `_` ; il devient un chemin.
    pub fn new(id: impl Into<String>, source: impl Into<String>) -> Result<Self, RegistryError> {
        let id = id.into();
        let ok = !id.is_empty()
            && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
        if !ok {
            return Err(BadPackIdSnafu { id }.build().into());
        }
        Ok(Self { id, source: source.into() })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    fn path(&self) -> String {
        format!("/packs/{}/widgets.typ", self.id)
    }
}

/// Source de la sonde : importe les registres, émet leurs schémas.
fn probe_source(pack: Option<&PackWidgets>) -> String {
    let mut s = String::from("#import \"/oku-slides/src/widgets.typ\": registry as base\n");
    let pack_expr = match pack {
        Some(p) => {
            s.push_str(&format!("#import \"{}\": registry as pack\n", p.path()));
            "pack.map(w => w.schema)"
        }
        None => "none",
    };
    s.push_str(&format!("#metadata((base: base.map(w => w.schema), pack: {pack_expr}))<oku-widgets>\n"));
    s
}

/// Compile la sonde et lit le registre (base, surchargée par le pack).
pub fn load_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<WidgetRegistry, RegistryError> {
    let mut world = OkuSlidesWorld::new(fonts.clone()).map_err(Box::new).context(SetupSnafu)?;
    if let Some(p) = pack {
        world.add_source(&p.path(), p.source().to_owned()).map_err(Box::new).context(SetupSnafu)?;
    }
    world.set_main_text(&probe_source(pack));
    let result = typst::compile::<PagedDocument>(&world);
    let doc = result.output.map_err(|errors| {
        RegistryCompileSnafu { diagnostics: errors.iter().map(|d| world.diagnostic(d)).collect::<Vec<_>>() }.build()
    })?;
    let label = Label::new(PicoStr::intern("oku-widgets")).context(MissingSnafu)?;
    let found = doc.introspector().query(&Selector::Label(label));
    let metadata = found.first().and_then(|c| c.get_by_name("value").ok());
    let value = probe_json(metadata.map(|v| serde_json::to_value(&v)))?;
    Ok(WidgetRegistry::from_probe(value).map_err(Box::new).context(SchemaSnafu)?)
}

/// Sépare « la sonde n'a émis aucune métadonnée » (`Missing`) de « la
/// métadonnée existe mais ne se convertit pas en JSON » (`Schema`, avec un
/// [`oku_slides_core::WidgetError`] `Unreadable`).
fn probe_json(metadata: Option<Result<Json, serde_json::Error>>) -> Result<Json, RegistryError> {
    let Some(result) = metadata else { return Err(MissingSnafu.build().into()) };
    result.map_err(oku_slides_core::WidgetError::unreadable).map_err(Box::new).context(SchemaSnafu).map_err(Into::into)
}

/// Registres déjà lus, par pack : une sonde ne compile qu'une fois par
/// source de pack. La bibliothèque est dans le binaire : constante.
pub struct Registries {
    fonts: Fonts,
    cache: HashMap<Option<[u8; 32]>, Arc<WidgetRegistry>>,
}

impl std::fmt::Debug for Registries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registries").field("cached", &self.cache.len()).finish_non_exhaustive()
    }
}

impl Registries {
    pub fn new(fonts: Fonts) -> Self {
        Self { fonts, cache: HashMap::new() }
    }

    /// Le registre de la base, surchargé par `pack`.
    pub fn get(&mut self, pack: Option<&PackWidgets>) -> Result<Arc<WidgetRegistry>, RegistryError> {
        let key = pack.map(|p| {
            let mut h = Sha256::new();
            h.update(p.id().as_bytes());
            h.update([0]);
            h.update(p.source().as_bytes());
            <[u8; 32]>::from(h.finalize())
        });
        if let Some(r) = self.cache.get(&key) {
            return Ok(r.clone());
        }
        let r = Arc::new(load_registry(&self.fonts, pack)?);
        self.cache.insert(key, r.clone());
        Ok(r)
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
