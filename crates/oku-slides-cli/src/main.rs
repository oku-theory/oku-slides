//! `oku-slides` : outil en ligne de commande pour rendre, enregistrer et rouvrir
//! des decks Oku Slides sans l'app. Sert aussi aux tests et aux mesures.
//!
//! Les résultats vont sur stdout ([`out`]), les journaux sur stderr ; l'aide
//! et les messages sont dans la langue des réglages.

mod bench;
mod guard;
mod out;
mod pack_render;
mod serve;
mod sse;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use color_eyre::eyre::{Result, WrapErr};

use oku_slides_core::{Deck, PackFiles, PackVersion, SourceBundle, SourceLimits, WidgetRegistry, validate};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_session::{Library, Storage, adopt_default_pack, read_document};

use crate::bench::ms;
use crate::out::Out;

// L'aide de chaque commande et option vient des messages Fluent
// (`cli-cmd-…`, `cli-arg-…`) : voir `command`.
#[derive(Parser)]
#[command(name = "oku-slides", version)]
struct Cli {
    #[arg(long, global = true)]
    fonts: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Check {
        deck: PathBuf,
    },
    Render {
        input: PathBuf,
        #[arg(long, default_value = "out")]
        out: PathBuf,
        #[arg(long)]
        theme: Option<String>,
    },
    Save {
        deck: PathBuf,
        output: PathBuf,
        #[arg(long)]
        assets: Option<PathBuf>,
        #[arg(long)]
        embed_fonts: bool,
        #[arg(long)]
        theme: Option<String>,
    },
    Open {
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Strip {
        input: PathBuf,
        output: PathBuf,
    },
    Bench {
        deck: PathBuf,
        #[arg(long, default_value_t = 5)]
        iterations: usize,
    },
    Schema,
    Serve {
        input: Option<PathBuf>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long, default_value = "ui/dist")]
        ui: PathBuf,
    },
}

/// La ligne de commande, avec l'aide dans la langue de l'utilisateur.
fn command(l: &Localizer) -> clap::Command {
    let help = |id: &str| l.tr(id);
    Cli::command()
        .about(help("cli-about"))
        .mut_arg("fonts", |a| a.help(help("cli-arg-fonts")))
        .mut_subcommand("check", |c| c.about(help("cli-cmd-check")).mut_arg("deck", |a| a.help(help("cli-arg-deck"))))
        .mut_subcommand("render", |c| {
            c.about(help("cli-cmd-render"))
                .mut_arg("input", |a| a.help(help("cli-arg-input")))
                .mut_arg("out", |a| a.help(help("cli-arg-out")))
                .mut_arg("theme", |a| a.help(help("cli-arg-theme")))
        })
        .mut_subcommand("save", |c| {
            c.about(help("cli-cmd-save"))
                .mut_arg("deck", |a| a.help(help("cli-arg-deck")))
                .mut_arg("output", |a| a.help(help("cli-arg-output")))
                .mut_arg("assets", |a| a.help(help("cli-arg-assets")))
                .mut_arg("embed_fonts", |a| a.help(help("cli-arg-embed-fonts")))
                .mut_arg("theme", |a| a.help(help("cli-arg-theme")))
        })
        .mut_subcommand("open", |c| {
            c.about(help("cli-cmd-open"))
                .mut_arg("input", |a| a.help(help("cli-arg-input")))
                .mut_arg("out", |a| a.help(help("cli-arg-extract")))
        })
        .mut_subcommand("strip", |c| {
            c.about(help("cli-cmd-strip"))
                .mut_arg("input", |a| a.help(help("cli-arg-input")))
                .mut_arg("output", |a| a.help(help("cli-arg-output")))
        })
        .mut_subcommand("bench", |c| {
            c.about(help("cli-cmd-bench"))
                .mut_arg("deck", |a| a.help(help("cli-arg-deck")))
                .mut_arg("iterations", |a| a.help(help("cli-arg-iterations")))
        })
        .mut_subcommand("schema", |c| c.about(help("cli-cmd-schema")))
        .mut_subcommand("serve", |c| {
            c.about(help("cli-cmd-serve"))
                .mut_arg("input", |a| a.help(help("cli-arg-serve-input")))
                .mut_arg("port", |a| a.help(help("cli-arg-port")))
                .mut_arg("ui", |a| a.help(help("cli-arg-ui")))
        })
}

fn read_deck(storage: &Storage, l: &Localizer, path: &Path) -> Result<Deck> {
    let shown = || path.display().to_string();
    let bytes = storage.read(path).wrap_err_with(|| l.tr_with("cli-reading", &[("path", shown().into())]))?;
    serde_json::from_slice(&bytes).wrap_err_with(|| l.tr_with("cli-invalid-deck", &[("path", shown().into())]))
}

/// Charge un deck depuis un JSON ou un .oku-slides.pdf ; `limits` borne les
/// sources lues dans le PDF.
fn load_bundle(
    storage: &Storage,
    l: &Localizer,
    path: &Path,
    limits: SourceLimits,
    registry: &Arc<WidgetRegistry>,
) -> Result<SourceBundle> {
    let opened = read_document(storage, path, limits, registry)
        .wrap_err_with(|| l.tr_with("cli-reading", &[("path", path.display().to_string().into())]))?;
    if opened.modified_elsewhere {
        tracing::warn!(path = %path.display(), "{}", l.tr("cli-pdf-modified"));
    }
    for issue in &opened.issues {
        tracing::warn!("{}", l.tr_with("cli-deck-issue", &[("issue", issue.localize(l).into())]));
    }
    Ok(opened.sources)
}

/// Signale les problèmes du deck ; vrai s'il n'y en a pas.
fn report_issues(l: &Localizer, deck: &Deck, registry: &WidgetRegistry, assets: &BTreeMap<String, Vec<u8>>) -> bool {
    let ids = assets.keys().cloned().collect();
    let issues = validate(deck, registry, Some(&ids));
    for issue in &issues {
        tracing::warn!("{}", l.tr_with("cli-deck-issue", &[("issue", issue.localize(l).into())]));
    }
    issues.is_empty()
}

/// Le pack que `bundle` embarque : la version que son deck référence, ou
/// la courante du pack par défaut (alors référencée).
///
/// La version vient d'abord de `<deck_dir>/packs/<hex>` (extrait par
/// `open --out`, vérifié), puis de la bibliothèque ; introuvable : erreur.
fn embed_pack(
    storage: &Storage,
    library: &Library,
    l: &Localizer,
    deck_dir: &Path,
    bundle: &mut SourceBundle,
) -> Result<()> {
    let Some(r) = bundle.deck().templates.clone() else {
        return adopt_default_pack(library, bundle).map_err(|e| color_eyre::eyre::eyre!(e.localize(l)));
    };
    let files = beside_deck(storage, deck_dir, &r.version).or_else(|| library.version(r.id, &r.version));
    let Some(files) = files else {
        let args = [("id", r.id.to_string().into()), ("version", r.version.as_str().into())];
        return Err(color_eyre::eyre::eyre!(l.tr_with("session-pack-version-missing", &args)));
    };
    bundle.add_pack(files);
    Ok(())
}

/// La version `v` extraite à côté du deck, si elle s'y trouve intacte.
fn beside_deck(storage: &Storage, deck_dir: &Path, v: &PackVersion) -> Option<PackFiles> {
    let dir = deck_dir.join("packs").join(v.hex());
    let read = |name: &str| -> Option<Vec<u8>> {
        let path = dir.join(name);
        match storage.len(&path) {
            Ok(Some(_)) => storage
                .read(&path)
                .inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "fichier du pack illisible"))
                .ok(),
            _ => None,
        }
    };
    let files = PackFiles::from_bytes(&read("pack.json")?, read("widgets.typ").as_deref())?;
    if files.matches(v) {
        Some(files)
    } else {
        tracing::warn!(dir = %dir.display(), "pack altéré à côté du deck : ignoré");
        None
    }
}

/// Registre de base des widgets, compilé depuis Typst une fois par processus
/// et partagé avec le rendu en mémoire ; le pack d'un deck passe par
/// [`pack_render`].
fn registry(fonts: &Fonts) -> Result<Arc<WidgetRegistry>> {
    Ok(oku_slides_render::Registries::new(fonts.clone()).get(None)?)
}

fn main() -> Result<()> {
    // Relancé en processus de rendu (`--render-worker`) : sert le tube, puis termine.
    oku_slides_render_worker::exit_if_worker();
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    let platform = Platform::load()?;
    let l: &Localizer = &platform.localizer;
    let cli = Cli::from_arg_matches(&command(l).get_matches())?;
    let mut out = Out::new(l);
    match run(cli, &platform, &mut out) {
        // Un tube fermé (`oku-slides schema | head`) n'est pas une erreur.
        Err(e) if e.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe) => {
            Ok(())
        }
        result => result,
    }
}

fn run(cli: Cli, platform: &Platform, out: &mut Out<'_>) -> Result<()> {
    let (storage, l) = (&platform.storage, out.localizer());
    let fonts = match cli.fonts.as_ref().or(platform.settings.paths.fonts.as_ref()) {
        Some(dir) => storage
            .read_fonts(dir)
            .wrap_err_with(|| l.tr_with("cli-reading", &[("path", dir.display().to_string().into())]))?,
        None => BTreeMap::new(),
    };
    let app_fonts = Fonts::new(fonts.values().cloned());
    let limits = SourceLimits {
        replica_bytes: platform.settings.sources.max_replica_bytes(),
        total_bytes: platform.settings.sources.max_sources_bytes(),
    };

    match cli.cmd {
        Cmd::Check { deck } => {
            let registry = registry(&app_fonts)?;
            let deck = read_deck(storage, l, &deck)?;
            if report_issues(l, &deck, &registry, &BTreeMap::new()) {
                out.line("cli-deck-valid", &[("slides", deck.slides.len().into())])?;
            }
        }
        Cmd::Render { input, out: dir, theme } => {
            let registry = registry(&app_fonts)?;
            let bundle = load_bundle(storage, l, &input, limits, &registry)?;
            let mut deck = bundle.deck().clone();
            if theme.is_some() {
                deck.theme = theme;
            }
            let t = Instant::now();
            let doc = pack_render::render(platform, l, app_fonts.with(bundle.fonts.values().cloned()), &bundle, &deck)?;
            let compile = t.elapsed();
            for w in doc.warnings() {
                tracing::warn!("{}", w.localize(l));
            }
            let t = Instant::now();
            for i in 0..doc.page_count() {
                storage.write(&dir.join(format!("page-{:02}.svg", i + 1)), doc.svg(i)?)?;
            }
            let svg = t.elapsed();
            storage.write(&dir.join("layout.json"), serde_json::to_string_pretty(doc.layout())?)?;
            out.line(
                "cli-rendered",
                &[
                    ("pages", doc.page_count().into()),
                    ("out", dir.display().to_string().into()),
                    ("compile", ms(compile).into()),
                    ("svg", ms(svg).into()),
                ],
            )?;
        }
        Cmd::Save { deck, output, assets, embed_fonts, theme } => {
            let registry = registry(&app_fonts)?;
            let deck_dir = deck.parent().map(Path::to_path_buf).unwrap_or_default();
            let mut deck = read_deck(storage, l, &deck)?;
            if theme.is_some() {
                deck.theme = theme;
            }
            let mut bundle = SourceBundle::new(deck, registry.clone())?;
            let library = Library::new(Some(platform.settings.paths.templates()), storage.clone());
            embed_pack(storage, &library, l, &deck_dir, &mut bundle)?;
            if let Some(dir) = assets {
                for path in storage.list_files(&dir)? {
                    let id = bundle.add_asset(storage.read(&path)?);
                    out.line("cli-asset-added", &[("path", path.display().to_string().into()), ("id", id.into())])?;
                }
            }
            if embed_fonts {
                bundle.fonts = fonts;
            }
            report_issues(l, bundle.deck(), &registry, &bundle.assets);
            let doc = pack_render::render(platform, l, app_fonts.clone(), &bundle, bundle.deck())?;
            let pdf = doc.pdf()?;
            let zip = bundle.to_zip()?;
            let written = oku_slides_pdf::embed_sources(&pdf, &zip)?;
            let size = written.len();
            storage.write_atomic(&output, written)?;
            out.line(
                "cli-saved",
                &[
                    ("path", output.display().to_string().into()),
                    ("pages", doc.page_count().into()),
                    ("pdf", (size / 1024).into()),
                    ("sources", (zip.len() / 1024).into()),
                ],
            )?;
        }
        Cmd::Open { input, out: extract } => {
            let registry = registry(&app_fonts)?;
            let bundle = load_bundle(storage, l, &input, limits, &registry)?;
            let deck = bundle.deck();
            out.line(
                "cli-opened",
                &[
                    ("path", input.display().to_string().into()),
                    ("slides", deck.slides.len().into()),
                    ("assets", bundle.assets.len().into()),
                    ("fonts", bundle.fonts.len().into()),
                ],
            )?;
            if let Some(dir) = extract {
                storage.write(&dir.join("deck.json"), deck.to_json()?)?;
                for (id, bytes) in &bundle.assets {
                    storage.write(&dir.join("assets").join(id.trim_start_matches("sha256:")), bytes.as_slice())?;
                }
                for (name, bytes) in &bundle.fonts {
                    storage.write(&dir.join("fonts").join(name), bytes.as_slice())?;
                }
                for (version, files) in &bundle.packs {
                    let at = dir.join("packs").join(version.hex());
                    storage.write(&at.join("pack.json"), files.pack_json()?)?;
                    if let Some(source) = files.widgets() {
                        storage.write(&at.join("widgets.typ"), source.as_bytes())?;
                    }
                }
                out.line("cli-extracted", &[("dir", dir.display().to_string().into())])?;
            }
        }
        Cmd::Strip { input, output } => {
            let pdf = storage.read(&input)?;
            storage.write_atomic(&output, oku_slides_pdf::strip_sources(&pdf)?)?;
            out.line("cli-stripped", &[("path", output.display().to_string().into())])?;
        }
        Cmd::Bench { deck, iterations } => {
            bench::bench(out, read_deck(storage, l, &deck)?, app_fonts, iterations, limits)?
        }
        Cmd::Schema => out.data(&serde_json::to_string_pretty(&oku_slides_core::ops_schema()?)?)?,
        Cmd::Serve { input, port, ui } => serve::serve(out, input, port, &ui, app_fonts, platform)?,
    }
    Ok(())
}
