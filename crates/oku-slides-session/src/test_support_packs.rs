//! Pack-tab fixtures for the tests: a library pack, an opener deck, read-only editors.
//!
//! Built through public calls where a request exists; assets and fonts are
//! added directly, as no request adds them.

use std::path::Path;
use std::sync::Arc;

use oku_slides_core::{Namespace, Op, PackPin, PackVersion, SourceBundle};
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::deck_closure::combined_pins;
use crate::library::{DraftWrite, draft_lock};
use crate::pack_lease::{LeaseScope, PackLease, pack_leased};
use crate::resume::Resumed;
use crate::test_support::{edit_state, fresh_dir, rename_title, session_on};
use crate::{BASE_PACK, Document, PackLibrary, Request, Session};

/// A minimal block widget `card` (the tests' `quote`, renamed).
pub(crate) const CARD_SRC: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("card", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// `card` rewritten: its body in bold.
pub(crate) const CARD_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("card", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", strong(args.body)))
"#;

/// The bytes of a 2 × 2 red PNG.
pub(crate) fn png_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 2, 2);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&[255, 0, 0].repeat(4)).unwrap();
    out
}

/// Panics with the session error's line.
pub(crate) fn ok<T>(r: crate::Result<T>, what: &str) -> T {
    r.unwrap_or_else(|e| panic!("{what}: {}", e.one_line()))
}

/// A non-built-in library widget pack `acme` (a copy of Base) the deck of `s` now pins.
///
/// The copy is saved from a pack tab of Base on `data`, the library of `s`.
pub(crate) fn acme_pack(s: &mut Session, data: &Path) -> (Uuid, Namespace) {
    let mut tab = pack_tab_of(data, BASE_PACK);
    ok(tab.apply("acme", vec![rename_title("acme")], None), "rename");
    ok(tab.save_pack(true), "save as new");
    let id = editing(&tab).unwrap_or_else(|| panic!("the tab edits the copy"));
    drop(tab);
    let stored = ok(s.pack_base(id), "the copy");
    let pin = PackPin { id, version: stored.version, requires: stored.manifest.requires.clone() };
    let ns = stored.manifest.namespace;
    let add = Op::AddPack { kind: oku_slides_core::PackKind::Widgets, namespace: ns.clone(), pin };
    ok(s.apply("acme", vec![add], None), "pin acme");
    // The deck adopts what it pins, as it did leaving the workshop.
    s.adopt_pins();
    assert_that!(s.library.current(id).is_some()).is_true();
    (id, ns)
}

/// The deck of `s` pins `acme` (`acme_pack`): its id.
pub(crate) fn acme_of(s: &Session) -> Uuid {
    let pins = s.user_deck().widgets.values();
    pins.map(|pin| pin.id).find(|id| !PackLibrary::is_builtin(*id)).unwrap_or_else(|| panic!("acme pinned"))
}

/// The lease scope of `s`'s library.
pub(crate) fn scope_of(s: &Session) -> LeaseScope {
    s.library.lease_scope().cloned().unwrap_or_else(|| panic!("a library with a lease scope"))
}

/// The theme lease scope of `s`'s library.
pub(crate) fn theme_scope_of(s: &Session) -> LeaseScope {
    s.library.theme_scope().cloned().unwrap_or_else(|| panic!("a theme library with a lease scope"))
}

/// An opener deck in `s`: a saved file, `acme` with a `card`, Pro, a token, a chart, an asset and a font.
///
/// The returned folder holds the deck's file: the caller keeps it. `data` is the library of `s`.
pub(crate) fn opener_deck(s: &mut Session, data: &Path) -> tempfile::TempDir {
    ok(s.new_deck(), "new deck");
    let folder = fresh_dir("oku-slides-opener-");
    let path = folder.path().join(format!("opener{}", crate::EXTENSION));
    ok(s.save(Some(&path)), "save");
    assert_that!(s.doc.path.as_deref()).is_equal_to(Some(path.as_path()));
    assert_that!(s.doc.saved_hash.is_some()).is_true();
    let (_, acme_ns) = acme_pack(s, data);
    ok(s.set_widget(Some(&acme_ns), "card", CARD_SRC.into()), "card");
    s.end_widget_session();
    ok(s.handle(Request::SetTheme { theme: "pro".into() }), "pro");
    let token = Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#ff0000") };
    ok(s.handle(Request::Apply { label: "token".into(), ops: vec![token], merge: None }), "token");
    let chart: Op = serde_json::from_value(json!({
        "op": "add_element", "slot": { "owner": { "slide": "s-1" }, "param": "blocks" }, "defaults": true,
        "element": { "id": "c-1", "kind": "base/chart", "params": { "option": {
            "dataset": { "source": [["Mois", "Ventes"], ["Jan", 3], ["Fév", 5]] },
            "xAxis": { "type": "category" }, "series": [{ "type": "bar" }],
        } } },
    }))
    .unwrap();
    ok(s.handle(Request::Apply { label: "chart".into(), ops: vec![chart], merge: None }), "chart");
    s.doc.sources.add_asset(png_bytes());
    s.doc.sources.fonts.insert("Inter.ttf".into(), b"police".to_vec());
    folder
}

/// Asserts that nobody holds the lease of `pack` (a pack or a theme) in `scope`.
pub(crate) fn assert_unleased(scope: &LeaseScope, pack: Uuid) {
    assert_that!(pack_leased(scope, pack)).is_none();
}

/// A fresh session on `data` turned into the pack tab of library pack `pack` (`open_pack_tab_here`).
pub(crate) fn pack_tab_of(data: &Path, pack: Uuid) -> Session {
    let mut t = session_on(data);
    ok(t.open_pack_tab_here(pack, None), "pack tab");
    t
}

/// A fresh session on `data` turned into a pack tab on `pack`, seeded from `opener`.
pub(crate) fn pack_tab_from(opener: &mut Session, data: &Path, pack: Uuid) -> Session {
    let seed = ok(opener.take_preview(pack), "preview");
    let lease = ok(PackLease::acquire(&scope_of(opener), pack, "acme", "acme"), "lease");
    let mut t = session_on(data);
    ok(t.init_pack_tab(*seed, lease, pack, None), "pack tab");
    t
}

/// An editor restored read-only: the holder holds the lease and has an unfrozen draft `h`.
pub(crate) struct ReadOnly {
    /// The restored editor, without the lease.
    pub(crate) tab: Session,
    /// The holder: a pack tab of `pack`.
    pub(crate) holder: Session,
    /// The tab's resume folder.
    pub(crate) dir: tempfile::TempDir,
    /// The opener deck's folder.
    pub(crate) deck_dir: tempfile::TempDir,
    /// The edited pack.
    pub(crate) pack: Uuid,
    /// The holder's unfrozen draft.
    pub(crate) h: PackVersion,
}

/// The holder of `pack` on `data`: a pack tab that saved once, its draft `h` unfrozen.
fn holder_of(data: &Path, pack: Uuid) -> (Session, PackVersion) {
    let mut holder = pack_tab_of(data, pack);
    ok(holder.apply("h", vec![rename_title("acme h")], None), "holder edit");
    let h = ok(holder.save_pack(false), "holder save");
    assert_that!(holder.library.draft_write(&draft_lock(), pack, Some(&h))).is_equal_to(DraftWrite::Replace(h.clone()));
    (holder, h)
}

/// Resumes `dir` read-only in a new session on `data` (asserted).
fn resumed_read_only(data: &Path, dir: &Path) -> Session {
    let mut tab = session_on(data);
    assert_that!(tab.resume_checked(dir.to_path_buf(), None)).is_equal_to(Resumed::Restored);
    let view = tab.editing_view().unwrap_or_else(|| panic!("pack mode restored"));
    assert_that!((view.read_only, view.leased)).is_equal_to((true, false));
    tab
}

/// An editor of `acme` persisted dirty in `dir`, then gone: its lease is free.
pub(crate) struct Dormant {
    /// The editor's resume folder.
    pub(crate) dir: tempfile::TempDir,
    /// The opener deck's folder.
    pub(crate) deck_dir: tempfile::TempDir,
    /// The edited pack.
    pub(crate) pack: Uuid,
}

/// A live editor of `acme` with resume on, before it goes dormant.
pub(crate) struct Live {
    /// The editor: a pack tab, standing for a deck tab's workshop when made by `live_workshop`.
    pub(crate) editor: Session,
    /// Its resume folder; for a workshop, the opener deck's, where `park` writes the workshop state.
    pub(crate) dir: tempfile::TempDir,
    /// The opener deck's folder.
    pub(crate) deck_dir: tempfile::TempDir,
    /// The edited pack.
    pub(crate) pack: Uuid,
    /// A workshop's pack tab resume folder, folded into `dir` by `park`.
    pub(crate) tab_dir: Option<tempfile::TempDir>,
}

/// A live pack tab of `acme` on `data` (see `dormant_editor`), its state not yet persisted.
pub(crate) fn live_editor(data: &Path) -> Live {
    live(data, false)
}

/// A live pack tab of `acme` whose opener deck persisted in `dir` first; `park` folds them together.
///
/// It stands for an older version's workshop, which this build no longer writes.
pub(crate) fn live_workshop(data: &Path) -> Live {
    live(data, true)
}

/// A live pack tab of `acme` on `data`; with `workshop`, its opener deck persisted in `dir`.
fn live(data: &Path, workshop: bool) -> Live {
    let mut opener = session_on(data);
    let deck_dir = opener_deck(&mut opener, data);
    let pack = acme_of(&opener);
    let dir = fresh_dir("oku-slides-dormant-");
    let mut editor = pack_tab_from(&mut opener, data, pack);
    let tab_dir = workshop.then(|| {
        assert_that!(opener.resume_fresh(dir.path().to_path_buf())).is_true();
        opener.persist(true);
        opener.flush_resume();
        fresh_dir("oku-slides-dormant-tab-")
    });
    drop(opener);
    let resume = tab_dir.as_ref().map_or(dir.path(), tempfile::TempDir::path);
    assert_that!(editor.resume_fresh(resume.to_path_buf())).is_true();
    Live { editor, dir, deck_dir, pack, tab_dir }
}

/// Folds the pack tab state of `tab` into the deck state of `deck`: an older version's workshop.
///
/// `deck` takes the tab's `pack/`, pack mode and UI state, at version 3 without `host`.
pub(crate) fn as_workshop(deck: &Path, tab: &Path) {
    copy_tree(&tab.join(crate::resume::PACK_DIR), &deck.join(crate::resume::PACK_DIR));
    let (mut mode, mut ui) = (json!(null), json!(null));
    edit_state(tab, |state| {
        assert_that!(state["version"]).is_equal_to(json!(5));
        (mode, ui) = (state["mode"].clone(), state["ui"].clone());
    });
    assert_that!(mode["pack"].get("host")).is_none();
    edit_state(deck, |state| {
        state["version"] = json!(3);
        (state["mode"], state["ui"]) = (mode, ui);
    });
}

/// Copies the folder `from` to `to`, recursively.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap_or_else(|e| panic!("{}: {e}", to.display()));
    for entry in std::fs::read_dir(from).unwrap_or_else(|e| panic!("{}: {e}", from.display())) {
        let path = entry.unwrap_or_else(|e| panic!("{e}")).path();
        let target = to.join(path.file_name().unwrap_or_else(|| panic!("a file name")));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
    }
}

/// `live` persisted and gone; a workshop folded into a version-3 state without `host` (`as_workshop`).
pub(crate) fn park(live: Live) -> Dormant {
    let Live { mut editor, dir, deck_dir, pack, tab_dir, .. } = live;
    let scope = scope_of(&editor);
    editor.persist(true);
    editor.flush_resume();
    drop(editor);
    assert_unleased(&scope, pack);
    if let Some(tab_dir) = tab_dir {
        as_workshop(dir.path(), tab_dir.path());
    }
    Dormant { dir, deck_dir, pack }
}

/// A dormant pack tab of `acme` on `data`, after one widget write.
pub(crate) fn dormant_editor(data: &Path) -> Dormant {
    dormant(live_editor(data))
}

/// A dormant deck tab's workshop of `acme` on `data`, after one widget write.
///
/// Its `state.json` is a version-3 one without `host`, as older versions wrote.
pub(crate) fn dormant_workshop(data: &Path) -> Dormant {
    dormant(live_workshop(data))
}

/// `live` after one widget write, parked.
fn dormant(mut live: Live) -> Dormant {
    let card = Request::SetWidget { pack: None, name: "card".into(), source: CARD_V2.into() };
    ok(live.editor.handle(card), "write");
    park(live)
}

/// Deletes `pack` from the library of `data`, as another deck tab would.
pub(crate) fn delete_from_library(data: &Path, pack: Uuid) {
    let mut other = session_on(data);
    ok(other.handle(Request::DeletePack { pack }), "delete");
    assert_that!(other.library.current(pack)).is_none();
}

/// Forgets that `version` of `pack` was adopted: the library may replace it again.
pub(crate) fn unfreeze(data: &Path, pack: Uuid, version: &PackVersion) {
    let marker = data.join("widgets").join(pack.to_string()).join("versions").join(version.hex()).join("adopted");
    std::fs::remove_file(&marker).unwrap_or_else(|e| panic!("{}: {e}", marker.display()));
}

/// A dirty pack tab restored while another editor holds its pack.
pub(crate) fn read_only_pack_tab(data: &Path) -> ReadOnly {
    let Dormant { dir, deck_dir, pack } = dormant_editor(data);
    let (holder, h) = holder_of(data, pack);
    let tab = resumed_read_only(data, dir.path());
    ReadOnly { tab, holder, dir, deck_dir, pack, h }
}

/// A deck in `s` pinning `acme` at an unfrozen widget draft `v`, and a fresh session `t` on `data`
/// that installed a copy of it through `install_document` (never adopting).
pub(crate) fn adopting_deck(s: &mut Session, data: &Path) -> Session {
    let (acme, acme_ns) = acme_pack(s, data);
    ok(s.set_widget(Some(&acme_ns), "card", CARD_SRC.into()), "card");
    let v = s.user_deck().widgets[&acme_ns].version.clone();
    assert_that!(s.library.draft_write(&draft_lock(), acme, Some(&v))).is_equal_to(DraftWrite::Replace(v.clone()));
    let mut t = session_on(data);
    let deck = s.user_deck().clone();
    let packs = s.user_blobs().packs.clone();
    let (widgets, notice) = ok(t.resolve_widgets(&combined_pins(&deck), &packs), "widgets");
    let mut bundle = SourceBundle::new(deck, Arc::clone(&widgets.registry)).unwrap_or_else(|e| panic!("bundle: {e}"));
    bundle.packs = packs;
    bundle.fonts.clone_from(&s.user_blobs().fonts);
    let doc = Document::new(bundle, None, &t.doc_defaults);
    ok(t.install_document(doc, widgets, notice), "install");
    t
}

/// An opener deck on `data` (`opener_deck`): the session, its file's folder, and `acme`.
pub(crate) fn opener_on(data: &Path) -> (Session, tempfile::TempDir, Uuid) {
    let mut s = session_on(data);
    let deck_dir = opener_deck(&mut s, data);
    let acme = acme_of(&s);
    (s, deck_dir, acme)
}

/// The snapshot of `s`, as the interface reads it.
pub(crate) fn snapshot_json(s: &Session) -> String {
    serde_json::to_string(&s.snapshot()).unwrap()
}

/// The library's way of saving `draft` of `pack`: `Replace` while unfrozen.
pub(crate) fn draft_of(s: &Session, pack: Uuid, draft: &PackVersion) -> DraftWrite {
    s.library.draft_write(&draft_lock(), pack, Some(draft))
}

/// The pack `s` edits, if any.
pub(crate) fn editing(s: &Session) -> Option<Uuid> {
    s.editing_view().map(|e| e.pack)
}

/// The owner deck's tab id in the deck-pack fixtures.
pub(crate) const OWNER_TAB: &str = "t-1";

/// The deck-pack tab's id in the deck-pack fixtures.
pub(crate) const LINKED_TAB: &str = "t-2";

/// A deck-pack tab `t-2` of `owner` (tab `t-1`) on its deck-only `pack`, linked by `link_deck_pack`.
///
/// The tab's session works on `data`, the owner's data folder and library, kept by the caller.
pub(crate) fn deck_pack_tab_of(owner: &mut Session, data: &Path, pack: Uuid) -> Session {
    let seed = ok(owner.link_deck_pack(LINKED_TAB, "Importé", pack), "link");
    let mut t = session_on(data);
    ok(t.init_deck_pack_tab(seed, OWNER_TAB, pack, None), "deck-pack tab");
    t
}

/// Forwards the due send of `tab` (`tab_id`) to `owner`, then settles its answer as the workspace does.
pub(crate) fn forward_to_owner(
    tab: &mut Session,
    tab_id: &str,
    owner: &mut Session,
) -> crate::Result<crate::deck_pack_receive::Received> {
    let send = tab.take_deck_pack_send().unwrap_or_else(|| panic!("a due send"));
    let answer = owner.receive_deck_pack(tab_id, &send);
    crate::deck_pack_receive::settle_answer(tab, send.seq, &answer);
    answer
}

/// An owner deck on `data` pinning a deck-only widgets pack under `deck`, linked (pending) to tab `t-2`.
pub(crate) fn linked_owner(data: &Path) -> (Session, PackPin) {
    let mut owner = crate::test_support::opened_on(data);
    let pin = crate::test_support::adopt_deck_only_pack(&mut owner);
    drop(ok(owner.link_deck_pack(LINKED_TAB, "Importé", pin.id), "link"));
    (owner, pin)
}

/// The link of `tab` in `owner`, which must be there.
pub(crate) fn link_of<'a>(owner: &'a Session, tab: &str) -> &'a crate::deck_pack_link::DeckLink {
    owner.deck_links.get(tab).unwrap_or_else(|| panic!("a link of {tab}"))
}

/// The files of `version` embedded in `owner`'s deck.
pub(crate) fn embedded(owner: &Session, version: &PackVersion) -> oku_slides_core::PackFiles {
    owner.user_blobs().packs.get(version).cloned().unwrap_or_else(|| panic!("version embedded"))
}

/// `manifest` with `widgets` as files of `kind`.
pub(crate) fn files_with(
    manifest: &crate::pack::PackManifest,
    kind: oku_slides_core::PackKind,
    widgets: &std::collections::BTreeMap<oku_slides_core::WidgetFile, String>,
) -> oku_slides_core::PackFiles {
    manifest.to_files(kind, widgets.clone()).unwrap_or_else(|e| panic!("manifest: {e}"))
}

/// A send as `tab`'s editor would make it: a new version of the linked pack, on the link's parent.
///
/// Its seq follows the link's last; its files change the description of the expected version.
pub(crate) fn valid_send(owner: &Session, tab: &str) -> crate::deck_pack_tab::DeckPackSend {
    let link = link_of(owner, tab);
    let parent = match &link.parent {
        crate::deck_pack_link::LinkParent::Known(parent) => parent.clone(),
        crate::deck_pack_link::LinkParent::Reseeded { pin, .. } => Some(pin.clone()),
    };
    let files = embedded(owner, &link.expect);
    let mut manifest = crate::pack::PackManifest::from_files(&files).unwrap_or_else(|e| panic!("manifest: {e}"));
    manifest.parent.clone_from(&parent);
    manifest.description.push('+');
    let base = crate::deck_pack_tab::SendBase { pack: link.pack, parent };
    let files = files_with(&manifest, files.kind(), files.widgets());
    crate::deck_pack_tab::DeckPackSend { seq: link.last_seq.map_or(1, |seq| seq + 1), base, files }
}

/// `valid_send` saved as a copy: a fresh id, no parent, the original's namespace.
pub(crate) fn copy_send(owner: &Session, tab: &str) -> crate::deck_pack_tab::DeckPackSend {
    let send = valid_send(owner, tab);
    let mut manifest = crate::pack::PackManifest::from_files(&send.files).unwrap_or_else(|e| panic!("manifest: {e}"));
    manifest.id = Uuid::new_v4();
    manifest.parent = None;
    let files = files_with(&manifest, send.files.kind(), send.files.widgets());
    crate::deck_pack_tab::DeckPackSend { files, ..send }
}

/// Receives `send` from `tab` in `owner`, which must apply it.
pub(crate) fn applied(owner: &mut Session, tab: &str, send: &crate::deck_pack_tab::DeckPackSend) {
    let received = ok(owner.receive_deck_pack(tab, send), "receive");
    assert_that!(received).is_equal_to(crate::deck_pack_receive::Received::Applied);
}

/// The deck-only layouts pack `deck-layouts` the deck of `s` gets from a slide widget write: its pin.
pub(crate) fn deck_layouts_pack(s: &mut Session) -> PackPin {
    ok(s.set_widget(None, "cover", crate::test_support::slide_widget("cover")), "deck-layouts write");
    s.end_widget_session();
    crate::test_support::pin(s, "deck-layouts").unwrap_or_else(|| panic!("deck-layouts pinned"))
}

/// What a refusal leaves as it was: the deck, its embedded versions, undo steps, undo labels and dirty flag.
pub(crate) type OwnerState = (oku_slides_core::Deck, Vec<PackVersion>, u64, (Option<String>, Option<String>), bool);

/// The state of `owner` a refusal must not change.
pub(crate) fn owner_state(owner: &Session) -> OwnerState {
    let sources = &owner.doc.sources;
    let labels = (sources.replica().undo_label(), sources.replica().redo_label());
    (sources.deck().clone(), sources.packs.keys().cloned().collect(), sources.undo_steps(), labels, owner.doc.dirty)
}

/// Receives `valid_send` from `tab` in `owner`, which must apply it; the send.
pub(crate) fn apply_valid(owner: &mut Session, tab: &str) -> crate::deck_pack_tab::DeckPackSend {
    let send = valid_send(owner, tab);
    applied(owner, tab, &send);
    send
}
