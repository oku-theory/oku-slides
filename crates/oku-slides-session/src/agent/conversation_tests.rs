use speculoos::prelude::*;

use super::SavedConversation;
use crate::agent::forms::frozen_choice;
use crate::test_support::{fresh_dir, session_on};

/// Une surface du fil, figée, rangée dans le tour `turn`.
fn frozen(id: &str, turn: u32) -> crate::agent::Surface {
    let mut s = frozen_choice(id, "Laquelle ?", Some("A"), &oku_slides_i18n::Localizer::new(Some("fr")));
    s.turn = Some(turn);
    s
}

#[test]
fn keep_thread_then_conversation_returns_the_thread() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = session_on(data.path());
    let thread = vec![frozen("report-1", 1)];
    s.keep_thread(thread.clone());
    assert_that!(s.conversation().map(|c| c.thread.clone())).is_equal_to(Some(thread));
}

#[test]
fn keep_agent_session_records_session_and_agent() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = session_on(data.path());
    s.keep_agent_session("sess-1".into(), "agent-a".into());
    let c = s.conversation().cloned().unwrap();
    assert_that!((c.session, c.agent)).is_equal_to((Some("sess-1".to_owned()), Some("agent-a".to_owned())));
}

#[test]
fn forget_conversation_returns_its_session_and_clears_it() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = session_on(data.path());
    s.keep_agent_session("sess-1".into(), "agent-a".into());
    assert_that!(s.forget_conversation()).is_equal_to(Some("sess-1".to_owned()));
    assert_that!(s.conversation()).is_none();
}

#[test]
fn forget_conversation_without_one_returns_none() {
    let data = fresh_dir("oku-slides-conv-");
    let mut s = session_on(data.path());
    assert_that!(s.forget_conversation()).is_none();
}

/// Écrite dans `agent/conversation.json`, relue par un onglet rouvert.
#[test]
fn conversation_survives_resume_in_conversation_json() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_thread(vec![frozen("report-3", 1)]);
    a.keep_agent_session("sess-9".into(), "empreinte".into());
    a.flush_resume();
    assert_that!(dir.join("agent/conversation.json").exists()).is_true();
    drop(a);

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    assert_that!(b.conversation().cloned()).is_equal_to(Some(SavedConversation {
        agent: Some("empreinte".into()),
        session: Some("sess-9".into()),
        thread: vec![frozen("report-3", 1)],
    }));
}

#[test]
fn forget_conversation_removes_its_file() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.keep_agent_session("sess-9".into(), "empreinte".into());
    a.flush_resume();
    assert_that!(a.forget_conversation()).is_equal_to(Some("sess-9".to_owned()));
    a.flush_resume();
    assert_that!(dir.join("agent/conversation.json").exists()).is_false();
}

/// Les fichiers par portée d'avant restent sur le disque, ignorés.
#[test]
fn resume_ignores_old_per_scope_files() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.flush_resume();
    drop(a);
    let old = dir.join("agent/slides.json");
    std::fs::create_dir_all(dir.join("agent")).unwrap();
    std::fs::write(&old, r#"{"version":1,"scope":{"kind":"slides"},"session":"vieille","thread":[]}"#).unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    assert_that!(b.conversation()).is_none();
    assert_that!(old.exists()).is_true();
}

#[test]
fn unreadable_conversation_file_is_ignored() {
    let data = fresh_dir("oku-slides-conv-");
    let dir = data.path().join("resume");
    let mut a = session_on(data.path());
    assert!(a.resume(dir.clone()));
    a.flush_resume();
    drop(a);
    std::fs::create_dir_all(dir.join("agent")).unwrap();
    std::fs::write(dir.join("agent/conversation.json"), b"{pas du json").unwrap();

    let mut b = session_on(data.path());
    assert!(b.resume(dir));
    assert_that!(b.conversation()).is_none();
}

#[test]
fn saved_conversation_without_session_reads_as_none() {
    let c: SavedConversation = serde_json::from_str("{}").unwrap();
    assert_that!(c.session).is_none();
    assert_that!(c.thread).is_empty();
}
