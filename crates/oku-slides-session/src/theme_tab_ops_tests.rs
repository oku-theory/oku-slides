use oku_slides_core::{Op, PackKind};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::document_kind::DocumentKind;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, ns, session_in_locale, session_on};
use crate::theme::{PAPIER_THEME, PRO_THEME};
use crate::theme_edit::DRAFT_SWITCH_VAR;
use crate::theme_edit::theme_edit_tests::p;
use crate::theme_tab::theme_tab_tests::{draft_of, theme_tab, with_maison};
use crate::{ExtendsEdit, Localize, Request, Session, SessionError, SessionErrorKind, TabDocument};

/// Asserts that `e` is the `WrongDocument` refusal naming `kind`.
fn assert_wrong_document(s: &Session, e: &SessionError, kind: DocumentKind) {
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::WrongDocument);
    let expected = s.localizer.tr_with("session-wrong-document", &[("kind", kind.as_str().into())]);
    assert_that!(e.localize(&s.localizer)).is_equal_to(expected);
}

fn apply(ops: Vec<Op>) -> Request {
    Request::Apply { label: "x".into(), ops, merge: None }
}

fn set_accent(value: &str) -> Op {
    Op::SetToken { path: p(&["color", "accent"]), value: json!(value) }
}

fn set_theme_op() -> Op {
    Op::SetTheme { theme: Some(ns("pro")), themes: std::collections::BTreeMap::new() }
}

fn add_slide() -> Op {
    serde_json::from_value(json!({ "op": "add_slide", "slide": { "id": "s-new", "kind": "essentiel/content" } }))
        .unwrap()
}

/// A session of `kind` on `data`: a deck tab, a help tab, a workshop, or a theme tab on Papier.
fn session_of(kind: DocumentKind, data: &std::path::Path) -> Session {
    let mut s = session_on(data);
    match kind {
        DocumentKind::Deck => {}
        DocumentKind::Help => s.open_help("fr", None).unwrap(),
        DocumentKind::Pack => s.edit_pack(ESSENTIEL_PACK).unwrap(),
        DocumentKind::Theme => s.open_theme_here(PAPIER_THEME).unwrap(),
    }
    s
}

#[rstest]
#[case::theme_add_slide(DocumentKind::Theme, apply(vec![add_slide()]))]
#[case::theme_save_pack(DocumentKind::Theme, Request::SavePack { as_new: false })]
#[case::deck_rename_edited_theme(DocumentKind::Deck, Request::RenameEditedTheme { name: "x".into() })]
#[case::pack_set_token(DocumentKind::Pack, apply(vec![set_accent("#ff0000")]))]
#[case::pack_set_theme(DocumentKind::Pack, apply(vec![set_theme_op()]))]
#[case::help_set_extends(DocumentKind::Help, Request::SetEditedThemeExtends { extends: Vec::new() })]
fn wrong_document_request_returns_typed_error(#[case] kind: DocumentKind, #[case] request: Request) {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = session_of(kind, data.path());
    let e = s.handle(request).unwrap_err();
    assert_wrong_document(&s, &e, kind);
}

#[test]
fn wrong_document_request_is_said_in_french() {
    let mut s = session_in_locale("fr");
    let e = s.handle(Request::RenameEditedTheme { name: "x".into() }).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr_with("session-wrong-document", &[("kind", "deck".into())]));
    assert_that!(e.localize(&fr)).is_not_equal_to(
        oku_slides_i18n::Localizer::new(Some("en")).tr_with("session-wrong-document", &[("kind", "deck".into())]),
    );
}

#[rstest]
#[case::op_alone(apply(vec![set_theme_op()]))]
#[case::request(Request::SetTheme { theme: "pro".into() })]
fn theme_tab_refuses_a_base_change(#[case] request: Request) {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    let e = s.handle(request).unwrap_err();
    assert_wrong_document(&s, &e, DocumentKind::Theme);
    assert_that!(draft_of(&s).base.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.tab_info().document).is_equal_to(TabDocument::Theme { theme: PAPIER_THEME });
}

#[test]
fn theme_tab_refuses_an_op_on_a_specimen_element() {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    let before = s.deck().clone();
    let op: Op = serde_json::from_value(
        json!({ "op": "set_param", "target": { "slide": "layout-title" }, "param": "title", "value": [] }),
    )
    .unwrap();
    let e = s.apply("x", vec![op], None).unwrap_err();
    assert_wrong_document(&s, &e, DocumentKind::Theme);
    assert_that!(*s.deck()).is_equal_to(before);
}

/// Runs `ops` on `s` through `handle`, or through `apply` as MCP does.
fn apply_through(s: &mut Session, via_handle: bool, ops: Vec<Op>) -> crate::Result<()> {
    if via_handle { s.handle(apply(ops)).map(|_| ()) } else { s.apply("x", ops, None) }
}

#[rstest]
#[case::handle(true)]
#[case::apply(false)]
fn read_only_theme_tab_refuses_a_non_token_op_as_the_wrong_document(#[case] via_handle: bool) {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let (mut holder, id) = with_maison(data.path());
    holder.open_theme_here(id).unwrap();
    let mut s = session_on(data.path());
    s.open_theme_here(id).unwrap();
    let e = apply_through(&mut s, via_handle, vec![add_slide()]).unwrap_err();
    assert_wrong_document(&s, &e, DocumentKind::Theme);
}

#[test]
fn theme_tab_accepts_token_ops() {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    s.apply("x", vec![set_accent("#ff0000")], None).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#ff0000")));
}

#[test]
fn theme_tab_extends_edit_is_one_undo_step() {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    let before = draft_of(&s).extends;
    s.edit_theme_extends(ExtendsEdit::Add { theme: ns("pro") }).unwrap();
    assert_that!(draft_of(&s).extends.len()).is_equal_to(before.len() + 1);
    s.undo().unwrap();
    assert_that!(draft_of(&s).extends).is_equal_to(before);
}

#[rstest]
#[case::deck(DocumentKind::Deck)]
#[case::pack(DocumentKind::Pack)]
#[case::theme(DocumentKind::Theme)]
fn set_var_of_the_draft_rank_is_refused_from_callers(#[case] kind: DocumentKind) {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = session_of(kind, data.path());
    let op = Op::SetVar { key: DRAFT_SWITCH_VAR.to_owned(), value: Value::from(99) };
    let e = s.apply("x", vec![op], None).unwrap_err();
    assert_wrong_document(&s, &e, kind);
    assert_that!(s.deck().vars.get(DRAFT_SWITCH_VAR) == Some(&Value::from(99))).is_false();
}

/// A deck mutator the agent calls directly, run on `s`.
type Mutator = fn(&mut Session) -> crate::Result<()>;

fn set_widget(s: &mut Session) -> crate::Result<()> {
    s.set_widget_by_agent(None, "w", "#let x = 1".into()).map(|_| ())
}

fn remove_deck_pack(s: &mut Session) -> crate::Result<()> {
    s.remove_deck_pack(PackKind::Layouts, &ns("essentiel"))
}

fn save_copy_new(s: &mut Session) -> crate::Result<()> {
    let dir = fresh_dir("oku-slides-theme-tab-copy-");
    s.save_copy_new(&dir.path().join("copie.oku-slides.pdf"))
}

fn new_deck(s: &mut Session) -> crate::Result<()> {
    s.new_deck()
}

#[rstest]
#[case::set_widget_by_agent(set_widget as Mutator)]
#[case::remove_deck_pack(remove_deck_pack as Mutator)]
#[case::save_copy_new(save_copy_new as Mutator)]
#[case::new_deck(new_deck as Mutator)]
fn theme_tab_refuses_direct_deck_mutators(#[case] mutator: Mutator) {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = theme_tab(data.path(), PAPIER_THEME);
    let before = s.deck().clone();
    let e = mutator(&mut s).unwrap_err();
    assert_wrong_document(&s, &e, DocumentKind::Theme);
    assert_that!(*s.deck()).is_equal_to(before);
}

#[test]
fn undo_turn_in_a_theme_tab_undoes_the_agent_tokens() {
    let data = fresh_dir("oku-slides-theme-tab-ops-");
    let mut s = theme_tab(data.path(), PRO_THEME);
    s.begin_turn(1, "accent");
    s.apply("x", vec![set_accent("#ff0000")], None).unwrap();
    s.end_turn();
    s.handle(Request::UndoTurn { turn: 1 }).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_none();
}
