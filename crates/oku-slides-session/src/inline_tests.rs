use oku_slides_core::Run;
use speculoos::prelude::*;

use super::*;

fn run(t: &str) -> Run {
    Run::plain(t)
}

fn bold(t: &str) -> Run {
    Run { bold: true, ..Run::plain(t) }
}

fn italic(t: &str) -> Run {
    Run { italic: true, ..Run::plain(t) }
}

#[test]
fn runs_to_markdown_shared_italic_stays_open() {
    let runs = [Run { italic: true, ..bold("X") }, italic("Y")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("***X**Y*".to_owned());
}

#[test]
fn runs_to_markdown_bold_inside_italic_nests() {
    let runs = [italic("a"), Run { bold: true, ..italic("b") }, italic("c")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("*a**b**c*".to_owned());
}

#[test]
fn runs_to_markdown_bold_highlight_glued_to_letters_keeps_both() {
    let runs = [run("上"), Run { highlight: Some(2), ..bold("a") }, run("です")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("上==**a**=={2}です".to_owned());
}

#[test]
fn runs_to_markdown_space_at_bold_edge_moves_outside() {
    let runs = [run("a"), bold(" x "), run("b")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("a **x** b".to_owned());
}

#[test]
fn runs_to_markdown_punctuation_glued_to_letter_moves_outside() {
    let runs = [run("a"), italic("\"x\""), run("b")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("a\"*x*\"b".to_owned());
}

#[test]
fn runs_to_markdown_punctuation_between_emphases_loses_bold() {
    let runs = [bold("-"), italic("a")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("-*a*".to_owned());
}

#[test]
fn runs_to_markdown_space_only_highlight_is_plain() {
    let runs = [run("a"), Run { highlight: Some(2), ..run(" ") }, run("b")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("a b".to_owned());
}

#[test]
fn runs_to_markdown_spaces_in_link_stay_in_brackets() {
    let runs = [Run { link: Some("https://a.b".into()), ..bold(" x ") }];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("[ **x** ](https://a.b)".to_owned());
}

#[test]
fn runs_to_markdown_italic_highlight_between_italics_closes_italic() {
    let runs = [italic("a"), Run { highlight: Some(1), ..italic("b") }, run("c")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("*a*==*b*==c".to_owned());
}

#[test]
fn runs_to_markdown_bom_at_bold_edge_moves_outside() {
    let runs = [bold("x\u{feff}"), run("!")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("**x**\u{feff}!".to_owned());
}

#[test]
fn runs_to_markdown_bom_at_bold_edge_inside_link_moves_outside_the_marks() {
    let runs = [Run { link: Some("https://a.b".into()), ..bold("x\u{feff}") }];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("[**x**\u{feff}](https://a.b)".to_owned());
}

#[test]
fn runs_to_markdown_circled_letter_is_punctuation_like_lezer() {
    // Ⓐ est une lettre pour Rust mais un symbole (\p{S}) pour Lezer : `a**Ⓐ**` resterait littéral.
    let runs = [run("a"), bold("Ⓐ")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("aⒶ".to_owned());
}

#[test]
fn runs_to_markdown_astral_symbol_is_a_letter_like_lezer() {
    // Lezer lit une demi-paire UTF-16 contre le délimiteur : ni espace ni ponctuation.
    let runs = [bold("!"), run("😀")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("!😀".to_owned());
}

#[test]
fn runs_to_markdown_next_line_char_is_not_a_space_for_lezer() {
    // U+0085 est un blanc Unicode mais pas un `\s` de JS.
    let runs = [bold("!"), run("\u{85}")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("!\u{85}".to_owned());
}

#[test]
fn escape_inline_underscore_between_circled_letters_is_escaped() {
    assert_that!(escape_inline("Ⓐ_Ⓑ")).is_equal_to("Ⓐ\\_Ⓑ".to_owned());
}

/// Générateur déterministe (SplitMix64) : le corpus ne dépend d'aucune crate.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}

/// Lettres, chiffres, espaces et toute la ponctuation qui compte en markdown,
/// plus les caractères que Rust et Lezer classaient différemment : Ⓐ et 🄰
/// (lettres pour Rust, symboles), 😀 (hors du plan de base) et U+0085.
const ALPHABET: &[char] = &[
    'a',
    'b',
    'é',
    '上',
    '1',
    '2',
    ' ',
    ' ',
    '\t',
    '\u{a0}',
    '\u{feff}',
    '!',
    '"',
    '*',
    '_',
    '=',
    '[',
    ']',
    '{',
    '}',
    '\\',
    '#',
    '-',
    '.',
    '(',
    ')',
    '<',
    '`',
    '~',
    '|',
    'Ⓐ',
    '\u{1F130}',
    '😀',
    '\u{85}',
];

fn corpus() -> Vec<Vec<Run>> {
    let mut rng = Rng(180);
    (0..1000)
        .map(|_| {
            (0..1 + rng.below(6))
                .map(|_| {
                    let t: String = (0..1 + rng.below(6)).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect();
                    Run {
                        bold: rng.below(3) == 0,
                        italic: rng.below(3) == 0,
                        link: (rng.below(5) == 0).then(|| ["https://a.b/x", "https://a.b/y"][rng.below(2)].to_owned()),
                        highlight: (rng.below(4) == 0).then(|| 1 + rng.below(3) as u8),
                        ..Run::plain(t)
                    }
                })
                .collect()
        })
        .collect()
}

#[test]
fn runs_to_markdown_corpus_matches_fixture() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/markdown-corpus.json");
    let cases: Vec<String> = corpus()
        .iter()
        .map(|runs| serde_json::json!({ "runs": runs, "markdown": runs_to_markdown(runs) }).to_string())
        .collect();
    let json = format!(
        "{{\n  \"_comment\": \"Généré par inline_tests.rs (UPDATE_CORPUS=1 pour le réécrire) ; relu par markdown.test.ts.\",\n  \"cases\": [\n    {}\n  ]\n}}\n",
        cases.join(",\n    ")
    );
    if std::env::var_os("UPDATE_CORPUS").is_some() {
        std::fs::write(path, &json).unwrap();
    }
    let expected = std::fs::read_to_string(path).unwrap_or_default();
    asserting("corpus à jour (UPDATE_CORPUS=1 pour le réécrire)").that(&(json == expected)).is_true();
}
