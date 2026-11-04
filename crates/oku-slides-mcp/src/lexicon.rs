//! Spoken command words of the user's languages, matched against dictated text.
//!
//! The forms live in the i18n crate (`voice.ftl`); here they are split into folded
//! tokens once per [`Lexicon`], and an utterance is matched whole, never by substring.

use std::collections::BTreeSet;

use oku_slides_i18n::{REFERENCE, shipped, voice_forms};

#[path = "lexicon_token.rs"]
mod token;
use token::{Form, Token, compile, is_unspaced_word, match_len, parse_all, segment, strip_fillers, tokens, words};

/// Number of ordinal positions each locale spells out.
const ORDINALS: usize = 9;

/// The verb a danger confirmation's button shows and the user must say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmVerb {
    /// Save over an existing file.
    Replace,
    /// Merge one deck into another.
    Merge,
    /// Open a new deck over a dirty one, or close with unsaved work.
    Discard,
    /// Delete a pack.
    Delete,
    /// Send dictation to the API.
    UseApi,
    /// Install an update.
    Install,
    /// Quit the app.
    Quit,
    /// Close another tab.
    Close,
    /// Import a file.
    Import,
}

impl ConfirmVerb {
    /// Every verb, in declaration order (the order of its discriminants).
    pub const ALL: [Self; 9] = [
        Self::Replace,
        Self::Merge,
        Self::Discard,
        Self::Delete,
        Self::UseApi,
        Self::Install,
        Self::Quit,
        Self::Close,
        Self::Import,
    ];

    /// The Fluent key of this verb's button label, one of its spoken forms.
    #[must_use]
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::Replace => "agent-surface-verb-replace",
            Self::Merge => "agent-surface-verb-merge",
            Self::Discard => "agent-surface-verb-discard",
            Self::Delete => "agent-surface-verb-delete",
            Self::UseApi => "agent-surface-verb-use-api",
            Self::Install => "agent-surface-verb-install",
            Self::Quit => "agent-surface-verb-quit",
            Self::Close => "agent-surface-verb-close",
            Self::Import => "agent-surface-verb-import",
        }
    }

    /// The Fluent key holding this verb's spoken forms.
    const fn key(self) -> &'static str {
        match self {
            Self::Replace => "voice-verb-replace",
            Self::Merge => "voice-verb-merge",
            Self::Discard => "voice-verb-discard",
            Self::Delete => "voice-verb-delete",
            Self::UseApi => "voice-verb-use-api",
            Self::Install => "voice-verb-install",
            Self::Quit => "voice-verb-quit",
            Self::Close => "voice-verb-close",
            Self::Import => "voice-verb-import",
        }
    }
}

/// What a spoken answer says to a yes-or-no question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YesNo {
    /// A clear yes, with the confirmation verb when one is asked.
    Yes,
    /// A refusal, or a negated verb.
    No,
    /// Anything else: chatter, both answers, or a bare verb.
    Unclear,
}

/// A spoken navigation command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// Go to the next slide.
    Next,
    /// Go to the previous slide.
    Previous,
    /// Go to the first slide.
    First,
    /// Go to the last slide.
    Last,
    /// Leave the presentation.
    Exit,
}

/// Spoken command words of the user's languages, folded once per utterance.
#[derive(Debug)]
pub struct Lexicon {
    yes: Vec<Form>,
    no: Vec<Form>,
    negation: Vec<Form>,
    stop: Vec<Form>,
    ordinals: Vec<Vec<Form>>,
    /// The ordinal forms with their filler words kept: pt "um" is an en filler.
    ordinals_raw: Vec<Vec<Form>>,
    nav: [(Nav, Vec<Form>); 5],
    wake: Vec<Form>,
    filler: Vec<Form>,
    option: Vec<Form>,
    verbs: Vec<Vec<Form>>,
    /// Every one-word form of an unspaced script: what such a token is cut into.
    pieces: BTreeSet<String>,
}

impl Lexicon {
    /// Builds from the live UI and dictation locales; duplicates merged.
    ///
    /// A UI locale that is not shipped reads as English. An unshipped dictation locale adds nothing.
    #[must_use]
    pub fn for_locales(ui: &str, dictation: Option<&str>) -> Self {
        let mut locales = vec![shipped(ui).unwrap_or(REFERENCE)];
        if let Some(extra) = dictation.and_then(shipped)
            && !locales.contains(&extra)
        {
            locales.push(extra);
        }
        Self::from_locales(&locales)
    }

    /// Builds from any set of locales; duplicates merged.
    fn from_locales(locales: &[&str]) -> Self {
        let gather = |key: &str| -> Vec<String> { locales.iter().flat_map(|l| voice_forms(l, key)).collect() };
        let filler = compile(gather("voice-filler"), None);
        let plain = |key: &str| compile(gather(key), Some(&filler));
        let nav = |kind, key| (kind, plain(key));
        let mut lexicon = Self {
            yes: plain("voice-yes"),
            no: plain("voice-no"),
            negation: plain("voice-negation"),
            stop: plain("voice-stop"),
            ordinals: (1..=ORDINALS).map(|n| plain(&format!("voice-ordinal-{n}"))).collect(),
            ordinals_raw: (1..=ORDINALS).map(|n| compile(gather(&format!("voice-ordinal-{n}")), None)).collect(),
            nav: [
                nav(Nav::Next, "voice-next"),
                nav(Nav::Previous, "voice-previous"),
                nav(Nav::First, "voice-first"),
                nav(Nav::Last, "voice-last"),
                nav(Nav::Exit, "voice-exit"),
            ],
            wake: compile(gather("voice-wake"), None),
            option: plain("voice-option"),
            verbs: ConfirmVerb::ALL.iter().map(|v| plain(v.key())).collect(),
            filler,
            pieces: BTreeSet::new(),
        };
        lexicon.pieces = lexicon.one_word_forms().filter(|word| is_unspaced_word(word)).map(str::to_owned).collect();
        lexicon
    }

    /// The words of every form that is a single word.
    fn one_word_forms(&self) -> impl Iterator<Item = &str> {
        let kinds = [&self.yes, &self.no, &self.negation, &self.stop, &self.wake, &self.filler, &self.option];
        let nav = self.nav.iter().map(|(_, forms)| forms);
        let ordinals = self.ordinals.iter();
        let verbs = self.verbs.iter();
        kinds.into_iter().chain(nav).chain(ordinals).chain(verbs).flatten().filter_map(|form| match form.as_slice() {
            [word] => Some(word.as_str()),
            _ => None,
        })
    }

    /// The utterance as folded words, unspaced scripts cut into known words.
    fn tokens(&self, said: &str) -> Vec<Token> {
        segment(&self.pieces, said, tokens(said))
    }

    /// The utterance as folded words, filler dropped.
    fn content(&self, said: &str) -> Vec<Token> {
        strip_fillers(&self.filler, self.tokens(said))
    }

    /// Reads a spoken answer; with `verb`, a yes needs exactly one yes form and that verb.
    ///
    /// The verb is read from the lexicon, never from text. Any negation or no form makes
    /// the answer `No` or `Unclear`, never `Yes`; a bare verb is `Unclear`. "Annule" and
    /// "cancel" are no-words, not stop-words: stop is routed before answers, so a stop
    /// form would turn a refusal into a cancelled turn.
    #[must_use]
    pub fn yes_no(&self, said: &str, verb: Option<ConfirmVerb>) -> YesNo {
        #[derive(Clone, Copy, PartialEq)]
        enum Word {
            Yes,
            No,
            Negation,
            Verb,
        }
        // Refusals first: on a tie between kinds the first wins, and a tie must never read as yes.
        let mut kinds: Vec<(Word, &[Form])> =
            vec![(Word::Negation, &self.negation), (Word::No, &self.no), (Word::Yes, &self.yes)];
        if let Some(verb) = verb {
            kinds.push((Word::Verb, self.verb(verb)));
        }
        let Some(heard) = parse_all(&self.content(said), &kinds) else { return YesNo::Unclear };
        let count = |word| heard.iter().filter(|w| **w == word).count();
        let (yes, refusals, verbs) = (count(Word::Yes), count(Word::No) + count(Word::Negation), count(Word::Verb));
        let yes_ok = match verb {
            Some(_) => yes == 1 && verbs == 1,
            None => yes >= 1,
        };
        if yes_ok && refusals == 0 {
            YesNo::Yes
        } else if yes == 0 && refusals > 0 && verbs <= 1 {
            YesNo::No
        } else {
            YesNo::Unclear
        }
    }

    /// The 0-based position of a spoken ordinal or number, if it is within `1..=count`.
    ///
    /// Read without filler words first; an utterance that is only filler words (pt "um", an
    /// en filler) is read again with them kept.
    #[must_use]
    pub fn ordinal(&self, said: &str, count: usize) -> Option<usize> {
        self.ordinal_in(&self.content(said), &self.ordinals, count)
            .or_else(|| self.ordinal_in(&self.tokens(said), &self.ordinals_raw, count))
    }

    /// The ordinal `all` spells with `ordinals`, if it is within `1..=count`.
    fn ordinal_in(&self, all: &[Token], ordinals: &[Vec<Form>], count: usize) -> Option<usize> {
        // "option two", "number 2": a leading option word is dropped, but never stands alone.
        let toks = all.get(match_len(&self.option, all, 0).unwrap_or(0)..).filter(|rest| !rest.is_empty())?;
        let number = match toks {
            [only] if only.folded.chars().all(|c| c.is_ascii_digit()) => only.folded.parse::<usize>().ok()?,
            _ => {
                let kinds: Vec<(usize, &[Form])> =
                    ordinals.iter().enumerate().map(|(i, forms)| (i + 1, forms.as_slice())).collect();
                let heard = parse_all(toks, &kinds)?;
                let first = *heard.first()?;
                if heard.iter().any(|n| *n != first) {
                    return None;
                }
                first
            }
        };
        (1..=count).contains(&number).then(|| number - 1)
    }

    /// The index of the one label the utterance is, accents, case and filler words ignored.
    ///
    /// The utterance must equal the label: one that only contains it ("make the title dark") is
    /// a prompt, never a pick. `None` when no label or several match. A label made only of
    /// filler words ("A", pt "O") is picked by an utterance equal to it, filler words kept.
    #[must_use]
    pub fn label(&self, said: &str, labels: &[&str]) -> Option<usize> {
        let raw = words(self.tokens(said));
        let said = words(self.content(said));
        if raw.is_empty() {
            return None;
        }
        // A label left empty by the fillers is compared whole.
        let hits: Vec<usize> = labels
            .iter()
            .enumerate()
            .filter(|(_, l)| match words(self.content(l)) {
                content if content.is_empty() => words(self.tokens(l)) == raw,
                content => !said.is_empty() && content == said,
            })
            .map(|(i, _)| i)
            .collect();
        match hits.as_slice() {
            [one] => Some(*one),
            _ => None,
        }
    }

    /// True if the utterance is only filler words ("hmm", "uh, thanks") or nothing at all.
    #[must_use]
    pub fn is_filler_only(&self, said: &str) -> bool {
        self.content(said).is_empty()
    }

    /// True if the whole utterance is a stop command.
    #[must_use]
    pub fn stop(&self, said: &str) -> bool {
        parse_all(&self.content(said), &[((), &self.stop)]).is_some_and(|heard| !heard.is_empty())
    }

    /// The navigation command the whole utterance spells, if it is one and only one.
    #[must_use]
    pub fn nav(&self, said: &str) -> Option<Nav> {
        let kinds: Vec<(Nav, &[Form])> = self.nav.iter().map(|(kind, forms)| (*kind, forms.as_slice())).collect();
        let heard = parse_all(&self.content(said), &kinds)?;
        let first = *heard.first()?;
        heard.iter().all(|n| *n == first).then_some(first)
    }

    /// The original text after the wake phrase, accents and case kept.
    ///
    /// The slice starts at the original position of the first word after the wake form,
    /// never at an offset of the folded text, which differs once accents are folded.
    #[must_use]
    pub fn wake<'a>(&self, said: &'a str) -> Option<&'a str> {
        let toks = self.tokens(said);
        let mut at = 0;
        while let Some(n) = match_len(&self.filler, &toks, at) {
            at += n;
        }
        if let Some(used) = match_len(&self.wake, &toks, at) {
            return Some(toks.get(at + used).and_then(|t| said.get(t.start..)).unwrap_or_default());
        }
        self.wake_prefix(said, &toks, at)
    }

    /// A wake form that ends the first word of an unspaced script ("奥库写标题"): the text after it.
    fn wake_prefix<'a>(&self, said: &'a str, toks: &[Token], at: usize) -> Option<&'a str> {
        self.wake.iter().find_map(|form| {
            let (last, head) = form.split_last()?;
            let first = toks.get(at..at + head.len())?;
            let tok = toks.get(at + head.len())?;
            let whole = head.iter().zip(first).all(|(w, t)| *w == t.folded);
            let prefix = is_unspaced_word(last)
                && tok.folded.starts_with(last.as_str())
                && said.get(tok.start..tok.end) == Some(tok.folded.as_str());
            if whole && prefix { said.get(tok.start + last.len()..) } else { None }
        })
    }

    /// The spoken forms of a confirmation verb, from `voice-verb-<name>`.
    #[must_use]
    pub fn verb(&self, verb: ConfirmVerb) -> &[Vec<String>] {
        self.verbs.get(verb as usize).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
#[path = "lexicon_tests.rs"]
mod lexicon_tests;
