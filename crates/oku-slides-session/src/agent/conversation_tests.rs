use speculoos::prelude::*;

use super::{AgentScope, SavedConversation, ScopeMove};
use crate::agent::forms::frozen_choice;
use crate::pack::DEFAULT_PACK;
use crate::test_support::{fresh_dir, session_on};

/// Une surface du fil, figée, rangée dans le tour `turn`.
fn frozen(id: &str, turn: u32) -> crate::agent::Surface {
    let mut s = frozen_choice(id, "Laquelle ?", Some("A"), &oku_slides_i18n::Localizer::new(Some("fr")));
    s.turn = Some(turn);
    s
}

#[test]
fn conversations_survive_a_restart() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_thread(AgentScope::Slides, vec![frozen("report-3", 1)]);
    a.keep_agent_session(AgentScope::Slides, "sess-9".into(), "empreinte".into());
    drop(a);

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    assert_that!(b.conversation(&AgentScope::Slides).cloned()).is_equal_to(Some(SavedConversation {
        scope: AgentScope::Slides,
        agent: Some("empreinte".into()),
        session: Some("sess-9".into()),
        thread: vec![frozen("report-3", 1)],
    }));
}

#[test]
fn forget_conversation_removes_its_file_and_returns_its_session() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_agent_session(AgentScope::Slides, "sess-9".into(), "empreinte".into());
    a.flush_resume();
    assert_that!(dir.join("agent/slides.json").exists()).is_true();
    assert_that!(a.forget_conversation(&AgentScope::Slides)).is_equal_to(Some("sess-9".to_owned()));
    a.flush_resume();
    assert_that!(dir.join("agent/slides.json").exists()).is_false();
    assert_that!(a.conversations()).is_empty();
}

#[test]
fn unreadable_conversation_file_is_skipped_and_the_others_kept() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_thread(AgentScope::Slides, vec![frozen("report-1", 1)]);
    drop(a);
    // Sous un nom valide : seul le contenu illisible le fait sauter.
    let atelier = AgentScope::Atelier { pack: DEFAULT_PACK };
    std::fs::write(dir.join("agent").join(atelier.file_name()), b"{pas du json").unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    let scopes: Vec<_> = b.conversations().into_iter().map(|c| c.scope).collect();
    assert_that!(scopes).is_equal_to(vec![AgentScope::Slides]);
}

#[test]
fn conversation_file_naming_another_scope_is_skipped() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_thread(AgentScope::Slides, vec![frozen("report-1", 1)]);
    a.flush_resume();
    drop(a);
    // Le contenu de `slides.json` sous un autre nom : il ne prend pas la place des slides.
    std::fs::rename(dir.join("agent/slides.json"), dir.join("agent/widget.x.json")).unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    assert_that!(b.conversations()).is_empty();
}

#[test]
fn saved_conversation_without_session_reads_as_none() {
    let c: SavedConversation = serde_json::from_str(r#"{"scope":{"kind":"slides"}}"#).unwrap();
    assert_that!(c.session).is_none();
    assert_that!(c.thread).is_empty();
}

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

fn widget(name: &str) -> AgentScope {
    AgentScope::Widget { pack: DEFAULT_PACK, name: name.into() }
}

/// Une session reprise dans `resume/`, en édition du pack par défaut, avec le widget `kpi`.
fn editing_with_kpi(data: &std::path::Path) -> crate::Session {
    let mut s = session_on(data);
    assert!(s.resume(data.join("resume")));
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
    s
}

#[test]
fn rename_widget_moves_its_conversation_and_notes_the_move() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = editing_with_kpi(data.path());
    s.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    s.rename_widget("kpi", "indicateur").unwrap();
    s.flush_resume();
    let agent_dir = data.path().join("resume/agent");
    assert_that!(agent_dir.join(widget("kpi").file_name()).exists()).is_false();
    assert_that!(agent_dir.join(widget("indicateur").file_name()).exists()).is_true();
    assert_that!(s.conversation(&widget("indicateur")).map(|c| c.thread.len())).is_equal_to(Some(1));
    assert_that!(s.take_scope_moves())
        .is_equal_to(vec![ScopeMove::Renamed { from: widget("kpi"), to: widget("indicateur") }]);
}

#[test]
fn renamed_conversation_is_read_back_under_its_new_name() {
    let data = fresh_dir("oku-slides-conv-");
    let mut a = editing_with_kpi(data.path());
    a.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    a.rename_widget("kpi", "indicateur").unwrap();
    drop(a);
    let mut b = session_on(data.path());
    assert!(b.resume(data.path().join("resume")));
    assert_that!(b.conversation(&widget("indicateur"))).is_some();
}

#[test]
fn remove_widget_then_undo_keeps_its_conversation() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = editing_with_kpi(data.path());
    s.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    s.remove_widget("kpi").unwrap();
    assert_that!(s.conversation(&widget("kpi"))).is_some();
    s.undo().unwrap();
    assert_that!(s.conversation(&widget("kpi"))).is_some();
}

#[test]
fn keep_thread_of_an_unknown_widget_writes_nothing() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = editing_with_kpi(data.path());
    s.keep_thread(widget("invente"), vec![frozen("report-1", 1)]);
    s.flush_resume();
    assert_that!(s.conversation(&widget("invente"))).is_none();
    assert_that!(data.path().join("resume/agent").join(widget("invente").file_name()).exists()).is_false();
}

#[test]
fn resume_prunes_only_conversations_of_widgets_missing_from_known_pack_files() {
    let data = fresh_dir("oku-slides-conv-");
    let mut a = editing_with_kpi(data.path());
    a.set_widget("disparu", KPI.replace("kpi", "disparu")).unwrap();
    a.keep_thread(widget("kpi"), vec![frozen("report-1", 1)]);
    a.keep_thread(widget("disparu"), vec![frozen("report-2", 2)]);
    let foreign = AgentScope::Widget { pack: uuid::Uuid::from_u128(42), name: "kpi".into() };
    a.keep_thread(AgentScope::Atelier { pack: DEFAULT_PACK }, vec![frozen("report-4", 4)]);
    a.set_widget("disparu", String::new()).unwrap(); // supprimé : sa conversation reste jusqu'à la reprise
    a.flush_resume();
    drop(a);
    // Une conversation d'un pack inconnu ici (deck d'une autre machine) : écrite à la main.
    let path = data.path().join("resume/agent").join(foreign.file_name());
    let json = serde_json::json!({ "version": 1, "scope": foreign, "thread": [] });
    std::fs::write(&path, json.to_string()).unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(data.path().join("resume")));
    let mut scopes: Vec<_> = b.conversations().into_iter().map(|c| c.scope).collect();
    scopes.sort();
    let mut expected = vec![AgentScope::Atelier { pack: DEFAULT_PACK }, foreign, widget("kpi")];
    expected.sort();
    assert_that!(scopes).is_equal_to(expected);
    b.flush_resume();
    assert_that!(data.path().join("resume/agent").join(widget("disparu").file_name()).exists()).is_false();
}
