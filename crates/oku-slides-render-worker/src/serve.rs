//! Boucle du travailleur : un `Renderer` par emplacement, polices et assets
//! par empreinte, un `Compiled` par génération jusqu'à sa libération.
//!
//! Règles :
//! - `Hello` répond la version du protocole du travailleur, quelle que soit celle de l'hôte.
//! - `fonts` et `assets` se désignent par empreinte ; `PutFont` et `PutAsset`
//!   rangent leur charge ; `DropFonts` oublie des polices (les emplacements
//!   ouverts gardent leur copie).
//! - `OpenSlot` bâtit les polices (un identifiant inconnu répond `Error Setup`),
//!   crée le `Renderer`, puis lui pose la langue courante si elle l'est.
//! - `SetWidgets` demande le registre de l'ensemble de packs à `Registries`, puis le pose.
//! - `SetTheme` pose les jetons du thème du slot (`None` : jetons par défaut).
//! - `ProbeFile` sonde un fichier seul, dans le dossier de la sorte de son pack
//!   (`Registries::probe_file`, mémoïsé).
//! - Un nom de fichier de widget invalide répond `Error Setup`.
//! - `Render` rend le deck sous une nouvelle génération (compteur global,
//!   croissant), rangée dans l'emplacement. Ensuite, les assets qui ne sont
//!   dans aucun des `assets` du dernier `Render` de chaque emplacement sont
//!   retirés (l'hôte applique la même règle).
//! - `Svg`, `Png` et `Pdf` d'une génération absente répondent `Error Stale`.
//! - `Release` retire les générations nommées ; `CloseSlot` l'emplacement.
//! - `Reference` rend la charge (un `.pptx`) par le témoin nommé (`Office2Pdf`
//!   ou `Soffice`) avec les polices nommées, une tranche de slides ou toutes,
//!   sous un nouvel identifiant (même compteur que les générations) ;
//!   `ReferencePng` en rastérise une page, `ReleaseReference` l'oublie. Une
//!   référence absente répond `Error Stale` ; LibreOffice hors délai,
//!   `Error Timeout` ; un autre échec du témoin, `Error Setup` (`reference: …`).
//! - Une requête qui échoue répond `Error` et la boucle continue ; une trame
//!   illisible arrête `serve`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_core::{Namespace, PackKind, WidgetFile};
use oku_slides_i18n::Localizer;
use oku_slides_platform::storage::Storage;
use oku_slides_reference::{
    Office2Pdf, ReferenceError, ReferenceErrorKind, ReferencePdf, ReferenceRenderer, SlideSpan, Soffice,
};
use oku_slides_render::{Compiled, Fonts, PackSet, PackWidgets, Registries, RenderError, Renderer, WireError};
use serde_json::{Map, Value};
use snafu::Snafu;

use crate::ReferenceWitness;
use crate::confine;
use crate::error::FrameError;
use crate::frame::{FrameLimits, read_frame, write_frame};
use crate::protocol::{CompiledHeader, PROTOCOL_VERSION, Request, Response, SlotId, WirePack};
use crate::watchdog::{Busy, Limits};

/// La boucle du travailleur s'est arrêtée sur un tube illisible.
#[derive(Debug, Snafu)]
pub struct ServeError(FrameError);

/// Réponse et charge d'une requête.
type Reply = (Response, Vec<u8>);

struct Slot {
    renderer: Renderer,
    generations: BTreeMap<u64, Compiled>,
    last_assets: Vec<String>,
}

#[derive(Default)]
struct Worker {
    fonts: HashMap<String, Vec<u8>>,
    assets: BTreeMap<String, Vec<u8>>,
    slots: BTreeMap<SlotId, Slot>,
    registries: Option<Registries>,
    localizer: Option<Localizer>,
    next_generation: u64,
    references: BTreeMap<u64, ReferencePdf>,
    /// Les fichiers du témoin LibreOffice, ouverts à son premier rendu.
    storage: Option<Storage>,
    /// Le dossier du travailleur (son TMPDIR), vidé après chaque référence.
    work: Option<PathBuf>,
}

/// Serves the requests of `input` until it ends.
///
/// # Errors
///
/// Fails if a frame of `input` is unreadable or if `output` rejects a reply.
pub fn serve(input: impl Read, output: impl Write, max_frame: usize) -> Result<(), ServeError> {
    serve_watched(input, output, max_frame, &Busy::default(), None)
}

/// [`serve`], en notant dans `busy` le début de la requête en cours.
fn serve_watched(
    mut input: impl Read,
    mut output: impl Write,
    max_frame: usize,
    busy: &Busy,
    work: Option<PathBuf>,
) -> Result<(), ServeError> {
    let mut worker = Worker { work, ..Worker::default() };
    while let Some(frame) = read_frame(&mut input, FrameLimits::requests(max_frame)).map_err(ServeError)? {
        busy.start(Instant::now());
        let (reply, payload) = match serde_json::from_slice::<Request>(&frame.header) {
            Ok(request) => worker.handle(request, frame.payload),
            Err(e) => error_reply(WireError::from_render(&RenderError::frame(e.to_string()))),
        };
        busy.end();
        // Pas de borne à l'écriture : l'hôte refuse à la lecture une réponse
        // trop grande (`Frame`, non bornée : aucun pack accusé) et, le tube
        // désynchronisé, relance le travailleur.
        write_frame(&mut output, &reply, &payload, FrameLimits::uniform(usize::MAX)).map_err(ServeError)?;
    }
    Ok(())
}

fn error_reply(error: WireError) -> Reply {
    (Response::Error { error }, Vec::new())
}

/// The `Ok` reply, without payload.
fn ok_reply() -> Reply {
    (Response::Ok, Vec::new())
}

fn setup(message: String) -> WireError {
    WireError::Setup { message }
}

fn render_error(e: &RenderError) -> WireError {
    WireError::from_render(e)
}

fn reference_error(e: &ReferenceError) -> WireError {
    setup(format!("reference: {e}"))
}

/// Le pack d'une requête ; un nom de fichier invalide répond `Error Setup`.
pub(crate) fn pack_of(p: WirePack) -> Result<PackWidgets, WireError> {
    let namespace = Namespace::parse(&p.namespace).ok_or_else(|| setup(format!("bad namespace {}", p.namespace)))?;
    let files = p
        .files
        .into_iter()
        .map(|(n, s)| WidgetFile::parse(&n).map(|n| (n, s)).ok_or_else(|| setup(format!("bad widget file {n}"))))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    PackWidgets::new(p.id, namespace, p.kind, files).map_err(|e| WireError::from_registry(&e))
}

impl Worker {
    fn handle(&mut self, request: Request, payload: Vec<u8>) -> Reply {
        let result = match request {
            Request::Hello { canary, .. } => self.hello(canary),
            Request::SetLanguage { locale } => self.set_language(&locale),
            Request::PutFont { id } => {
                self.fonts.insert(id, payload);
                Ok(ok_reply())
            }
            Request::DropFonts { ids } => {
                for id in ids {
                    self.fonts.remove(&id);
                }
                Ok(ok_reply())
            }
            Request::PutAsset { id } => {
                self.assets.insert(id, payload);
                Ok(ok_reply())
            }
            Request::ProbeFile { kind, name, source, common } => {
                self.probe_file(kind, &name, &source, common.as_deref())
            }
            Request::OpenSlot { slot, fonts } => self.open_slot(slot, &fonts),
            Request::CloseSlot { slot } => {
                self.slots.remove(&slot);
                self.evict_assets();
                Ok(ok_reply())
            }
            Request::SetWidgets { slot, packs } => self.set_widgets(slot, packs),
            Request::SetTheme { slot, tokens } => self.set_theme(slot, tokens),
            Request::Render { slot, deck, assets } => self.render(slot, &deck, assets),
            Request::Svg { slot, generation, page } => {
                self.with(slot, generation, |c| c.svg(page).map(String::into_bytes))
            }
            Request::Png { slot, generation, page, pixel_per_pt } => {
                self.with(slot, generation, |c| c.png(page, pixel_per_pt))
            }
            Request::Pdf { slot, generation } => self.with(slot, generation, Compiled::pdf),
            Request::Release { slot, generations } => {
                if let Some(s) = self.slots.get_mut(&slot) {
                    for g in generations {
                        s.generations.remove(&g);
                    }
                }
                Ok(ok_reply())
            }
            Request::Reference { witness, fonts, slides } => {
                let reply = self.reference(&witness, &fonts, slides, &payload);
                self.wipe_work();
                reply
            }
            Request::ReferencePng { reference, page, width } => self.reference_png(reference, page, width),
            Request::ReleaseReference { reference } => {
                self.references.remove(&reference);
                Ok(ok_reply())
            }
        };
        result.unwrap_or_else(error_reply)
    }

    /// Répond `Hello` : la version du travailleur et ce qu'a donné le canari.
    fn hello(&mut self, canary: Option<PathBuf>) -> Result<Reply, WireError> {
        let canary = match canary {
            Some(path) => Some(confine::probe_canary(&self.storage()?, &path)),
            None => None,
        };
        Ok((Response::Hello { version: PROTOCOL_VERSION, canary }, Vec::new()))
    }

    /// Le `Storage` local, ouvert à la première demande.
    fn storage(&mut self) -> Result<Storage, WireError> {
        let storage = match &self.storage {
            Some(storage) => storage.clone(),
            None => Storage::local().map_err(|e| setup(format!("storage: {e}")))?,
        };
        self.storage = Some(storage.clone());
        Ok(storage)
    }

    /// Vide le dossier du travailleur ; un échec est journalisé, la réponse reste.
    fn wipe_work(&mut self) {
        let Some(work) = self.work.clone() else { return };
        let wiped = self
            .storage()
            .map_err(|e| format!("{e:?}"))
            .and_then(|s| confine::wipe(&s, &work).map_err(|e| e.to_string()));
        if let Err(e) = wiped {
            tracing::warn!(error = %e, work = %work.display(), "dossier du travailleur non vidé");
        }
    }

    fn reference(
        &mut self,
        witness: &ReferenceWitness,
        fonts: &[String],
        slides: Option<(u32, u32)>,
        pptx: &[u8],
    ) -> Result<Reply, WireError> {
        // Une seule copie des polices : le témoin les garde le temps du rendu.
        let files = fonts
            .iter()
            .map(|id| self.fonts.get(id).cloned().ok_or_else(|| setup(format!("unknown font {id}"))))
            .collect::<Result<Vec<_>, _>>()?;
        let span = slides.map(|(first, last)| SlideSpan { first, last });
        let (pdf, name) = match witness {
            ReferenceWitness::Office2Pdf => {
                let pdf = Office2Pdf.render(pptx, files, span).map_err(|e| reference_error(&e))?;
                (pdf, Office2Pdf.witness().to_owned())
            }
            ReferenceWitness::Soffice { program, timeout_ms } => {
                let storage = self.storage()?;
                let soffice = Soffice::new(storage, program.clone(), Duration::from_millis(*timeout_ms));
                let pdf = soffice.render(pptx, files, span).map_err(|e| match e.kind() {
                    ReferenceErrorKind::Timeout => WireError::Timeout { ms: *timeout_ms },
                    _ => reference_error(&e),
                })?;
                (pdf, soffice.witness().to_owned())
            }
        };
        let pages = pdf.page_count();
        self.next_generation += 1;
        let reference = self.next_generation;
        self.references.insert(reference, pdf);
        Ok((Response::Reference { reference, witness: name, pages }, Vec::new()))
    }

    fn reference_png(&self, reference: u64, page: usize, width: u32) -> Result<Reply, WireError> {
        let pdf = self.references.get(&reference).ok_or(WireError::Stale)?;
        let png = pdf.raster(page, width).and_then(|r| r.to_png()).map_err(|e| reference_error(&e))?;
        Ok((Response::Bytes, png))
    }

    fn fonts_of(&self, ids: &[String]) -> Result<Fonts, WireError> {
        let files = ids
            .iter()
            .map(|id| self.fonts.get(id).cloned().ok_or_else(|| setup(format!("unknown font {id}"))))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Fonts::new(files))
    }

    fn registries(&mut self) -> &mut Registries {
        // Les schémas ne dépendent pas des polices : celles de Typst suffisent.
        self.registries.get_or_insert_with(|| Registries::new(Fonts::default()))
    }

    fn probe_file(
        &mut self,
        kind: PackKind,
        name: &str,
        source: &str,
        common: Option<&str>,
    ) -> Result<Reply, WireError> {
        let name = WidgetFile::parse(name).ok_or_else(|| setup(format!("bad widget file {name}")))?;
        let probe = self.registries().probe_file(kind, &name, source, common);
        Ok((Response::FileProbe { probe }, Vec::new()))
    }

    fn set_language(&mut self, locale: &str) -> Result<Reply, WireError> {
        let localizer = Localizer::new(Some(locale));
        for slot in self.slots.values_mut() {
            slot.renderer.set_language(&localizer).map_err(|e| render_error(&e))?;
        }
        self.localizer = Some(localizer);
        Ok(ok_reply())
    }

    fn open_slot(&mut self, slot: SlotId, fonts: &[String]) -> Result<Reply, WireError> {
        let mut renderer = Renderer::new(self.fonts_of(fonts)?).map_err(|e| render_error(&e))?;
        if let Some(l) = &self.localizer {
            renderer.set_language(l).map_err(|e| render_error(&e))?;
        }
        self.slots.insert(slot, Slot { renderer, generations: BTreeMap::new(), last_assets: Vec::new() });
        Ok(ok_reply())
    }

    fn set_widgets(&mut self, slot: SlotId, packs: Vec<WirePack>) -> Result<Reply, WireError> {
        let packs = packs.into_iter().map(pack_of).collect::<Result<Vec<_>, _>>()?;
        let set = PackSet::new(packs).map_err(|e| WireError::from_registry(&e))?;
        let registry = self.registries().get(&set).map_err(|e| WireError::from_registry(&e))?;
        let s = self.slots.get_mut(&slot).ok_or_else(|| setup(format!("unknown slot {slot}")))?;
        s.renderer.set_widgets(&set, registry).map_err(|e| render_error(&e))?;
        Ok(ok_reply())
    }

    fn set_theme(&mut self, slot: SlotId, tokens: Option<Map<String, Value>>) -> Result<Reply, WireError> {
        let s = self.slots.get_mut(&slot).ok_or_else(|| setup(format!("unknown slot {slot}")))?;
        s.renderer.set_theme(tokens).map_err(|e| render_error(&e))?;
        Ok(ok_reply())
    }

    fn render(&mut self, slot: SlotId, deck: &oku_slides_core::Deck, assets: Vec<String>) -> Result<Reply, WireError> {
        let s = self.slots.get_mut(&slot).ok_or_else(|| setup(format!("unknown slot {slot}")))?;
        let compiled = s.renderer.render(deck, &self.assets).map_err(|e| render_error(&e))?;
        self.next_generation += 1;
        let generation = self.next_generation;
        let page_count = compiled.page_count();
        let header = CompiledHeader {
            generation,
            page_count,
            page_hashes: (0..page_count)
                .map(|i| format!("{:032x}", compiled.page_hash(i).unwrap_or_default()))
                .collect(),
            layout: compiled.layout().clone(),
            tokens: compiled.layout().tokens.clone(),
            warnings: compiled.warnings.clone(),
            chart_palette: s.renderer.chart_palette().to_vec(),
        };
        s.generations.insert(generation, compiled);
        s.last_assets = assets;
        self.evict_assets();
        Ok((Response::Compiled(Box::new(header)), Vec::new()))
    }

    /// Ne garde que les assets du dernier rendu de chaque emplacement (règle partagée avec l'hôte).
    fn evict_assets(&mut self) {
        let live: BTreeSet<&String> = self.slots.values().flat_map(|s| &s.last_assets).collect();
        self.assets.retain(|id, _| live.contains(id));
    }

    fn with(
        &self,
        slot: SlotId,
        generation: u64,
        f: impl FnOnce(&Compiled) -> Result<Vec<u8>, RenderError>,
    ) -> Result<Reply, WireError> {
        let compiled = self.slots.get(&slot).and_then(|s| s.generations.get(&generation)).ok_or(WireError::Stale)?;
        Ok((Response::Bytes, f(compiled).map_err(|e| render_error(&e))?))
    }
}

/// Taille de trame par défaut d'un travailleur lancé sans `--max-frame-bytes`.
const DEFAULT_MAX_FRAME: usize = 512 * 1024 * 1024;

/// Arguments d'un processus lancé en travailleur.
#[derive(Debug)]
pub(crate) struct WorkerArgs {
    pub(crate) max_frame: usize,
    /// Bornes du chien de garde, tirées de celles de l'hôte.
    pub(crate) limits: Limits,
    /// Le dossier du travailleur, vidé après chaque référence (`--work-dir`).
    pub(crate) work: Option<PathBuf>,
}

/// Les arguments du travailleur (`--render-worker` en premier), sinon `None`.
///
/// `--max-frame-bytes <n>`, `--timeout-ms <n>`, `--memory-bytes <n>` et
/// `--work-dir <path>` sont facultatifs : sans eux, taille par défaut et pas de borne.
pub(crate) fn worker_args(args: &[OsString]) -> Option<WorkerArgs> {
    if args.first().is_none_or(|a| a != "--render-worker") {
        return None;
    }
    let value = |name: &str| -> Option<u64> {
        let i = args.iter().position(|a| a == name)?;
        args.get(i + 1)?.to_str()?.parse().ok()
    };
    let max_frame = value("--max-frame-bytes").and_then(|n| usize::try_from(n).ok()).unwrap_or(DEFAULT_MAX_FRAME);
    let work = args.iter().position(|a| a == "--work-dir").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    Some(WorkerArgs { max_frame, limits: Limits::from_host(value("--timeout-ms"), value("--memory-bytes")), work })
}

/// Si ce processus a été lancé en travailleur (`--render-worker
/// --max-frame-bytes <n> --timeout-ms <n> --memory-bytes <n> --work-dir <path>`), sert
/// stdin/stdout sous son chien de garde puis termine le processus ; sinon ne
/// fait rien. À appeler en tête de `main`, avant toute initialisation.
///
/// Les trames partent sur le stdout d'origine ; tout `print!` du processus va
/// sur stderr.
pub fn exit_if_worker() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let Some(args) = worker_args(&args) else { return };
    oku_slides_platform::telemetry::init(); // journaux sur stderr : stdout est le tube
    let output = match protocol_output() {
        Ok(output) => output,
        Err(e) => {
            tracing::error!(error = %e, "sortie du processus de rendu");
            std::process::exit(1);
        }
    };
    let busy = Arc::new(Busy::default());
    crate::watchdog::spawn(args.limits, busy.clone());
    let code = match serve_watched(std::io::stdin().lock(), output, args.max_frame, &busy, args.work) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!(error = %e, "processus de rendu");
            1
        }
    };
    std::process::exit(code);
}

/// La sortie des trames : une copie de stdout, stdout redirigé vers stderr
/// (voir `stdio`) ; tamponnée, `write_frame` vide le tampon à chaque trame.
#[cfg(unix)]
fn protocol_output() -> std::io::Result<Box<dyn Write>> {
    Ok(Box::new(std::io::BufWriter::new(crate::stdio::protocol_output()?)))
}

/// Ailleurs qu'en unix, stdout tel quel.
#[cfg(not(unix))]
fn protocol_output() -> std::io::Result<Box<dyn Write>> {
    Ok(Box::new(std::io::stdout()))
}

#[cfg(test)]
#[path = "serve_tests.rs"]
mod serve_tests;
