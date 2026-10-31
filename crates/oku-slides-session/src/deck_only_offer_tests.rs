use oku_slides_core::{Namespace, PackKind, PackPin, PackScope};
use speculoos::prelude::*;

use crate::Session;
use crate::deck_packs_panel::PacksEvent;
use crate::pack::PackManifest;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, library_files, ns, opened_on};

/// The deck-only pack `pin` removed from the deck of `s`.
fn removed(s: &mut Session, pin: &PackPin) {
    let ns = s.deck().widgets.iter().find(|(_, p)| *p == pin).map(|(n, _)| n.clone()).unwrap();
    s.remove_deck_pack(PackKind::Widgets, &ns).unwrap();
}

/// A child version of the deck-only pack `pin`, embedded in the sources of `s`, created at `created`.
fn embed_child(s: &mut Session, pin: &PackPin, created: &str) -> PackPin {
    let files = s.user_blobs().packs.get(&pin.version).cloned().unwrap();
    let mut m = PackManifest::from_files(&files).unwrap();
    m.parent = Some(pin.version.clone());
    created.clone_into(&mut m.created);
    m.description = format!("child {created}");
    let child = m.to_files(PackKind::Widgets, files.widgets().clone()).unwrap();
    let out = PackPin { id: m.id, version: child.version(), requires: m.requires.clone() };
    s.doc.sources.add_pack(child);
    out
}

#[test]
fn deck_only_choices_of_a_pinned_pack_are_empty() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-pinned-").path());
    adopt_deck_only_pack(&mut s);
    assert_that!(s.deck_only_choices(PackKind::Widgets)).is_empty();
}

#[test]
fn deck_only_choices_after_removal_offer_the_pack_with_its_name() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-removed-").path());
    let pin = adopt_deck_only_pack(&mut s);
    removed(&mut s, &pin);
    let choices = s.deck_only_choices(PackKind::Widgets);
    assert_that!(choices).has_length(1);
    assert_that!(choices[0].namespace).is_equal_to(Namespace::deck());
    assert_that!(choices[0].name.as_str()).is_equal_to("Importé");
    assert_that!(choices[0].pin).is_equal_to(pin);
}

#[test]
fn deck_only_choices_of_layouts_leave_out_a_widgets_pack() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-kind-").path());
    let pin = adopt_deck_only_pack(&mut s);
    removed(&mut s, &pin);
    assert_that!(s.deck_only_choices(PackKind::Layouts)).is_empty();
}

#[test]
fn deck_only_choices_take_the_leaf_over_its_parent() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-leaf-").path());
    let pin = adopt_deck_only_pack(&mut s);
    // An older `created` than the parent's: the leaf wins on the parent chain, not on time.
    let child = embed_child(&mut s, &pin, "2000-01-01T00:00:00Z");
    removed(&mut s, &pin);
    let choices = s.deck_only_choices(PackKind::Widgets);
    assert_that!(choices).has_length(1);
    assert_that!(choices[0].pin.version).is_equal_to(child.version);
}

#[test]
fn deck_only_choices_between_two_leaves_take_the_latest_created() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-two-leaves-").path());
    let pin = adopt_deck_only_pack(&mut s);
    embed_child(&mut s, &pin, "2001-01-01T00:00:00Z");
    let late = embed_child(&mut s, &pin, "2099-01-01T00:00:00Z");
    removed(&mut s, &pin);
    let choices = s.deck_only_choices(PackKind::Widgets);
    assert_that!(choices).has_length(1);
    assert_that!(choices[0].pin.version).is_equal_to(late.version);
}

#[test]
fn deck_packs_addable_lists_the_removed_deck_only_pack() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-addable-").path());
    let pin = adopt_deck_only_pack(&mut s);
    removed(&mut s, &pin);
    let view = s.deck_packs().unwrap();
    let addable = view.addable.get(&PackKind::Widgets).cloned().unwrap_or_default();
    assert_that!(addable).contains((Namespace::deck(), "Importé".to_owned()));
}

#[test]
fn packs_action_add_of_a_deck_only_choice_pins_it_and_writes_no_library() {
    let data = fresh_dir("oku-slides-deck-only-add-");
    let mut s = opened_on(data.path());
    let pin = adopt_deck_only_pack(&mut s);
    removed(&mut s, &pin);
    let before = library_files(data.path());
    s.packs_action(PacksEvent::AddPack { kind: PackKind::Widgets, namespace: Namespace::deck() }).unwrap();
    assert_that!(s.deck().widgets.get(&Namespace::deck())).is_equal_to(Some(&pin));
    assert_that!(library_files(data.path())).is_equal_to(before);
}

#[test]
fn pin_deck_only_is_one_undo_step() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-undo-").path());
    let pin = adopt_deck_only_pack(&mut s);
    removed(&mut s, &pin);
    let choice = s.deck_only_choice(PackKind::Widgets, &Namespace::deck()).unwrap();
    s.pin_deck_only(PackKind::Widgets, choice).unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().widgets.contains_key(&Namespace::deck())).is_false();
}

#[test]
fn deck_only_choice_without_such_a_choice_is_none() {
    let s = opened_on(fresh_dir("oku-slides-deck-only-unknown-").path());
    assert_that!(s.deck_only_choice(PackKind::Widgets, &ns("acme"))).is_none();
}

#[test]
fn deck_only_choices_leave_out_a_library_scoped_version() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-scope-").path());
    let pin = adopt_deck_only_pack(&mut s);
    let files = s.user_blobs().packs.get(&pin.version).cloned().unwrap();
    let mut m = PackManifest::from_files(&files).unwrap();
    m.scope = PackScope::Library;
    m.id = uuid::Uuid::new_v4();
    s.doc.sources.add_pack(m.to_files(PackKind::Widgets, files.widgets().clone()).unwrap());
    removed(&mut s, &pin);
    let ids: Vec<uuid::Uuid> = s.deck_only_choices(PackKind::Widgets).iter().map(|c| c.pin.id).collect();
    assert_that!(ids).is_equal_to(vec![pin.id]);
}

/// A version of the deck-only pack `pin`, changed by `change`, embedded in the sources of `s`.
fn embed_changed(s: &mut Session, pin: &PackPin, change: impl FnOnce(&mut PackManifest)) {
    let files = s.user_blobs().packs.get(&pin.version).cloned().unwrap();
    let mut m = PackManifest::from_files(&files).unwrap();
    change(&mut m);
    s.doc.sources.add_pack(m.to_files(PackKind::Widgets, files.widgets().clone()).unwrap());
}

#[test]
fn deck_only_choices_leave_out_a_builtin_namespace_of_another_id() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-r15-").path());
    let pin = adopt_deck_only_pack(&mut s);
    embed_changed(&mut s, &pin, |m| {
        m.id = uuid::Uuid::new_v4();
        m.namespace = ns("essentiel");
    });
    removed(&mut s, &pin);
    let spaces: Vec<Namespace> = s.deck_only_choices(PackKind::Widgets).into_iter().map(|c| c.namespace).collect();
    assert_that!(spaces).is_equal_to(vec![Namespace::deck()]);
}

#[test]
fn deck_only_choices_leave_out_a_version_followed_by_a_library_scoped_child() {
    let mut s = opened_on(fresh_dir("oku-slides-deck-only-library-child-").path());
    let pin = adopt_deck_only_pack(&mut s);
    embed_changed(&mut s, &pin, |m| {
        m.parent = Some(pin.version.clone());
        m.scope = PackScope::Library;
    });
    removed(&mut s, &pin);
    assert_that!(s.deck_only_choices(PackKind::Widgets)).is_empty();
}
