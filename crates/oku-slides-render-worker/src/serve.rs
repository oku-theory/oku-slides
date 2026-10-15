//! Boucle du travailleur : un `Renderer` par emplacement, polices et assets
//! par empreinte, un `Compiled` par génération jusqu'à sa libération.
//!
//! Règles :
//! - `fonts` et `assets` se désignent par empreinte ; `PutFont` et `PutAsset`
//!   rangent leur charge.
//! - `OpenSlot` bâtit les polices (un identifiant inconnu répond `Error Setup`),
//!   crée le `Renderer`, puis lui pose la langue courante si elle l'est.
//! - `SetWidgets` demande le registre du pack à `Registries`, puis le pose.
//! - `SetTheme` pose les jetons du thème du slot (`None` : jetons par défaut).
//! - `ProbeFile` sonde un fichier seul (`Registries::probe_file`, mémoïsé).
//! - Un nom de fichier de widget invalide répond `Error Setup`.
//! - `Render` rend le deck sous une nouvelle génération (compteur global,
//!   croissant), rangée dans l'emplacement. Ensuite, les assets qui ne sont
//!   dans aucun des `assets` du dernier `Render` de chaque emplacement sont
//!   retirés (l'hôte applique la même règle).
//! - `Svg`, `Png` et `Pdf` d'une génération absente répondent `Error Stale`.
//! - `Release` retire les générations nommées ; `CloseSlot` l'emplacement.
//! - `Reference` rend la charge (un `.pptx`) par le témoin (`Office2Pdf`) avec
//!   les polices nommées, une tranche de slides ou toutes, sous un nouvel identifiant (même compteur que les
//!   générations) ; `ReferencePng` en rastérise une page, `ReleaseReference`
//!   l'oublie. Une référence absente répond `Error Stale` ; un échec du témoin,
//!   `Error Setup` (`reference: …`).
//! - Une requête qui échoue répond `Error` et la boucle continue ; une trame
//!   illisible arrête `serve`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Instant;

use oku_slides_core::WidgetFile;
use oku_slides_i18n::Localizer;
use oku_slides_reference::{Office2Pdf, ReferenceError, ReferencePdf, ReferenceRenderer, SlideSpan};
use oku_slides_render::{Compiled, Fonts, PackWidgets, Registries, RenderError, Renderer, WireError};
use serde_json::{Map, Value};
use snafu::Snafu;

use crate::error::FrameError;
use crate::frame::{FrameLimits, read_frame, write_frame};
use crate::protocol::{CompiledHeader, Request, Response, SlotId, WirePack};
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
}

/// Sert les requêtes de `input` jusqu'à sa fin.
pub fn serve(input: impl Read, output: impl Write, max_frame: usize) -> Result<(), ServeError> {
    serve_watched(input, output, max_frame, &Busy::default())
}

/// [`serve`], en notant dans `busy` le début de la requête en cours.
fn serve_watched(
    mut input: impl Read,
    mut output: impl Write,
    max_frame: usize,
    busy: &Busy,
) -> Result<(), ServeError> {
    let mut worker = Worker::default();
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

fn ok() -> Result<Reply, WireError> {
    Ok((Response::Ok, Vec::new()))
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
fn pack_of(p: WirePack) -> Result<PackWidgets, WireError> {
    let files = p
        .files
        .into_iter()
        .map(|(n, s)| WidgetFile::parse(&n).map(|n| (n, s)).ok_or_else(|| setup(format!("bad widget file {n}"))))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    PackWidgets::new(p.id, files).map_err(|e| WireError::from_registry(&e))
}

impl Worker {
    fn handle(&mut self, request: Request, payload: Vec<u8>) -> Reply {
        let result = match request {
            Request::SetLanguage { locale } => self.set_language(&locale),
            Request::PutFont { id } => {
                self.fonts.insert(id, payload);
                ok()
            }
            Request::PutAsset { id } => {
                self.assets.insert(id, payload);
                ok()
            }
            Request::ProbeFile { name, source, common } => self.probe_file(&name, &source, common.as_deref()),
            Request::OpenSlot { slot, fonts } => self.open_slot(slot, &fonts),
            Request::CloseSlot { slot } => {
                self.slots.remove(&slot);
                self.evict_assets();
                ok()
            }
            Request::SetWidgets { slot, pack } => self.set_widgets(slot, pack),
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
                ok()
            }
            Request::Reference { fonts, slides } => self.reference(&fonts, slides, &payload),
            Request::ReferencePng { reference, page, width } => self.reference_png(reference, page, width),
            Request::ReleaseReference { reference } => {
                self.references.remove(&reference);
                ok()
            }
        };
        result.unwrap_or_else(error_reply)
    }

    fn reference(&mut self, fonts: &[String], slides: Option<(u32, u32)>, pptx: &[u8]) -> Result<Reply, WireError> {
        // Une seule copie des polices : le témoin les garde le temps du rendu.
        let files = fonts
            .iter()
            .map(|id| self.fonts.get(id).cloned().ok_or_else(|| setup(format!("unknown font {id}"))))
            .collect::<Result<Vec<_>, _>>()?;
        let witness = Office2Pdf;
        let span = slides.map(|(first, last)| SlideSpan { first, last });
        let pdf = witness.render(pptx, files, span).map_err(|e| reference_error(&e))?;
        let pages = pdf.page_count();
        self.next_generation += 1;
        let reference = self.next_generation;
        self.references.insert(reference, pdf);
        Ok((Response::Reference { reference, witness: witness.witness().to_owned(), pages }, Vec::new()))
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

    fn probe_file(&mut self, name: &str, source: &str, common: Option<&str>) -> Result<Reply, WireError> {
        let name = WidgetFile::parse(name).ok_or_else(|| setup(format!("bad widget file {name}")))?;
        let probe = self.registries().probe_file(&name, source, common);
        Ok((Response::FileProbe { probe }, Vec::new()))
    }

    fn set_language(&mut self, locale: &str) -> Result<Reply, WireError> {
        let localizer = Localizer::new(Some(locale));
        for slot in self.slots.values_mut() {
            slot.renderer.set_language(&localizer).map_err(|e| render_error(&e))?;
        }
        self.localizer = Some(localizer);
        ok()
    }

    fn open_slot(&mut self, slot: SlotId, fonts: &[String]) -> Result<Reply, WireError> {
        let mut renderer = Renderer::new(self.fonts_of(fonts)?).map_err(|e| render_error(&e))?;
        if let Some(l) = &self.localizer {
            renderer.set_language(l).map_err(|e| render_error(&e))?;
        }
        self.slots.insert(slot, Slot { renderer, generations: BTreeMap::new(), last_assets: Vec::new() });
        ok()
    }

    fn set_widgets(&mut self, slot: SlotId, pack: Option<WirePack>) -> Result<Reply, WireError> {
        let pack = pack.map(pack_of).transpose()?;
        let registry = self.registries().get(pack.as_ref()).map_err(|e| WireError::from_registry(&e))?;
        let s = self.slots.get_mut(&slot).ok_or_else(|| setup(format!("unknown slot {slot}")))?;
        s.renderer.set_widgets(pack.as_ref(), registry).map_err(|e| render_error(&e))?;
        ok()
    }

    fn set_theme(&mut self, slot: SlotId, tokens: Option<Map<String, Value>>) -> Result<Reply, WireError> {
        let s = self.slots.get_mut(&slot).ok_or_else(|| setup(format!("unknown slot {slot}")))?;
        s.renderer.set_theme(tokens).map_err(|e| render_error(&e))?;
        ok()
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
}

/// Les arguments du travailleur (`--render-worker` en premier), sinon `None`.
///
/// `--max-frame-bytes <n>`, `--timeout-ms <n>` et `--memory-bytes <n>` sont
/// facultatifs : sans eux, taille par défaut et pas de borne.
pub(crate) fn worker_args(args: &[OsString]) -> Option<WorkerArgs> {
    if args.first().map(|a| a == "--render-worker") != Some(true) {
        return None;
    }
    let value = |name: &str| -> Option<u64> {
        let i = args.iter().position(|a| a == name)?;
        args.get(i + 1)?.to_str()?.parse().ok()
    };
    let max_frame = value("--max-frame-bytes").and_then(|n| usize::try_from(n).ok()).unwrap_or(DEFAULT_MAX_FRAME);
    Some(WorkerArgs { max_frame, limits: Limits::from_host(value("--timeout-ms"), value("--memory-bytes")) })
}

/// Si ce processus a été lancé en travailleur (`--render-worker
/// --max-frame-bytes <n> --timeout-ms <n> --memory-bytes <n>`), sert
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
    let code = match serve_watched(std::io::stdin().lock(), output, args.max_frame, &busy) {
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
