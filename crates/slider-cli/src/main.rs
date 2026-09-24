//! `slider` : outil en ligne de commande pour rendre, enregistrer et rouvrir
//! des decks Slider sans l'app. Sert aussi aux tests et aux mesures.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use slider_core::{Deck, Op, Paragraph, SourceBundle, validate};
use slider_render::Renderer;

#[derive(Parser)]
#[command(name = "slider", version, about = "Rendu et stockage des decks Slider")]
struct Cli {
    /// Dossier de polices supplémentaires (TTF/OTF).
    #[arg(long, global = true)]
    fonts: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Vérifie un deck.
    Check { deck: PathBuf },
    /// Rend chaque slide en SVG, avec les boîtes des éléments.
    Render {
        /// deck.json ou fichier .slider.pdf
        input: PathBuf,
        #[arg(long, default_value = "out")]
        out: PathBuf,
        /// Remplace le thème du deck.
        #[arg(long)]
        theme: Option<String>,
    },
    /// Enregistre un deck en .slider.pdf (PDF + blob des sources).
    Save {
        deck: PathBuf,
        output: PathBuf,
        /// Dossier d'assets : chaque fichier devient un asset sha256:…
        #[arg(long)]
        assets: Option<PathBuf>,
        /// Joint aussi les polices de --fonts aux sources.
        #[arg(long)]
        embed_fonts: bool,
        #[arg(long)]
        theme: Option<String>,
    },
    /// Rouvre un .slider.pdf : extrait le deck et les assets.
    Open {
        input: PathBuf,
        /// Dossier où écrire deck.json, assets/ et fonts/.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Copie d'un .slider.pdf, sans le blob des sources.
    Strip { input: PathBuf, output: PathBuf },
    /// Mesure les temps de compilation et d'export.
    Bench {
        deck: PathBuf,
        #[arg(long, default_value_t = 5)]
        iterations: usize,
    },
    /// Affiche le schéma JSON des opérations.
    Schema,
}

fn load_fonts(dir: Option<&Path>) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut out = BTreeMap::new();
    let Some(dir) = dir else { return Ok(out) };
    for entry in std::fs::read_dir(dir).with_context(|| format!("lecture de {}", dir.display()))? {
        let path = entry?.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ["ttf", "otf", "ttc", "otc"].contains(&ext.as_str()) {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            out.insert(name, std::fs::read(&path)?);
        }
    }
    Ok(out)
}

fn read_deck(path: &Path) -> Result<Deck> {
    let text = std::fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
    Deck::from_json(&text).with_context(|| format!("deck invalide : {}", path.display()))
}

/// Charge un deck depuis un JSON ou un .slider.pdf.
fn load_bundle(path: &Path) -> Result<SourceBundle> {
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
        let pdf = std::fs::read(path)?;
        let sources = slider_pdf::extract_sources(&pdf)?
            .context("ce PDF n'a pas été créé par Slider, il ne peut pas être modifié")?;
        if sources.modified_elsewhere() {
            eprintln!("attention : le PDF a été modifié par un autre outil après Slider");
        }
        Ok(SourceBundle::from_zip(&sources.zip)?)
    } else {
        Ok(SourceBundle::new(read_deck(path)?))
    }
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

fn report_issues(deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> bool {
    let ids = assets.keys().cloned().collect();
    let issues = validate(deck, Some(&ids));
    for i in &issues {
        eprintln!("problème : {i}");
    }
    issues.is_empty()
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let fonts = load_fonts(cli.fonts.as_deref())?;
    let font_data = || fonts.values().cloned().collect::<Vec<_>>();

    match cli.cmd {
        Cmd::Check { deck } => {
            let deck = read_deck(&deck)?;
            if report_issues(&deck, &BTreeMap::new()) {
                println!("deck valide : {} slides", deck.slides.len());
            }
        }
        Cmd::Render { input, out, theme } => {
            let mut bundle = load_bundle(&input)?;
            let deck = bundle.deck.as_mut().unwrap();
            if theme.is_some() {
                deck.theme = theme;
            }
            let mut all_fonts = font_data();
            all_fonts.extend(bundle.fonts.values().cloned());
            let mut r = Renderer::new(all_fonts);
            r.set_deck(deck, &bundle.assets);
            let t = Instant::now();
            let doc = r.compile()?;
            let compile = t.elapsed();
            for w in &doc.warnings {
                eprintln!("{w}");
            }
            std::fs::create_dir_all(&out)?;
            let t = Instant::now();
            for i in 0..doc.page_count() {
                std::fs::write(out.join(format!("page-{:02}.svg", i + 1)), doc.svg(i)?)?;
            }
            let svg = t.elapsed();
            let layout = serde_json::json!({ "slides": doc.slide_pages(), "boxes": doc.element_boxes() });
            std::fs::write(out.join("layout.json"), serde_json::to_string_pretty(&layout)?)?;
            println!("{} pages → {} (compilation {}, SVG {})", doc.page_count(), out.display(), ms(compile), ms(svg));
        }
        Cmd::Save { deck, output, assets, embed_fonts, theme } => {
            let mut deck = read_deck(&deck)?;
            if theme.is_some() {
                deck.theme = theme;
            }
            let mut bundle = SourceBundle::new(deck.clone());
            if let Some(dir) = assets {
                for entry in std::fs::read_dir(&dir)? {
                    let path = entry?.path();
                    if path.is_file() {
                        let id = bundle.add_asset(std::fs::read(&path)?);
                        println!("asset {} → {id}", path.display());
                    }
                }
            }
            if embed_fonts {
                bundle.fonts = fonts.clone();
            }
            report_issues(&deck, &bundle.assets);
            let mut r = Renderer::new(font_data());
            r.set_deck(&deck, &bundle.assets);
            let doc = r.compile()?;
            let pdf = doc.pdf()?;
            let zip = bundle.to_zip()?;
            let out = slider_pdf::embed_sources(&pdf, &zip)?;
            write_atomic(&output, &out)?;
            println!(
                "{} : {} pages, PDF {} Ko, sources {} Ko",
                output.display(),
                doc.page_count(),
                out.len() / 1024,
                zip.len() / 1024
            );
        }
        Cmd::Open { input, out } => {
            let bundle = load_bundle(&input)?;
            let deck = bundle.deck.as_ref().unwrap();
            println!(
                "{} : {} slides, {} assets, {} polices, historique : {}",
                input.display(),
                deck.slides.len(),
                bundle.assets.len(),
                bundle.fonts.len(),
                if bundle.history.is_some() { "oui" } else { "non" }
            );
            if let Some(dir) = out {
                std::fs::create_dir_all(dir.join("assets"))?;
                std::fs::write(dir.join("deck.json"), deck.to_json())?;
                for (id, bytes) in &bundle.assets {
                    std::fs::write(dir.join("assets").join(id.trim_start_matches("sha256:")), bytes)?;
                }
                if !bundle.fonts.is_empty() {
                    std::fs::create_dir_all(dir.join("fonts"))?;
                    for (name, bytes) in &bundle.fonts {
                        std::fs::write(dir.join("fonts").join(name), bytes)?;
                    }
                }
                println!("extrait dans {}", dir.display());
            }
        }
        Cmd::Strip { input, output } => {
            let pdf = std::fs::read(&input)?;
            write_atomic(&output, &slider_pdf::strip_sources(&pdf)?)?;
            println!("{} : copie sans sources", output.display());
        }
        Cmd::Bench { deck, iterations } => bench(read_deck(&deck)?, font_data(), iterations)?,
        Cmd::Schema => println!("{}", serde_json::to_string_pretty(&slider_core::ops_schema())?),
    }
    Ok(())
}

/// Écrit dans un fichier temporaire à côté, puis renomme.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let tmp = dir.join(format!(".{}.tmp", path.file_name().unwrap().to_string_lossy()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn bench(mut deck: Deck, fonts: Vec<Vec<u8>>, n: usize) -> Result<()> {
    if n == 0 {
        bail!("--iterations doit être > 0");
    }
    let assets = BTreeMap::new();
    let t = Instant::now();
    let mut r = Renderer::new(fonts);
    println!("création du monde (polices)       {}", ms(t.elapsed()));

    r.set_deck(&deck, &assets);
    let t = Instant::now();
    let doc = r.compile()?;
    println!("première compilation              {}  ({} pages)", ms(t.elapsed()), doc.page_count());

    let mut same = Vec::new();
    for _ in 0..n {
        r.set_deck(&deck, &assets);
        let t = Instant::now();
        r.compile()?;
        same.push(t.elapsed());
    }
    println!("recompilation sans changement     {}", ms(median(same)));

    // Une modification de texte sur une slide de contenu, comme une dictée.
    let target = deck
        .slides
        .iter()
        .find_map(|s| s.blocks.first().map(|b| (s.id.clone(), b.id.clone())))
        .context("le deck n'a aucun bloc à modifier")?;
    let mut edits = Vec::new();
    let mut last = None;
    for i in 0..n {
        let op = Op::SetText {
            slide: target.0.clone(),
            element: target.1.clone(),
            text: vec![Paragraph::plain(format!("Texte modifié {i}"))],
        };
        op.apply(&mut deck).ok();
        r.set_deck(&deck, &assets);
        let t = Instant::now();
        last = Some(r.compile()?);
        edits.push(t.elapsed());
    }
    println!("recompilation après set_text      {}", ms(median(edits)));

    let doc = last.unwrap();
    let t = Instant::now();
    for i in 0..doc.page_count() {
        doc.svg(i)?;
    }
    let all = t.elapsed();
    println!("SVG de toutes les slides          {}  ({} par slide)", ms(all), ms(all / doc.page_count().max(1) as u32));

    let t = Instant::now();
    let pdf = doc.pdf()?;
    println!("export PDF                        {}  ({} Ko)", ms(t.elapsed()), pdf.len() / 1024);

    let zip = SourceBundle::new(deck.clone()).to_zip()?;
    let t = Instant::now();
    let out = slider_pdf::embed_sources(&pdf, &zip)?;
    println!("ajout du blob des sources         {}", ms(t.elapsed()));
    let t = Instant::now();
    let back = slider_pdf::extract_sources(&out)?.context("blob introuvable")?;
    let bundle = SourceBundle::from_zip(&back.zip)?;
    println!("relecture du blob                 {}", ms(t.elapsed()));
    if bundle.deck.as_ref() != Some(&deck) {
        bail!("le deck relu diffère du deck enregistré");
    }
    println!("aller-retour du deck              identique");
    Ok(())
}
