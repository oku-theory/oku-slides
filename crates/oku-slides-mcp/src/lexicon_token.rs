//! Words of an utterance: tokens, unspaced-script segmentation and whole-form matching.

use std::collections::BTreeSet;

use oku_slides_i18n::fold;

/// A word of an utterance: folded, with its byte range in the original text.
#[derive(Debug)]
pub(super) struct Token {
    pub(super) folded: String,
    pub(super) start: usize,
    pub(super) end: usize,
}

/// One spoken form as folded tokens.
pub(super) type Form = Vec<String>;

/// True for a character that ends a word: whitespace and punctuation of any script.
fn is_separator(c: char) -> bool {
    c.is_whitespace()
        || c.is_ascii_punctuation()
        || matches!(c,
            '\u{2000}'..='\u{206F}' | '\u{3000}'..='\u{3004}' | '\u{3008}'..='\u{3020}' | '\u{30FB}'
            | '\u{FF01}'..='\u{FF0F}' | '\u{FF1A}'..='\u{FF20}' | '\u{FF3B}'..='\u{FF40}' | '\u{FF5B}'..='\u{FF65}'
            | '¡' | '¿' | '«' | '»' | '،' | '؛' | '؟' | '۔' | '।' | '॥')
}

/// True for a character of a script written without spaces (Han, kana, Thai).
fn is_unspaced(c: char) -> bool {
    matches!(c,
        '\u{0E00}'..='\u{0E7F}' | '\u{3040}'..='\u{30FF}' | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}' | '\u{FF66}'..='\u{FF9F}')
}

/// True if every character of `word` is of an unspaced script.
pub(super) fn is_unspaced_word(word: &str) -> bool {
    !word.is_empty() && word.chars().all(is_unspaced)
}

/// Splits each unspaced-script token into known `pieces`, longest first, covering the whole token.
///
/// A piece keeps its byte range in `text`. A token that pieces cannot cover, or whose fold
/// changed its bytes, stays whole, so an unknown word never matches by accident.
pub(super) fn segment(pieces: &BTreeSet<String>, text: &str, toks: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::with_capacity(toks.len());
    // No piece is longer: each start tries that many ends at most, so a long token stays linear.
    let longest = pieces.iter().map(|p| p.chars().count()).max().unwrap_or(0);
    for tok in toks {
        if !is_unspaced_word(&tok.folded) || text.get(tok.start..tok.end) != Some(tok.folded.as_str()) {
            out.push(tok);
            continue;
        }
        let bounds: Vec<usize> = tok.folded.char_indices().map(|(at, _)| at).chain([tok.folded.len()]).collect();
        let last = bounds.len() - 1;
        // `next[i]`: where the longest piece starting at char `i` ends, if the rest is coverable.
        let mut next: Vec<Option<usize>> = vec![None; bounds.len()];
        for from in (0..last).rev() {
            next[from] = (from + 1..=last.min(from.saturating_add(longest))).rev().find(|&to| {
                (to == last || next[to].is_some())
                    && tok.folded.get(bounds[from]..bounds[to]).is_some_and(|piece| pieces.contains(piece))
            });
        }
        if next[0].is_none() {
            out.push(tok);
            continue;
        }
        let mut from = 0;
        while from < last {
            let Some(to) = next[from] else { break };
            let (start, end) = (tok.start + bounds[from], tok.start + bounds[to]);
            out.push(Token { folded: tok.folded[bounds[from]..bounds[to]].to_owned(), start, end });
            from = to;
        }
    }
    out
}

/// The words of `text`, each folded on its own with its byte offset in `text`.
pub(super) fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut open: Option<usize> = None;
    let close = |out: &mut Vec<Token>, start: usize, end: usize| {
        let folded = fold(text.get(start..end).unwrap_or_default());
        if !folded.is_empty() {
            out.push(Token { folded, start, end });
        }
    };
    for (at, c) in text.char_indices() {
        match (is_separator(c), open) {
            (true, Some(start)) => {
                close(&mut out, start, at);
                open = None;
            }
            (false, None) => open = Some(at),
            _ => {}
        }
    }
    if let Some(start) = open {
        close(&mut out, start, text.len());
    }
    out
}

/// Length in tokens of the first form matching `toks` at `at`; forms are longest first.
pub(super) fn match_len(forms: &[Form], toks: &[Token], at: usize) -> Option<usize> {
    let rest = toks.get(at..)?;
    forms
        .iter()
        .find(|form| !form.is_empty() && form.len() <= rest.len() && form.iter().zip(rest).all(|(w, t)| *w == t.folded))
        .map(Vec::len)
}

/// `toks` without the filler words.
pub(super) fn strip_fillers(fillers: &[Form], toks: Vec<Token>) -> Vec<Token> {
    let mut keep = vec![true; toks.len()];
    let mut at = 0;
    while at < toks.len() {
        match match_len(fillers, &toks, at) {
            Some(n) => {
                keep[at..at + n].fill(false);
                at += n;
            }
            None => at += 1,
        }
    }
    toks.into_iter().zip(keep).filter_map(|(t, k)| k.then_some(t)).collect()
}

/// Reads the whole of `toks` as a sequence of forms of the given kinds, longest form first.
///
/// `None` when any word belongs to no kind: an utterance must match whole.
pub(super) fn parse_all<K: Copy>(toks: &[Token], kinds: &[(K, &[Form])]) -> Option<Vec<K>> {
    let mut at = 0;
    let mut out = Vec::new();
    while at < toks.len() {
        let mut best: Option<(K, usize)> = None;
        for (kind, forms) in kinds {
            if let Some(n) = match_len(forms, toks, at)
                && best.is_none_or(|(_, longest)| n > longest)
            {
                best = Some((*kind, n));
            }
        }
        let (kind, n) = best?;
        out.push(kind);
        at += n;
    }
    Some(out)
}

/// Raw forms as folded token sequences: filler words dropped, duplicates merged, longest first.
pub(super) fn compile(raw: Vec<String>, fillers: Option<&[Form]>) -> Vec<Form> {
    let mut forms: Vec<Form> = Vec::new();
    for text in raw {
        let toks = match fillers {
            Some(f) => strip_fillers(f, tokens(&text)),
            None => tokens(&text),
        };
        let form: Form = toks.into_iter().map(|t| t.folded).collect();
        if !form.is_empty() && !forms.contains(&form) {
            forms.push(form);
        }
    }
    forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
    forms
}

/// The words of a token list, for comparing texts.
pub(super) fn words(toks: Vec<Token>) -> Vec<String> {
    toks.into_iter().map(|t| t.folded).collect()
}
