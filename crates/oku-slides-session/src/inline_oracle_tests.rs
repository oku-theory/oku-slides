//! Oracle : l'ancienne normalisation des bords recalculait `steps` à chaque
//! correctif (~O(n³) en runs). La version linéaire doit rendre la même chose.

use oku_slides_core::Run;
use speculoos::prelude::*;

use super::inline_tests::{ALPHABET, Rng};
use super::*;

/// Ancienne version de `runs_to_markdown`, gardée telle quelle.
fn old_runs_to_markdown(runs: &[Run]) -> String {
    let mut runs = runs.to_vec();
    merge(&mut runs);
    while let Some(fix) = violation(&runs) {
        // Invariant : si rien ne bouge, `violation` rendrait le même correctif à l'infini.
        if !apply(&mut runs, fix) {
            break;
        }
    }
    let steps = old_steps(&runs);
    let mut out = String::new();
    let mut after_highlight = false;
    for (i, r) in runs.iter().enumerate() {
        let step = &steps[i];
        for m in step.close.iter().chain(&step.open) {
            out.push_str(m.delimiter());
        }
        if !step.open.is_empty() {
            after_highlight = false;
        }
        if barrier(r) {
            out.push_str(&write_barrier(r, runs.get(i + 1)));
            after_highlight = r.link.is_none();
        } else {
            let text = escape_inline(&r.t);
            // `{n}` juste après une pastille serait lu comme sa couleur.
            if after_highlight && text.starts_with('{') {
                out.push('\\');
            }
            out.push_str(&text);
            after_highlight = false;
        }
    }
    for m in &steps[runs.len()].close {
        out.push_str(m.delimiter());
    }
    out
}
fn old_steps(runs: &[Run]) -> Vec<Step> {
    let mut stack: Vec<Mark> = Vec::new();
    let mut out = Vec::with_capacity(runs.len() + 1);
    for (i, r) in runs.iter().enumerate() {
        let mut step = Step::default();
        if let Some(depth) = stack.iter().position(|&m| barrier(r) || !m.of(r)) {
            step.close = stack.drain(depth..).rev().collect();
        }
        step.kept = stack.clone();
        if !barrier(r) {
            // La plus longue à l'extérieur : elle reste ouverte le plus longtemps.
            let span = |m: Mark| runs[i..].iter().take_while(|r| !barrier(r) && m.of(r)).count();
            let mut open: Vec<Mark> = Mark::ALL.into_iter().filter(|&m| m.of(r) && !stack.contains(&m)).collect();
            open.sort_by_key(|&m| std::cmp::Reverse(span(m)));
            stack.extend(&open);
            step.open = open;
        }
        out.push(step);
    }
    out.push(Step { close: stack.into_iter().rev().collect(), ..Step::default() });
    out
}
/// Caractère à sortir de ses délimiteurs : premier ou dernier du run `run`.
struct Fix {
    run: usize,
    first: bool,
    /// Emphases restées ouvertes autour : le caractère sorti les garde.
    kept: Vec<Mark>,
}

/// Premier caractère de bord que CommonMark ne lirait pas comme écrit.
fn violation(runs: &[Run]) -> Option<Fix> {
    // Pastille hors lien : pas d'espace contre `==` (voir markdownHighlight.ts).
    for (i, r) in runs.iter().enumerate() {
        if r.highlight.is_some() && r.link.is_none() {
            if class(r.t.chars().next()) == Class::Space {
                return Some(Fix { run: i, first: true, kept: Vec::new() });
            }
            if class(r.t.chars().last()) == Class::Space {
                return Some(Fix { run: i, first: false, kept: Vec::new() });
            }
        }
    }
    for (k, step) in old_steps(runs).into_iter().enumerate() {
        // Crochets, `==` et `{n}` des barrières comptent comme de la ponctuation.
        let a = match k.checked_sub(1).map(|j| &runs[j]) {
            Some(r) if barrier(r) => Class::Punct,
            Some(r) => class(r.t.chars().last()),
            None => Class::Space,
        };
        let b = match runs.get(k) {
            Some(r) if barrier(r) => Class::Punct,
            Some(r) => class(r.t.chars().next()),
            None => Class::Space,
        };
        // Fermetures et ouvertures sont des `*` : une seule suite entre `a` et `b`,
        // qui doit pouvoir fermer (côté `a`) et ouvrir (côté `b`).
        if !step.close.is_empty() && (a == Class::Space || (a == Class::Punct && b == Class::Alnum)) {
            return Some(Fix { run: k - 1, first: false, kept: step.kept });
        }
        if !step.open.is_empty() && (b == Class::Space || (b == Class::Punct && a == Class::Alnum)) {
            return Some(Fix { run: k, first: true, kept: step.kept });
        }
    }
    None
}

/// Sort le caractère : il devient un run avec les seules emphases `kept`.
/// Chaque déplacement retire de la mise en forme, la normalisation termine.
fn apply(runs: &mut Vec<Run>, fix: Fix) -> bool {
    let Fix { run, first, kept } = fix;
    let r = &mut runs[run];
    let c = if first { r.t.chars().next() } else { r.t.chars().last() };
    let Some(c) = c else { return false };
    if first {
        r.t.drain(..c.len_utf8());
    } else {
        r.t.pop();
    }
    let moved = Run { bold: kept.contains(&Mark::Bold), italic: kept.contains(&Mark::Italic), ..Run::plain(c) };
    runs.insert(if first { run } else { run + 1 }, moved);
    merge(runs);
    true
}

/// Runs voisins de même format fusionnés, runs vides retirés.
fn merge(runs: &mut Vec<Run>) {
    let same =
        |a: &Run, b: &Run| a.bold == b.bold && a.italic == b.italic && a.link == b.link && a.highlight == b.highlight;
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for r in runs.drain(..).filter(|r| !r.t.is_empty()) {
        match out.last_mut() {
            Some(last) if same(last, &r) => last.t.push_str(&r.t),
            _ => out.push(r),
        }
    }
    *runs = out;
}

/// Paragraphes très découpés : runs courts, bords chargés d'espaces et de ponctuation.
fn fragmented(rng: &mut Rng, runs: usize) -> Vec<Run> {
    const EDGES: &[char] = &[' ', ' ', '\t', '\u{feff}', '!', '"', '*', '-', '.', 'Ⓐ', 'a', 'é', '1', '😀', '\u{85}'];
    (0..runs)
        .map(|_| {
            let t: String = (0..1 + rng.below(4))
                .map(|_| {
                    if rng.below(2) == 0 { EDGES[rng.below(EDGES.len())] } else { ALPHABET[rng.below(ALPHABET.len())] }
                })
                .collect();
            Run {
                bold: rng.below(2) == 0,
                italic: rng.below(2) == 0,
                link: (rng.below(8) == 0).then(|| ["https://a.b/x", "https://a.b/y"][rng.below(2)].to_owned()),
                highlight: (rng.below(6) == 0).then(|| 1 + rng.below(3) as u8),
                ..Run::plain(t)
            }
        })
        .collect()
}

/// Ce que l'écriture lit d'un run : texte, gras, italique, lien, pastille.
type Shape = (String, bool, bool, Option<String>, Option<u8>);

/// Ce que l'écriture lit de chaque run.
fn shape(runs: &[Run]) -> Vec<Shape> {
    runs.iter().map(|r| (r.t.clone(), r.bold, r.italic, r.link.clone(), r.highlight)).collect()
}

#[test]
fn runs_to_markdown_generated_paragraphs_match_old_algorithm() {
    let mut rng = Rng(217);
    for case in 0..20_000 {
        let size = 1 + rng.below(if case % 10 == 0 { 60 } else { 12 });
        let runs = fragmented(&mut rng, size);
        asserting(&format!("{runs:?}")).that(&runs_to_markdown(&runs)).is_equal_to(old_runs_to_markdown(&runs));
    }
}

#[test]
fn normalize_generated_paragraphs_match_old_runs() {
    let mut rng = Rng(1217);
    for _ in 0..5_000 {
        let size = 1 + rng.below(20);
        let runs = fragmented(&mut rng, size);
        let mut old = runs.clone();
        merge(&mut old);
        while let Some(fix) = violation(&old) {
            if !apply(&mut old, fix) {
                break;
            }
        }
        asserting(&format!("{runs:?}")).that(&shape(&normalize(&runs))).is_equal_to(shape(&old));
    }
}

#[test]
fn normalize_five_thousand_runs_is_fast_and_keeps_the_text() {
    let runs = fragmented(&mut Rng(5000), 5_000);
    let start = std::time::Instant::now();
    let normalized = normalize(&runs);
    let markdown = runs_to_markdown(&runs);
    // L'ancien algorithme y passait des minutes ; quelques millisecondes suffisent.
    asserting("borne large, même sous charge").that(&start.elapsed().as_secs()).is_less_than(5);
    let text = |runs: &[Run]| runs.iter().map(|r| r.t.as_str()).collect::<String>();
    asserting("aucun texte perdu").that(&text(&normalized)).is_equal_to(text(&runs));
    asserting("markdown écrit").that(&markdown.is_empty()).is_false();
}
