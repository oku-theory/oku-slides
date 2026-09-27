//! Runs → markdown inline d'édition, relu par `@lezer/markdown` (TS).
//!
//! Les emphases partagées par des runs voisins restent ouvertes (pile, la
//! plus longue à l'extérieur) ; pastilles et liens portent leurs propres
//! délimiteurs. Avant d'écrire, les bords que CommonMark ne lirait pas
//! comme écrits sortent de leurs délimiteurs : seule de la mise en forme
//! sur des espaces et de la ponctuation peut s'y perdre, jamais du texte.
//! Contrat : `fixtures/markdown.json` et `fixtures/markdown-corpus.json`.

use oku_slides_core::Run;

/// Runs en markdown inline ; `color` et `size` n'ont pas d'écriture markdown.
pub(crate) fn runs_to_markdown(runs: &[Run]) -> String {
    let mut runs = runs.to_vec();
    merge(&mut runs);
    while let Some(fix) = violation(&runs) {
        // Invariant : si rien ne bouge, `violation` rendrait le même correctif à l'infini.
        if !apply(&mut runs, fix) {
            break;
        }
    }
    let steps = steps(&runs);
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

/// Emphase écrite avec `*` : gras ou italique.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mark {
    Bold,
    Italic,
}

impl Mark {
    const ALL: [Mark; 2] = [Mark::Bold, Mark::Italic];

    fn delimiter(self) -> &'static str {
        match self {
            Mark::Bold => "**",
            Mark::Italic => "*",
        }
    }

    fn of(self, r: &Run) -> bool {
        match self {
            Mark::Bold => r.bold,
            Mark::Italic => r.italic,
        }
    }
}

/// Une pastille ou un lien a ses propres délimiteurs : l'emphase ne le traverse pas.
fn barrier(r: &Run) -> bool {
    r.link.is_some() || r.highlight.is_some()
}

/// Emphases fermées puis ouvertes avant un run.
#[derive(Default, Debug)]
struct Step {
    close: Vec<Mark>,
    /// Emphases encore ouvertes entre les fermetures et les ouvertures.
    kept: Vec<Mark>,
    open: Vec<Mark>,
}

/// Transitions de la pile : `steps[i]` précède le run `i`, `steps[n]` clôt la ligne.
fn steps(runs: &[Run]) -> Vec<Step> {
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

/// Nature d'un caractère pour les règles de délimiteurs de CommonMark.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Espace, ou bord de ligne.
    Space,
    Punct,
    Alnum,
}

/// Espace au sens du `\s` de JS (Lezer) : les blancs Unicode, plus U+FEFF (BOM).
fn is_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn class(c: Option<char>) -> Class {
    match c {
        None => Class::Space,
        Some(c) if is_space(c) => Class::Space,
        Some(c) if c.is_alphanumeric() => Class::Alnum,
        Some(_) => Class::Punct,
    }
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
    for (k, step) in steps(runs).into_iter().enumerate() {
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

/// Pastille ou lien, avec ses propres emphases à l'intérieur.
fn write_barrier(r: &Run, next: Option<&Run>) -> String {
    // Dans un lien, les espaces de bord restent dans les crochets, hors délimiteurs.
    let core = r.t.trim_matches(is_space);
    let start = r.t.len() - r.t.trim_start_matches(is_space).len();
    let (lead, trail) = (&r.t[..start], &r.t[start + core.len()..]);
    let marks = match (r.bold, r.italic) {
        (true, true) => "***",
        (true, false) => "**",
        (false, true) => "*",
        (false, false) => "",
    };
    let mut body = if core.is_empty() { String::new() } else { format!("{marks}{}{marks}", escape_inline(core)) };
    if let Some(n) = r.highlight.filter(|_| !core.is_empty()) {
        // Une pastille collée à la suivante garde sa couleur, même `{1}` :
        // sinon `==a====b==` se lirait comme une seule pastille.
        let before_highlight = r.link.is_none() && next.is_some_and(|r| r.highlight.is_some());
        let color = if n == 1 && !before_highlight { String::new() } else { format!("{{{n}}}") };
        body = format!("=={body}=={color}");
    }
    let body = format!("{}{body}{}", escape_inline(lead), escape_inline(trail));
    match &r.link {
        Some(url) => {
            let url = url.replace(' ', "%20").replace('(', "%28").replace(')', "%29");
            format!("[{body}]({url})")
        }
        None => body,
    }
}

/// Échappe ce qui se lirait comme du markdown ; `_` au milieu d'un mot et
/// `=` isolé restent tels quels.
fn escape_inline(t: &str) -> String {
    let chars: Vec<char> = t.chars().collect();
    let mut out = String::with_capacity(t.len());
    for (i, &c) in chars.iter().enumerate() {
        let intraword = || {
            let alnum = |j: Option<usize>| j.and_then(|j| chars.get(j)).is_some_and(|c| c.is_alphanumeric());
            alnum(i.checked_sub(1)) && alnum(Some(i + 1))
        };
        // `=` contre un autre `=` ou au bord du run pourrait former `==`.
        let equals = || {
            [i.checked_sub(1), Some(i + 1)].into_iter().any(|j| j.and_then(|j| chars.get(j)).is_none_or(|&n| n == '='))
        };
        if matches!(c, '\\' | '*' | '[' | ']') || (c == '_' && !intraword()) || (c == '=' && equals()) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
#[path = "inline_tests.rs"]
mod inline_tests;
