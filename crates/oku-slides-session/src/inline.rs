//! Runs → markdown inline d'édition, relu par `@lezer/markdown` (TS).
//!
//! Les emphases partagées par des runs voisins restent ouvertes (pile, la
//! plus longue à l'extérieur) ; pastilles et liens portent leurs propres
//! délimiteurs. Avant d'écrire, les bords que CommonMark ne lirait pas
//! comme écrits sortent de leurs délimiteurs : seule de la mise en forme
//! sur des espaces et de la ponctuation peut s'y perdre, jamais du texte.
//! Contrat : `fixtures/markdown.json` et `fixtures/markdown-corpus.json`.

use oku_slides_core::Run;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

/// Runs en markdown inline ; `color` et `size` n'ont pas d'écriture markdown.
pub(crate) fn runs_to_markdown(runs: &[Run]) -> String {
    let runs = normalize(runs);
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

    fn bit(self) -> u8 {
        match self {
            Mark::Bold => 1,
            Mark::Italic => 2,
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
    // `spans[i][m]` : runs consécutifs portant `m` dès `i`, calculés une seule fois.
    let mut spans = vec![[0usize; 2]; runs.len() + 1];
    for i in (0..runs.len()).rev() {
        for m in Mark::ALL {
            if !barrier(&runs[i]) && m.of(&runs[i]) {
                spans[i][m as usize] = spans[i + 1][m as usize] + 1;
            }
        }
    }
    let mut stack: Vec<Mark> = Vec::new();
    let mut out = Vec::with_capacity(runs.len() + 1);
    for (i, r) in runs.iter().enumerate() {
        let mut step = Step::default();
        if let Some(depth) = stack.iter().position(|&m| barrier(r) || !m.of(r)) {
            step.close = stack.drain(depth..).rev().collect();
        }
        step.kept.clone_from(&stack);
        if !barrier(r) {
            // La plus longue à l'extérieur : elle reste ouverte le plus longtemps.
            let span = |m: Mark| spans[i][m as usize];
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

/// Espace au sens du `\s` de JS (Lezer) : les blancs Unicode sauf U+0085, plus U+FEFF (BOM).
fn is_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}

/// Classe d'un caractère telle que Lezer la voit contre un délimiteur.
///
/// Lezer teste `[\p{S}\p{P}]` sur une seule unité UTF-16 : hors du plan de base,
/// il ne voit qu'une demi-paire de substitution, ni espace ni ponctuation. Ⓐ
/// (symbole) est donc de la ponctuation, et 😀 une lettre.
fn class(c: Option<char>) -> Class {
    match c {
        None => Class::Space,
        Some(c) if is_space(c) => Class::Space,
        Some(c)
            if u32::from(c) <= 0xFFFF
                && matches!(
                    c.general_category_group(),
                    GeneralCategoryGroup::Punctuation | GeneralCategoryGroup::Symbol
                ) =>
        {
            Class::Punct
        }
        Some(_) => Class::Alnum,
    }
}

/// Format d'un caractère : des caractères voisins de même format forment un run.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Format<'a> {
    /// Emphases en bits (`Mark::bit`).
    marks: u8,
    link: Option<&'a str>,
    highlight: Option<u8>,
}

impl<'a> Format<'a> {
    fn of(r: &'a Run) -> Self {
        let marks = Mark::ALL.into_iter().filter(|m| m.of(r)).fold(0u8, |acc, m| acc | m.bit());
        Self { marks, link: r.link.as_deref(), highlight: r.highlight }
    }

    /// Format d'un caractère sorti de ses délimiteurs : les seules emphases `marks`.
    fn emphasis(marks: u8) -> Self {
        Self { marks, link: None, highlight: None }
    }

    fn barrier(self) -> bool {
        self.link.is_some() || self.highlight.is_some()
    }
}

/// Emphases ouvertes avant un bord, en bits.
#[derive(Clone, Copy, Default)]
struct Stack {
    open: u8,
    /// Les deux ouvertes l'une après l'autre : bit de la première (dehors), sinon 0.
    outer: u8,
}

/// Passage d'un bord : emphases fermées, gardées, ouvertes, puis la pile d'après.
///
/// Deux emphases ouvertes ensemble s'ordonnent par longueur (`steps`) : celle
/// qui cesse la première est dedans, elle se ferme seule. Ce choix ne dépend
/// donc que des runs déjà lus, ce qui permet de normaliser en un seul passage.
struct Edge {
    close: u8,
    kept: u8,
    open: u8,
    after: Stack,
}

fn edge(stack: Stack, next: Option<Format<'_>>) -> Edge {
    let Some(marks) = next.filter(|f| !f.barrier()).map(|f| f.marks) else {
        return Edge { close: stack.open, kept: 0, open: 0, after: Stack::default() };
    };
    let kept = if stack.open & !marks == 0 {
        stack.open
    } else if stack.outer == 0 {
        // Une seule ouverte, ou deux ouvertes ensemble : chacune garde sa place.
        stack.open & marks
    } else if stack.outer & marks == 0 {
        0
    } else {
        stack.outer
    };
    let open = marks & !kept;
    let outer = match (kept, open) {
        (3, _) => stack.outer,
        (0, _) | (_, 0) => 0,
        _ => kept,
    };
    Edge { close: stack.open & !kept, kept, open, after: Stack { open: marks, outer } }
}

/// Classe vue contre un délimiteur ; crochets, `==` et `{n}` des barrières sont de la ponctuation.
fn side(c: Option<(char, Format<'_>)>) -> Class {
    match c {
        Some((_, f)) if f.barrier() => Class::Punct,
        c => class(c.map(|(c, _)| c)),
    }
}

/// Runs dont les bords se lisent comme écrits : seuls des espaces et de la
/// ponctuation y perdent leur mise en forme, jamais du texte.
fn normalize(runs: &[Run]) -> Vec<Run> {
    let mut chars: Vec<(char, Format<'_>)> = runs
        .iter()
        .flat_map(|r| {
            let f = Format::of(r);
            r.t.chars().map(move |c| (c, f))
        })
        .collect();
    unhighlight_edge_spaces(&mut chars);
    fix_emphasis_edges(&mut chars);
    let mut out: Vec<Run> = Vec::new();
    let mut last: Option<Format<'_>> = None;
    for (c, f) in chars {
        match out.last_mut() {
            Some(r) if last == Some(f) => r.t.push(c),
            _ => {
                out.push(Run {
                    bold: f.marks & Mark::Bold.bit() != 0,
                    italic: f.marks & Mark::Italic.bit() != 0,
                    link: f.link.map(str::to_owned),
                    highlight: f.highlight,
                    ..Run::plain(c)
                });
                last = Some(f);
            }
        }
    }
    out
}

/// Fin du run qui commence en `start`.
fn run_end(chars: &[(char, Format<'_>)], start: usize) -> usize {
    let mut end = start;
    while chars.get(end + 1).is_some_and(|&(_, f)| Some(f) == chars.get(start).map(|&(_, f)| f)) {
        end += 1;
    }
    end + 1
}

/// Pastille hors lien : pas d'espace contre `==` (voir markdownHighlight.ts).
fn unhighlight_edge_spaces(chars: &mut [(char, Format<'_>)]) {
    let mut start = 0;
    while start < chars.len() {
        let end = run_end(chars, start);
        let f = chars[start].1;
        if f.highlight.is_some() && f.link.is_none() {
            let lead = chars[start..end].iter().take_while(|&&(c, _)| is_space(c)).count();
            let trail = chars[start + lead..end].iter().rev().take_while(|&&(c, _)| is_space(c)).count();
            for (_, g) in &mut chars[start..start + lead] {
                *g = Format::emphasis(0);
            }
            for (_, g) in &mut chars[end - trail..end] {
                *g = Format::emphasis(0);
            }
        }
        start = end;
    }
}

/// Sort de leurs délimiteurs, un par un, les caractères de bord que CommonMark
/// ne lirait pas comme écrits : chacun garde les seules emphases restées
/// ouvertes autour, et la lecture reprend au bord d'avant.
///
/// Un correctif en `q` ne change aucun bord avant `q` : la pile d'un bord ne
/// dépend que des runs déjà lus (`edge`). Chaque correctif retire une emphase
/// à un caractère, et les correctifs avancent : le tout reste linéaire.
fn fix_emphasis_edges(chars: &mut [(char, Format<'_>)]) {
    // Bords sans correctif, avec la pile d'avant chacun.
    let mut clean: Vec<(usize, Stack)> = Vec::new();
    let (mut p, mut stack) = (0usize, Stack::default());
    loop {
        let (prev, next) = (p.checked_sub(1).map(|i| chars[i]), chars.get(p).copied());
        let this_edge = edge(stack, next.map(|(_, f)| f));
        let (left, right) = (side(prev), side(next));
        // Closers and openers are `*`: a single run between `left` and `right`,
        // which must be able to close (on the `left` side) and open (on the `right` side).
        let fix = if this_edge.close != 0 && (left == Class::Space || (left == Class::Punct && right == Class::Alnum)) {
            p.checked_sub(1)
        } else if this_edge.open != 0 && (right == Class::Space || (right == Class::Punct && left == Class::Alnum)) {
            Some(p)
        } else {
            None
        };
        let Some(q) = fix else {
            if p == chars.len() {
                return;
            }
            clean.push((p, stack));
            stack = this_edge.after;
            p = run_end(chars, p);
            continue;
        };
        chars[q].1 = Format::emphasis(this_edge.kept);
        while clean.last().is_some_and(|&(x, _)| x >= q) {
            clean.pop();
        }
        (p, stack) = match clean.last() {
            // Le run qui commence en `x` garde son format jusqu'à `q` exclu.
            Some(&(x, before)) => {
                let after = edge(before, Some(chars[x].1)).after;
                (if chars[q].1 == chars[x].1 { run_end(chars, q) } else { q }, after)
            }
            None => (0, Stack::default()),
        };
    }
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
            // `<` initial ouvrirait une destination entre chevrons, `\` final
            // échapperait la parenthèse fermante : les deux sont encodés.
            let url = url.replace(' ', "%20").replace('(', "%28").replace(')', "%29").replace('\\', "%5C");
            let url = match url.strip_prefix('<') {
                Some(rest) => format!("%3C{rest}"),
                None => url,
            };
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
            let alnum =
                |j: Option<usize>| j.and_then(|j| chars.get(j)).is_some_and(|&c| class(Some(c)) == Class::Alnum);
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

#[cfg(test)]
#[path = "inline_oracle_tests.rs"]
mod inline_oracle_tests;
