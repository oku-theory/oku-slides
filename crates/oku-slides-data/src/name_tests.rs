use std::collections::BTreeSet;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::words("Matrice des risques", "matrice_des_risques")]
#[case::accent("Écarts", "ecarts")]
#[case::cyrillic("Продажи", "data")]
#[case::leading_digit("2026 ventes", "_2026_ventes")]
#[case::empty("", "data")]
#[case::emoji("📈", "data")]
#[case::underscores("___", "data")]
fn default_name_folds_the_text(#[case] text: &str, #[case] expected: &str) {
    assert_that!(default_name(text, 2, |_| false).0).is_equal_to(expected.to_owned());
}

#[test]
fn default_name_cuts_a_long_text_to_sixty_chars() {
    assert_that!(default_name(&"a".repeat(200), 2, |_| false).0).is_equal_to("a".repeat(60));
}

#[test]
fn default_name_suffix_after_nine_taken_stays_within_the_name_limit() {
    let base = "a".repeat(60);
    let taken: BTreeSet<String> = std::iter::once(base.clone()).chain((2..=9).map(|n| format!("{base}_{n}"))).collect();
    let name = default_name(&"a".repeat(200), 2, |n| taken.contains(n)).0;
    assert_that!(name).is_equal_to(format!("{base}_10"));
    assert_that!(name.len()).is_less_than_or_equal_to(64);
}

#[test]
fn default_name_skips_a_name_taken_in_another_case() {
    let name = default_name("Ventes", 2, |n| n.eq_ignore_ascii_case("VENTES")).0;
    assert_that!(name).is_equal_to("ventes_2".to_owned());
}

#[rstest]
#[case::base_free(&[][..], 5, ("ventes", 1))]
#[case::from_five(&["ventes"][..], 5, ("ventes_5", 5))]
#[case::five_taken(&["ventes", "ventes_5"][..], 5, ("ventes_6", 6))]
fn default_name_tries_suffixes_from_first(
    #[case] taken: &[&str],
    #[case] first: usize,
    #[case] expected: (&str, usize),
) {
    let (name, n) = default_name("Ventes", first, |n| taken.contains(&n));
    assert_that!((name.as_str(), n)).is_equal_to(expected);
}

/// A fixed-seed xorshift: the same strings on every run.
struct Strings(u64);

impl Strings {
    const ALPHABET: &'static [char] =
        &['a', 'Z', '0', '9', '_', ' ', '-', 'é', 'Ç', 'ß', 'œ', 'ж', '中', '📈', '\u{202E}', '\u{200D}', '\n', '.'];

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn string(&mut self) -> String {
        let len = usize::try_from(self.next() % 80).unwrap();
        (0..len).map(|_| Self::ALPHABET[usize::try_from(self.next()).unwrap() % Self::ALPHABET.len()]).collect()
    }
}

#[test]
fn default_name_is_valid_for_generated_strings() {
    let mut strings = Strings(0x5eed_1690);
    for _ in 0..1_000 {
        let text = strings.string();
        let name = default_name(&text, 2, |_| false).0;
        asserting(&format!("{text:?} gives {name:?}")).that(&valid_name(&name)).is_true();
    }
}

#[rstest]
#[case::at_bound(&"a".repeat(64), true)]
#[case::over_bound(&"a".repeat(65), false)]
#[case::empty("", false)]
#[case::uppercase("Ventes", false)]
#[case::accent("é", false)]
#[case::leading_digit("1a", false)]
#[case::leading_underscore("_1", true)]
#[case::sql_reserved("select", true)]
fn valid_name_follows_the_name_rules(#[case] name: &str, #[case] expected: bool) {
    assert_that!(valid_name(name)).is_equal_to(expected);
}
