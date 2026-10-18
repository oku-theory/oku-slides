//! Registre des widgets : une sonde Typst par ensemble de packs (base, puis le
//! module d'assemblage de chaque pack) et une sonde par fichier de widget (`widget_probe`).

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use snafu::{OptionExt, ResultExt};
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::{Namespace, WidgetFile, WidgetRegistry};

use crate::error::{
    BadPackIdSnafu, DuplicatePackSnafu, MissingSnafu, RegistryCompileSnafu, RegistryError, SchemaSnafu, SetupSnafu,
};
use crate::fonts::Fonts;
use crate::probe_cache::FileProbeCache;
use crate::widget_probe::{FileProbe, probe_key};
use crate::world::OkuSlidesWorld;

/// Un identifiant de pack utilisable comme nom de dossier et de chemin :
/// lettres minuscules ASCII, chiffres, `-` et `_`. Jamais `..` ni un chemin.
pub fn is_pack_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// Les widgets d'un pack : son identifiant, son espace de noms et ses fichiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackWidgets {
    id: String,
    namespace: Namespace,
    files: BTreeMap<WidgetFile, String>,
}

impl PackWidgets {
    /// `id` : voir [`is_pack_id`] ; il devient un chemin.
    pub fn new(
        id: impl Into<String>,
        namespace: Namespace,
        files: BTreeMap<WidgetFile, String>,
    ) -> Result<Self, RegistryError> {
        let id = id.into();
        if !is_pack_id(&id) {
            return Err(BadPackIdSnafu { id }.build().into());
        }
        Ok(Self { id, namespace, files })
    }

    /// L'identifiant du pack, servi comme dossier sous `/packs/`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// L'espace de noms qui préfixe les widgets du pack.
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
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
        Self { id: self.id.clone(), namespace: self.namespace.clone(), files }
    }

    /// Empreinte (identifiant, espace de noms et fichiers) : clé des caches de registre.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.id.as_bytes());
        h.update([0]);
        h.update(self.namespace.as_str().as_bytes());
        h.update([0]);
        for (name, source) in &self.files {
            h.update(name.as_str().as_bytes());
            h.update([0]);
            h.update(source.as_bytes());
            h.update([0]);
        }
        h.finalize().into()
    }

    /// Module d'assemblage : importe chaque widget (pas `_commun`) et forme `registry`,
    /// chaque nom préfixé par l'espace de noms du pack (voir `oku_slides_core::qualify`).
    pub(crate) fn assembly(&self) -> String {
        let mut out = String::new();
        let mut names = String::new();
        for (i, name) in self.files.keys().filter(|n| !n.is_common()).enumerate() {
            out.push_str(&format!("#import \"widgets/{}\": widget as w{i}\n", name.file_name()));
            names.push_str(&format!("w{i},"));
        }
        // Le préfixe suit `qualify` : `<namespace>/<nom>` ; l'espace de noms est contrôlé.
        out.push_str(&format!(
            "#let registry = ({names}).map(w => w + (name: \"{ns}/\" + w.name))\n",
            ns = self.namespace
        ));
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

/// Les widgets de plusieurs packs d'un deck, triés par espace de noms.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackSet(Vec<PackWidgets>);

impl PackSet {
    /// L'ensemble de `packs`, trié par espace de noms.
    ///
    /// Deux packs sous le même espace de noms ou au même identifiant (même
    /// dossier `/packs/<id>/`) sont refusés (`DuplicatePack`).
    pub fn new(mut packs: Vec<PackWidgets>) -> Result<Self, RegistryError> {
        packs.sort_by(|a, b| a.namespace.cmp(&b.namespace));
        let mut ids = BTreeSet::new();
        for (i, p) in packs.iter().enumerate() {
            if i > 0 && packs[i - 1].namespace == p.namespace {
                return Err(DuplicatePackSnafu { pack: p.namespace.as_str() }.build().into());
            }
            if !ids.insert(p.id.as_str()) {
                return Err(DuplicatePackSnafu { pack: p.id.as_str() }.build().into());
            }
        }
        Ok(Self(packs))
    }

    /// Les packs, triés par espace de noms.
    pub fn packs(&self) -> &[PackWidgets] {
        &self.0
    }

    /// Vrai sans aucun pack : seuls les widgets du noyau.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Empreinte de l'ensemble : clé du cache des registres assemblés.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        for p in &self.0 {
            h.update(p.fingerprint());
        }
        h.finalize().into()
    }

    /// Chaque source de chaque pack et son chemin, assemblages compris.
    pub(crate) fn sources(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.0.iter().flat_map(PackWidgets::sources)
    }
}

/// Source de la sonde : importe la base et le module d'assemblage de chaque pack
/// (`(espace de noms, chemin)`), émet leurs schémas.
pub(crate) fn probe_source<'a>(packs: impl IntoIterator<Item = (&'a Namespace, String)>) -> String {
    let mut s = String::from("#import \"/oku-slides/src/widgets.typ\": registry as base\n");
    // Un tableau Typst à un élément s'écrit `(x,)` : chaque entrée finit par une virgule.
    let mut entries = String::new();
    for (i, (namespace, path)) in packs.into_iter().enumerate() {
        s.push_str(&format!("#import \"{path}\": registry as pack{i}\n"));
        // Le schéma garde le nom nu : `WidgetRegistry::from_probe` le qualifie.
        entries.push_str(&format!("(namespace: \"{namespace}\", widgets: pack{i}.map(w => w.schema)),"));
    }
    s.push_str(&format!("#metadata((base: base.map(w => w.schema), packs: ({entries})))<oku-widgets>\n"));
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

/// Compile la sonde et rend son JSON brut (schémas de la base et de chaque pack).
pub fn probe_registry(fonts: &Fonts, packs: &PackSet) -> Result<Json, RegistryError> {
    let mut world = OkuSlidesWorld::new(fonts.clone()).map_err(Box::new).context(SetupSnafu)?;
    for (path, text) in packs.sources() {
        world.add_source(&path, text).map_err(Box::new).context(SetupSnafu)?;
    }
    world.set_main_text(&probe_source(packs.packs().iter().map(|p| (p.namespace(), p.path()))));
    compile_probe(&world)
}

/// Le registre (base, et widgets qualifiés des packs) décrit par le JSON d'une sonde.
pub fn registry_from_probe(json: Json) -> Result<WidgetRegistry, RegistryError> {
    Ok(WidgetRegistry::from_probe(json).map_err(Box::new).context(SchemaSnafu)?)
}

/// Compile la sonde et lit le registre (base, et widgets qualifiés des packs).
pub fn load_registry(fonts: &Fonts, packs: &PackSet) -> Result<WidgetRegistry, RegistryError> {
    registry_from_probe(probe_registry(fonts, packs)?)
}

/// Sonde brute de la base, lue une fois par processus.
static BASE_PROBE: Mutex<Option<Arc<Json>>> = Mutex::new(None);

/// La sonde brute (JSON) de la base, compilée au premier appel.
pub fn base_probe(fonts: &Fonts) -> Result<Arc<Json>, RegistryError> {
    let mut probe = BASE_PROBE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(p) = probe.as_ref() {
        return Ok(p.clone());
    }
    let p = Arc::new(probe_registry(fonts, &PackSet::default())?);
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

/// Registres assemblés gardés par [`Registries`], au plus : les plus récents.
pub(crate) const MAX_CACHED_PACKS: usize = 8;

/// Sondes et registres déjà lus, par ensemble de packs ([`PackSet::fingerprint`]) :
/// une sonde ne compile qu'une fois par ensemble. La bibliothèque est dans le binaire : constante.
///
/// Caches bornés : les [`MAX_CACHED_PACKS`] ensembles les plus récemment servis,
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

    /// Le registre de la base et des widgets qualifiés de `set`. Sans pack :
    /// celui de la base, lu une fois par processus (le même que [`crate::Renderer::new`]).
    pub fn get(&mut self, set: &PackSet) -> Result<Arc<WidgetRegistry>, RegistryError> {
        if set.is_empty() {
            return base_registry(&self.fonts);
        }
        self.entry(set)
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

    fn entry(&mut self, set: &PackSet) -> Result<Arc<WidgetRegistry>, RegistryError> {
        let key = set.fingerprint();
        let registry = match self.cache.get(&key) {
            Some(registry) => {
                let registry = registry.clone();
                self.recent.retain(|k| *k != key);
                registry
            }
            None => {
                let registry = Arc::new(load_registry(&self.fonts, set)?);
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
