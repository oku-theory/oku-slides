//! Registre des widgets : une sonde Typst par pack (base, puis le module
//! d'assemblage du pack) et une sonde par fichier de widget (`widget_probe`).

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use snafu::{OptionExt, ResultExt};
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::{WidgetFile, WidgetRegistry};

use crate::error::{BadPackIdSnafu, MissingSnafu, RegistryCompileSnafu, RegistryError, SchemaSnafu, SetupSnafu};
use crate::fonts::Fonts;
use crate::probe_cache::FileProbeCache;
use crate::widget_probe::{FileProbe, probe_key};
use crate::world::OkuSlidesWorld;

/// Un identifiant de pack utilisable comme nom de dossier et de chemin :
/// lettres minuscules ASCII, chiffres, `-` et `_`. Jamais `..` ni un chemin.
pub fn is_pack_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// Les widgets d'un pack de modèles : son identifiant et ses fichiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackWidgets {
    id: String,
    files: BTreeMap<WidgetFile, String>,
}

impl PackWidgets {
    /// `id` : voir [`is_pack_id`] ; il devient un chemin.
    pub fn new(id: impl Into<String>, files: BTreeMap<WidgetFile, String>) -> Result<Self, RegistryError> {
        let id = id.into();
        if !is_pack_id(&id) {
            return Err(BadPackIdSnafu { id }.build().into());
        }
        Ok(Self { id, files })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Les fichiers du pack, `_commun` compris.
    pub fn files(&self) -> &BTreeMap<WidgetFile, String> {
        &self.files
    }

    /// La source de `_commun`, s'il y en a un.
    pub fn common(&self) -> Option<&str> {
        self.files.get(&WidgetFile::common()).map(String::as_str)
    }

    /// Le même pack, réduit à `_commun` et aux widgets de `keep`.
    pub fn only(&self, keep: &BTreeSet<WidgetFile>) -> Self {
        let files = self
            .files
            .iter()
            .filter(|(n, _)| n.is_common() || keep.contains(*n))
            .map(|(n, s)| (n.clone(), s.clone()))
            .collect();
        Self { id: self.id.clone(), files }
    }

    /// Empreinte (identifiant et fichiers) : clé des caches de registre.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.id.as_bytes());
        h.update([0]);
        for (name, source) in &self.files {
            h.update(name.as_str().as_bytes());
            h.update([0]);
            h.update(source.as_bytes());
            h.update([0]);
        }
        h.finalize().into()
    }

    /// Module d'assemblage : importe chaque widget (pas `_commun`) et forme `registry`.
    pub(crate) fn assembly(&self) -> String {
        let mut out = String::new();
        let mut names = String::new();
        for (i, name) in self.files.keys().filter(|n| !n.is_common()).enumerate() {
            out.push_str(&format!("#import \"widgets/{}\": widget as w{i}\n", name.file_name()));
            names.push_str(&format!("w{i},"));
        }
        out.push_str(&format!("#let registry = ({names})\n"));
        out
    }

    /// Chemin du module d'assemblage.
    pub(crate) fn path(&self) -> String {
        format!("/packs/{}/widgets.typ", self.id)
    }

    /// Chaque source du pack et son chemin, assemblage compris.
    pub(crate) fn sources(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .files
            .iter()
            .map(|(n, s)| (format!("/packs/{}/widgets/{}", self.id, n.file_name()), s.clone()))
            .collect();
        out.push((self.path(), self.assembly()));
        out
    }
}

/// Source de la sonde : importe la base et le module `pack_path`, émet leurs schémas.
pub(crate) fn probe_source(pack_path: Option<&str>) -> String {
    let mut s = String::from("#import \"/oku-slides/src/widgets.typ\": registry as base\n");
    let pack_expr = match pack_path {
        Some(path) => {
            s.push_str(&format!("#import \"{path}\": registry as pack\n"));
            "pack.map(w => w.schema)"
        }
        None => "none",
    };
    s.push_str(&format!("#metadata((base: base.map(w => w.schema), pack: {pack_expr}))<oku-widgets>\n"));
    s
}

/// Compile la sonde posée dans `world` et rend son JSON brut.
pub(crate) fn compile_probe(world: &OkuSlidesWorld) -> Result<Json, RegistryError> {
    let result = typst::compile::<PagedDocument>(world);
    let doc = result.output.map_err(|errors| {
        RegistryCompileSnafu { diagnostics: errors.iter().map(|d| world.diagnostic(d)).collect::<Vec<_>>() }.build()
    })?;
    let label = Label::new(PicoStr::intern("oku-widgets")).context(MissingSnafu)?;
    let found = doc.introspector().query(&Selector::Label(label));
    let metadata = found.first().and_then(|c| c.get_by_name("value").ok());
    probe_json(metadata.map(|v| serde_json::to_value(&v)))
}

/// Compile la sonde et rend son JSON brut (schémas de la base et du pack).
pub fn probe_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<Json, RegistryError> {
    let mut world = OkuSlidesWorld::new(fonts.clone()).map_err(Box::new).context(SetupSnafu)?;
    if let Some(p) = pack {
        for (path, text) in p.sources() {
            world.add_source(&path, text).map_err(Box::new).context(SetupSnafu)?;
        }
    }
    world.set_main_text(&probe_source(pack.map(PackWidgets::path).as_deref()));
    compile_probe(&world)
}

/// Le registre (base surchargée par le pack) décrit par le JSON d'une sonde.
pub fn registry_from_probe(json: Json) -> Result<WidgetRegistry, RegistryError> {
    Ok(WidgetRegistry::from_probe(json).map_err(Box::new).context(SchemaSnafu)?)
}

/// Compile la sonde et lit le registre (base, surchargée par le pack).
pub fn load_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<WidgetRegistry, RegistryError> {
    registry_from_probe(probe_registry(fonts, pack)?)
}

/// Sonde brute de la base, lue une fois par processus.
static BASE_PROBE: Mutex<Option<Arc<Json>>> = Mutex::new(None);

/// La sonde brute (JSON) de la base, compilée au premier appel.
pub fn base_probe(fonts: &Fonts) -> Result<Arc<Json>, RegistryError> {
    let mut probe = BASE_PROBE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(p) = probe.as_ref() {
        return Ok(p.clone());
    }
    let p = Arc::new(probe_registry(fonts, None)?);
    *probe = Some(p.clone());
    Ok(p)
}

/// Registre de base, lu une fois par processus : la bibliothèque est dans
/// le binaire, et les polices n'entrent pas dans les schémas.
static BASE: Mutex<Option<Arc<WidgetRegistry>>> = Mutex::new(None);

/// Le registre des widgets de base (sonde compilée au premier appel).
pub fn base_registry(fonts: &Fonts) -> Result<Arc<WidgetRegistry>, RegistryError> {
    let mut base = BASE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(r) = base.as_ref() {
        return Ok(r.clone());
    }
    let r = Arc::new(registry_from_probe((*base_probe(fonts)?).clone())?);
    *base = Some(r.clone());
    Ok(r)
}

/// Sépare « la sonde n'a émis aucune métadonnée » (`Missing`) de « la
/// métadonnée existe mais ne se convertit pas en JSON » (`Schema`, avec un
/// [`oku_slides_core::WidgetError`] `Unreadable`).
fn probe_json(metadata: Option<Result<Json, serde_json::Error>>) -> Result<Json, RegistryError> {
    let Some(result) = metadata else { return Err(MissingSnafu.build().into()) };
    result.map_err(oku_slides_core::WidgetError::unreadable).map_err(Box::new).context(SchemaSnafu).map_err(Into::into)
}

/// Registres de packs gardés par [`Registries`], au plus : les plus récents.
pub(crate) const MAX_CACHED_PACKS: usize = 8;

/// Sondes et registres déjà lus, par pack : une sonde ne compile qu'une
/// fois par source de pack. La bibliothèque est dans le binaire : constante.
///
/// Caches bornés : les [`MAX_CACHED_PACKS`] packs les plus récemment servis,
/// et quelques sondes par fichier ([`FileProbeCache`]) ; éditer un widget
/// ne les fait pas grandir.
pub struct Registries {
    fonts: Fonts,
    cache: HashMap<[u8; 32], Arc<WidgetRegistry>>,
    /// Clés de `cache`, de la moins à la plus récemment servie.
    recent: VecDeque<[u8; 32]>,
    /// Sondes de fichiers, par nom et [`probe_key`].
    files: FileProbeCache,
    /// Compilations de sondes de fichiers (tests du cache).
    #[cfg(test)]
    pub(crate) file_compiles: usize,
}

impl std::fmt::Debug for Registries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registries")
            .field("cached", &self.cache.len())
            .field("files", &self.files.len())
            .finish_non_exhaustive()
    }
}

impl Registries {
    pub fn new(fonts: Fonts) -> Self {
        Self {
            fonts,
            cache: HashMap::new(),
            recent: VecDeque::new(),
            files: FileProbeCache::default(),
            #[cfg(test)]
            file_compiles: 0,
        }
    }

    /// Le registre de la base, surchargé par `pack`. Sans pack : celui de
    /// la base, lu une fois par processus (le même que [`crate::Renderer::new`]).
    pub fn get(&mut self, pack: Option<&PackWidgets>) -> Result<Arc<WidgetRegistry>, RegistryError> {
        match pack {
            None => base_registry(&self.fonts),
            Some(p) => self.entry(p),
        }
    }

    /// La sonde du fichier `name` (avec `_commun`), mémoïsée par [`probe_key`]
    /// dans un [`FileProbeCache`].
    pub fn probe_file(&mut self, name: &WidgetFile, source: &str, common: Option<&str>) -> FileProbe {
        let key = probe_key(name, source, common);
        if let Some(probe) = self.files.get(name, &key) {
            return probe;
        }
        #[cfg(test)]
        {
            self.file_compiles += 1;
        }
        let probe = crate::widget_probe::probe_file(&self.fonts, name, source, common);
        self.files.insert(name, key, probe.clone());
        probe
    }

    fn entry(&mut self, pack: &PackWidgets) -> Result<Arc<WidgetRegistry>, RegistryError> {
        let key = pack.fingerprint();
        let registry = match self.cache.get(&key) {
            Some(registry) => {
                let registry = registry.clone();
                self.recent.retain(|k| *k != key);
                registry
            }
            None => {
                let registry = Arc::new(load_registry(&self.fonts, Some(pack))?);
                while self.recent.len() >= MAX_CACHED_PACKS {
                    if let Some(old) = self.recent.pop_front() {
                        self.cache.remove(&old);
                    }
                }
                self.cache.insert(key, registry.clone());
                registry
            }
        };
        self.recent.push_back(key);
        Ok(registry)
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
