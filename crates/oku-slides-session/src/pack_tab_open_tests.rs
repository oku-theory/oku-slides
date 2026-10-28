use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::library::DraftWrite;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, preview_of, rename_title, session_on};
use crate::test_support_packs::{
    CARD_V2, adopting_deck, assert_unleased, draft_of, editing, opener_on, pack_tab_from, pack_tab_of, scope_of,
};
use crate::{BASE_PACK, Localize, SessionErrorKind};

/// The opener of `init_pack_tab_preview_*`: a deck or a pack tab.
#[derive(Debug, Clone, Copy)]
enum Opener {
    Deck,
    PackTab,
}

/// A pack tab on Base, seeded by an opener of `kind` on `data`.
fn base_tab_from(kind: Opener, data: &std::path::Path) -> Session {
    let (mut s, _deck, acme) = opener_on(data);
    let mut opener = match kind {
        Opener::Deck => s,
        Opener::PackTab => pack_tab_from(&mut s, data, acme),
    };
    pack_tab_from(&mut opener, data, BASE_PACK)
}

#[test]
fn init_pack_tab_edits_the_pack_in_a_tab() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let t = pack_tab_from(&mut s, data.path(), acme);
    assert_that!(editing(&t)).is_equal_to(Some(acme));
    assert_that!(t.tab_info().document).is_equal_to(TabDocument::Pack { pack: acme });
    assert_that!(crate::pack_lease::pack_leased(&scope_of(&t), acme)).is_equal_to(Some("acme".to_owned()));
}

#[test]
fn init_pack_tab_preview_keeps_the_opener_theme_and_tokens() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let (theme, tokens) = (s.deck().theme.clone(), s.deck().tokens.clone());
    let t = pack_tab_from(&mut s, data.path(), acme);
    let preview = preview_of(&t).sources.deck();
    assert_that!(preview.theme.as_ref().map(oku_slides_core::Namespace::as_str)).is_equal_to(Some("pro"));
    assert_that!((preview.theme.clone(), preview.tokens.clone())).is_equal_to((theme, tokens));
}

#[rstest]
#[case::deck(Opener::Deck)]
#[case::pack_tab(Opener::PackTab)]
fn init_pack_tab_preview_has_no_broken_widget_notice(#[case] kind: Opener) {
    let data = fresh_dir("oku-slides-open-");
    let t = base_tab_from(kind, data.path());
    assert_that!(preview_of(&t).notice.as_deref()).is_none();
}

#[rstest]
#[case::deck(Opener::Deck)]
#[case::pack_tab(Opener::PackTab)]
fn init_pack_tab_preview_registry_holds_the_chart_kind(#[case] kind: Opener) {
    let data = fresh_dir("oku-slides-open-");
    let t = base_tab_from(kind, data.path());
    let preview = &preview_of(&t).sources;
    assert_that!(preview.deck().slide("s-1").and_then(|s| s.locate("c-1")).is_some()).is_true();
    assert_that!(preview.registry().get("base/chart").is_some()).is_true();
}

#[test]
fn init_pack_tab_preview_renders_with_the_opener_fonts() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let t = pack_tab_from(&mut s, data.path(), acme);
    assert_that!(t.render_deck_fonts).is_equal_to(1);
}

#[test]
fn init_pack_tab_preview_has_no_file_and_a_new_identity() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let uid = s.doc.sources.replica().deck_uid();
    let t = pack_tab_from(&mut s, data.path(), acme);
    let preview = preview_of(&t);
    assert_that!((preview.path.is_none(), preview.saved_hash.is_none())).is_equal_to((true, true));
    assert_that!(preview.sources.replica().deck_uid()).is_not_equal_to(uid);
}

#[test]
fn init_pack_tab_preview_is_isolated_from_the_opener() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let t = pack_tab_from(&mut s, data.path(), acme);
    let title = preview_of(&t).sources.deck().params.title.clone();
    s.apply("titre", vec![rename_title("Autre")], None).unwrap();
    assert_that!(preview_of(&t).sources.deck().params.title.clone()).is_equal_to(title);
}

#[test]
fn init_pack_tab_preview_adopts_nothing() {
    let data = fresh_dir("oku-slides-open-");
    let mut s = session_on(data.path());
    let mut seeded = adopting_deck(&mut s, data.path());
    let acme = crate::test_support_packs::acme_of(&s);
    let v = s.user_deck().widgets.values().find(|p| p.id == acme).map(|p| p.version.clone()).unwrap();
    let _t = pack_tab_from(&mut seeded, data.path(), BASE_PACK);
    assert_that!(draft_of(&s, acme, &v)).is_equal_to(DraftWrite::Replace(v));
}

#[test]
fn init_pack_tab_freezes_its_base() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    // Another pack tab leaves an unfrozen current draft, then goes without closing.
    let mut other = pack_tab_of(data.path(), acme);
    other.apply("x", vec![rename_title("acme 2")], None).unwrap();
    let draft = other.save_pack(false).unwrap();
    drop(other);
    assert_that!(draft_of(&s, acme, &draft)).is_equal_to(DraftWrite::Replace(draft.clone()));
    let _t = pack_tab_from(&mut s, data.path(), acme);
    assert_that!(draft_of(&s, acme, &draft)).is_equal_to(DraftWrite::Child(draft));
}

/// How `init_pack_tab_never_freezes_the_draft` writes.
#[derive(Debug, Clone, Copy)]
enum Write {
    Save,
    Widget,
    TwoSaves,
}

#[rstest]
#[case::save(Write::Save)]
#[case::widget(Write::Widget)]
#[case::two_saves(Write::TwoSaves)]
fn init_pack_tab_never_freezes_the_draft(#[case] write: Write) {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let draft = match write {
        Write::Save => t.save_pack(false).unwrap(),
        Write::Widget => {
            t.set_widget(None, "card", CARD_V2.into()).unwrap();
            crate::test_support::edit_of(&t).session_version.unwrap()
        }
        Write::TwoSaves => {
            t.save_pack(false).unwrap();
            t.apply("x", vec![rename_title("acme 3")], None).unwrap();
            t.save_pack(false).unwrap()
        }
    };
    assert_that!(draft_of(&t, acme, &draft)).is_equal_to(DraftWrite::Replace(draft));
}

#[test]
fn init_pack_tab_of_a_deck_only_pack_is_refused() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, _acme) = opener_on(data.path());
    let d = adopt_deck_only_pack(&mut s);
    let seed = s.take_preview(BASE_PACK).unwrap();
    let lease = PackLease::acquire(&scope_of(&s), d.id, "d", "d").unwrap();
    let mut t = session_on(data.path());
    let err = t.init_pack_tab(*seed, lease, d.id, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::DeckOnly);
}

#[test]
fn init_pack_tab_on_a_used_session_is_refused() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let seed = s.take_preview(acme).unwrap();
    let lease = PackLease::acquire(&scope_of(&s), acme, "acme", "acme").unwrap();
    let mut t = session_on(data.path());
    t.apply("titre", vec![rename_title("Utilisé")], None).unwrap();
    let err = t.init_pack_tab(*seed, lease, acme, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackTabNotFresh);
    assert_that!(err.localize(&t.localizer)).is_equal_to("L'onglet du pack n'a pas pu être préparé.".to_owned());
}

#[test]
fn init_pack_tab_failure_frees_the_lease() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let unknown = Uuid::new_v4();
    let seed = s.take_preview(acme).unwrap();
    let lease = PackLease::acquire(&scope_of(&s), unknown, "x", "x").unwrap();
    let mut t = session_on(data.path());
    assert_that!(t.init_pack_tab(*seed, lease, unknown, None).unwrap_err().kind())
        .is_equal_to(SessionErrorKind::UnknownPack);
    assert_unleased(&scope_of(&s), unknown);
}

/// A pack tab on `acme` given a lease of Base: the refusal and the two packs' scope.
fn init_with_a_lease_of_base(data: &std::path::Path) -> (crate::SessionError, crate::pack_lease::LeaseScope, Uuid) {
    let (mut s, _deck, acme) = opener_on(data);
    let seed = s.take_preview(acme).unwrap();
    let lease = PackLease::acquire(&scope_of(&s), BASE_PACK, "Base", "Base").unwrap();
    let mut t = session_on(data);
    (t.init_pack_tab(*seed, lease, acme, None).unwrap_err(), scope_of(&s), acme)
}

#[test]
fn init_pack_tab_with_a_lease_of_another_pack_is_refused() {
    let data = fresh_dir("oku-slides-open-");
    let (err, _, _) = init_with_a_lease_of_base(data.path());
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::LeaseMissing);
}

#[test]
fn init_pack_tab_refusal_leaves_no_lease_on_either_pack() {
    let data = fresh_dir("oku-slides-open-");
    let (_, scope, acme) = init_with_a_lease_of_base(data.path());
    assert_unleased(&scope, acme);
    assert_unleased(&scope, BASE_PACK);
}

#[test]
fn init_pack_tab_with_a_widget_opens_its_sub_tab() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let seed = s.take_preview(acme).unwrap();
    let lease = PackLease::acquire(&scope_of(&s), acme, "acme", "acme").unwrap();
    let mut t = session_on(data.path());
    t.init_pack_tab(*seed, lease, acme, Some("card")).unwrap();
    assert_that!((t.ui.widgets.clone(), t.ui.widget.clone()))
        .is_equal_to((vec!["card".to_owned()], Some("card".to_owned())));
}

#[test]
fn open_widget_tab_of_an_unknown_widget_is_refused() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let before = t.ui.clone();
    assert_that!(t.open_widget_tab("nope").unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownWidget);
    assert_that!((t.ui.widgets.clone(), t.ui.widget.clone())).is_equal_to((before.widgets, before.widget));
}

#[rstest]
#[case::oldest_non_active((0..MAX_WIDGET_TABS).map(|i| format!("w{i}")).collect(), Some("w0"), MAX_WIDGET_TABS, "w1")]
#[case::active_when_alone(vec!["w0".to_owned()], Some("w0"), 1, "w0")]
fn open_widget_tab_makes_room_in_a_full_list(
    #[case] mut widgets: Vec<String>,
    #[case] active: Option<&str>,
    #[case] cap: usize,
    #[case] dropped: &str,
) {
    let before = widgets.len();
    make_room(&mut widgets, active, cap);
    assert_that!(widgets.len()).is_equal_to(before - 1);
    assert_that!(widgets.iter().any(|w| w == dropped)).is_false();
}

#[test]
fn open_widget_tab_persists_the_sub_tab() {
    let data = fresh_dir("oku-slides-open-");
    let dir = fresh_dir("oku-slides-open-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    assert_that!(t.resume_fresh(dir.path().to_path_buf())).is_true();
    t.open_widget_tab("card").unwrap();
    t.flush_resume();
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    assert_that!(state["ui"]["widget"]).is_equal_to(serde_json::json!("card"));
}

#[test]
fn enter_pack_in_a_pack_tab_is_refused() {
    let data = fresh_dir("oku-slides-open-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let err = t.enter_pack(BASE_PACK, None, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InPackTab);
    assert_that!(editing(&t)).is_equal_to(Some(acme));
}
