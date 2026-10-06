//! `oku-slides bench` : temps de rendu, d'export et d'enregistrement.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use color_eyre::eyre::{ContextCompat, Result, bail};
use oku_slides_core::{Deck, Op, Paragraph, ParamValue, SourceBundle, SourceLimits, Target};
use oku_slides_render::{Fonts, Renderer};

use crate::out::Out;

pub fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v.get(v.len() / 2).copied().unwrap_or_default()
}

/// `tokens` : les jetons du thème du deck (voir `oku_slides_session::theme_tokens`).
pub fn bench(
    out: &mut Out<'_>,
    mut deck: Deck,
    tokens: Option<serde_json::Map<String, serde_json::Value>>,
    fonts: Fonts,
    n: usize,
    limits: SourceLimits,
) -> Result<()> {
    let l = out.localizer();
    if n == 0 {
        bail!(l.tr("cli-iterations"));
    }
    let assets = BTreeMap::new();
    let t = Instant::now();
    let mut r = Renderer::new(fonts)?;
    r.set_theme(tokens)?;
    let registry = r.registry().clone();
    out.line("cli-bench-world", &[("time", ms(t.elapsed()).into())])?;

    r.set_deck(&deck, &assets)?;
    let t = Instant::now();
    let doc = r.compile()?;
    out.line("cli-bench-first", &[("time", ms(t.elapsed()).into()), ("pages", doc.page_count().into())])?;

    let mut same = Vec::new();
    for _ in 0..n {
        r.set_deck(&deck, &assets)?;
        let t = Instant::now();
        r.compile()?;
        same.push(t.elapsed());
    }
    out.line("cli-bench-same", &[("time", ms(median(same)).into())])?;

    // Une modification de texte sur une slide de contenu, comme une dictée.
    let (slide, element) = deck
        .slides
        .iter()
        .find_map(|s| s.params.slot("blocks").and_then(<[_]>::first).map(|e| (s.id.clone(), e.id.clone())))
        .with_context(|| l.tr("cli-no-block"))?;
    let mut edits = Vec::new();
    let mut last = None;
    for i in 0..n {
        let text = l.tr_with("cli-bench-edited-text", &[("n", i.into())]);
        let value = serde_json::to_value(vec![Paragraph::plain(text)])?;
        let op = Op::SetParam {
            target: Target { slide: slide.clone(), element: Some(element.clone()) },
            param: "body".into(),
            value,
        };
        op.apply(&mut deck, &registry)?;
        r.set_deck(&deck, &assets)?;
        let t = Instant::now();
        last = Some(r.compile()?);
        edits.push(t.elapsed());
    }
    out.line("cli-bench-edit", &[("time", ms(median(edits)).into())])?;

    let doc = last.with_context(|| l.tr("cli-iterations"))?;
    let t = Instant::now();
    for i in 0..doc.page_count() {
        doc.svg(i)?;
    }
    let all = t.elapsed();
    let pages = u32::try_from(doc.page_count().max(1)).unwrap_or(u32::MAX);
    out.line("cli-bench-svg", &[("time", ms(all).into()), ("each", ms(all / pages).into())])?;

    let t = Instant::now();
    let pdf = doc.pdf()?;
    out.line("cli-bench-pdf", &[("time", ms(t.elapsed()).into()), ("size", (pdf.len() / 1024).into())])?;

    let zip = SourceBundle::new(deck.clone(), registry.clone())?.to_zip()?;
    let t = Instant::now();
    let embedded = oku_slides_pdf::embed_sources(&pdf, &zip)?;
    out.line("cli-bench-embed", &[("time", ms(t.elapsed()).into())])?;
    let t = Instant::now();
    let back = oku_slides_pdf::extract_sources(&embedded)?.with_context(|| l.tr("cli-blob-missing"))?;
    let (bundle, _) = SourceBundle::from_zip(&back.zip, limits, registry.clone())?;
    out.line("cli-bench-extract", &[("time", ms(t.elapsed()).into())])?;
    // La vue relue est canonique : on compare à la forme canonique du deck.
    oku_slides_core::canonicalize(&mut deck);
    if *bundle.deck() != deck {
        bail!(l.tr("cli-roundtrip-mismatch"));
    }
    out.line("cli-bench-roundtrip", &[])?;

    // Coût d'un cran d'historique : lecture + rendu, pour décider si le
    // rendu se fait pendant le glissement du curseur ou à son relâchement.
    let mut bundle = SourceBundle::new(deck.clone(), registry.clone())?;
    for i in 0..50 {
        let mut edited = bundle.deck().clone();
        let title = (i + 1).to_string();
        if let Some(slide) = edited.slides.get_mut(2) {
            slide.params.set("title", ParamValue::Text(title.clone()));
        }
        oku_slides_core::canonicalize(&mut edited);
        bundle.apply(edited, &title, None)?;
    }
    let history = bundle.history();
    let mut steps = Vec::new();
    for i in 1..history.len() {
        let t = Instant::now();
        let (deck, _issues) = history.deck_at(i)?;
        r.set_deck(&deck, &assets)?;
        r.compile()?;
        steps.push(t.elapsed());
    }
    let sum: Duration = steps.iter().sum();
    let mean = steps.len().try_into().map(|n: u32| sum / n).unwrap_or_default();
    let ms_value = |d: Duration| format!("{:.1}", d.as_secs_f64() * 1000.0);
    out.line("cli-bench-history", &[("mean", ms_value(mean).into()), ("p50", ms_value(median(steps)).into())])?;

    Ok(())
}
