use speculoos::prelude::*;

use super::*;

#[test]
fn pc_keys_command_letter_becomes_ctrl_plus_letter() {
    assert_that!(pc_keys("Annuler (⌘Z)").into_owned()).is_equal_to("Annuler (Ctrl+Z)".to_owned());
}

#[test]
fn pc_keys_modifiers_follow_the_pc_order() {
    assert_that!(pc_keys("⌘⇧S").into_owned()).is_equal_to("Ctrl+Shift+S".to_owned());
    assert_that!(pc_keys("⌃⇧Tab").into_owned()).is_equal_to("Ctrl+Shift+Tab".to_owned());
}

#[test]
fn pc_keys_option_becomes_alt() {
    assert_that!(pc_keys("⌥↑ ⌥↓").into_owned()).is_equal_to("Alt+↑ Alt+↓".to_owned());
}

#[test]
fn pc_keys_modifier_then_space_then_word_joins_them() {
    assert_that!(pc_keys("⌘ Entrée pour valider").into_owned()).is_equal_to("Ctrl+Entrée pour valider".to_owned());
}

#[test]
fn pc_keys_lone_modifier_is_its_name() {
    assert_that!(pc_keys("⇧").into_owned()).is_equal_to("Shift".to_owned());
    assert_that!(pc_keys("avec ⇧ : un").into_owned()).is_equal_to("avec Shift : un".to_owned());
}

#[test]
fn pc_keys_without_modifier_borrows_the_text() {
    assert_that!(matches!(pc_keys("Entrée, Échap"), Cow::Borrowed(_))).is_true();
}

#[test]
fn platform_keys_on_this_platform_matches_its_notation() {
    let shown = platform_keys("⌘O").into_owned();
    let expected = if cfg!(target_os = "macos") { "⌘O" } else { "Ctrl+O" };
    assert_that!(shown).is_equal_to(expected.to_owned());
}
