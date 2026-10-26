//! Pack-tab fixtures for the tests: a library pack, an opener deck, read-only editors.
//!
//! Built through public calls where a request exists; assets and fonts are
//! added directly, as no request adds them.

use std::path::Path;
use std::sync::Arc;

use oku_slides_core::{Namespace, Op, PackVersion, SourceBundle};
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
pub(crate) fn acme_pack(s: &mut Session) -> (Uuid, Namespace) {
    ok(s.edit_pack(BASE_PACK), "edit base");
    ok(s.apply("acme", vec![rename_title("acme")], None), "rename");
    ok(s.save_pack(true), "save as new");
    ok(s.close_pack(), "close");
    let acme = s
        .snapshot()
        .widgets
        .iter()
        .find(|(_, pin)| !PackLibrary::is_builtin(pin.id))
        .map(|(ns, pin)| (pin.id, ns.clone()));
    let (id, ns) = acme.unwrap_or_else(|| panic!("the deck pins the copy"));
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

/// An opener deck in `s`: a saved file, `acme` with a `card`, Pro, a token, a chart, an asset and a font.
///
/// The returned folder holds the deck's file: the caller keeps it.
pub(crate) fn opener_deck(s: &mut Session) -> tempfile::TempDir {
    ok(s.new_deck(), "new deck");
    let folder = fresh_dir("oku-slides-opener-");
    let path = folder.path().join(format!("opener{}", crate::EXTENSION));
    ok(s.save(Some(&path)), "save");
    assert_that!(s.doc.path.as_deref()).is_equal_to(Some(path.as_path()));
    assert_that!(s.doc.saved_hash.is_some()).is_true();
    let (_, acme_ns) = acme_pack(s);
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

/// Asserts that nobody holds `pack`'s lease in `scope`.
pub(crate) fn assert_unleased(scope: &LeaseScope, pack: Uuid) {
    assert_that!(pack_leased(scope, pack)).is_none();
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
    /// The holder: an in-deck workshop of `pack`.
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

/// The holder of `pack` on `data`: a workshop that saved once, its draft `h` unfrozen.
fn holder_of(data: &Path, pack: Uuid) -> (Session, PackVersion) {
    let mut holder = session_on(data);
    ok(holder.edit_pack(pack), "holder workshop");
    ok(holder.apply("h", vec![rename_title("acme h")], None), "holder edit");
    let h = ok(holder.save_pack(false), "holder save");
    assert_that!(holder.library.draft_write(&draft_lock(), pack, Some(&h))).is_equal_to(DraftWrite::Replace(h.clone()));
    (holder, h)
}

/// Resumes `dir` read-only in a new session on `data` (asserted).
fn resumed_read_only(data: &Path, dir: &Path) -> Session {
    let mut tab = session_on(data);
    assert_that!(tab.resume_checked(dir.to_path_buf())).is_equal_to(Resumed::Restored);
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
    /// The editor: a pack tab, or a deck tab's workshop.
    pub(crate) editor: Session,
    /// Its host.
    pub(crate) host: crate::PackHost,
    /// Its resume folder.
    pub(crate) dir: tempfile::TempDir,
    /// The opener deck's folder.
    pub(crate) deck_dir: tempfile::TempDir,
    /// The edited pack.
    pub(crate) pack: Uuid,
}

/// A live editor of `acme` on `data` (see `dormant_editor`), its state not yet persisted.
pub(crate) fn live_editor(data: &Path, host: crate::PackHost) -> Live {
    let mut opener = session_on(data);
    let deck_dir = opener_deck(&mut opener);
    let pack = acme_of(&opener);
    let dir = fresh_dir("oku-slides-dormant-");
    let mut editor = match host {
        crate::PackHost::Tab => pack_tab_from(&mut opener, data, pack),
        crate::PackHost::Deck => opener,
    };
    assert_that!(editor.resume_fresh(dir.path().to_path_buf())).is_true();
    if host == crate::PackHost::Deck {
        ok(editor.handle(Request::EditPack { pack }), "workshop");
    }
    Live { editor, host, dir, deck_dir, pack }
}

/// `live` persisted and gone; a workshop's state made a version-3 one without `host`.
pub(crate) fn park(live: Live) -> Dormant {
    let Live { mut editor, host, dir, deck_dir, pack } = live;
    let scope = scope_of(&editor);
    editor.persist(true);
    editor.flush_resume();
    drop(editor);
    assert_unleased(&scope, pack);
    if host == crate::PackHost::Deck {
        edit_state(dir.path(), |state| {
            assert_that!(state["version"]).is_equal_to(json!(3));
            let removed = state["mode"]["pack"].as_object_mut().and_then(|m| m.remove("host"));
            assert_that!(removed).is_some();
        });
    }
    Dormant { dir, deck_dir, pack }
}

/// A dormant editor of `acme` on `data`: a pack tab (host `Tab`) or a deck tab's workshop
/// whose `state.json` is a version-3 one without `host` (host `Deck`), each after one widget write.
pub(crate) fn dormant_editor(data: &Path, host: crate::PackHost) -> Dormant {
    let mut live = live_editor(data, host);
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
    read_only(data, crate::PackHost::Tab)
}

/// A deck tab's workshop (a version-3 state without `host`) restored while another editor holds its pack.
pub(crate) fn read_only_workshop(data: &Path) -> ReadOnly {
    read_only(data, crate::PackHost::Deck)
}

/// A dormant editor of `host` restored read-only behind a holder.
pub(crate) fn read_only(data: &Path, host: crate::PackHost) -> ReadOnly {
    let Dormant { dir, deck_dir, pack } = dormant_editor(data, host);
    let (holder, h) = holder_of(data, pack);
    let tab = resumed_read_only(data, dir.path());
    ReadOnly { tab, holder, dir, deck_dir, pack, h }
}

/// A deck in `s` pinning `acme` at an unfrozen widget draft `v`, and a fresh session `t` on `data`
/// that installed a copy of it through `install_document` (never adopting).
pub(crate) fn adopting_deck(s: &mut Session, data: &Path) -> Session {
    let (acme, acme_ns) = acme_pack(s);
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
    let deck_dir = opener_deck(&mut s);
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

/// The edited pack of `s` and its host.
pub(crate) fn editing(s: &Session) -> Option<(Uuid, crate::PackHost)> {
    s.editing_view().map(|e| (e.pack, e.host))
}
