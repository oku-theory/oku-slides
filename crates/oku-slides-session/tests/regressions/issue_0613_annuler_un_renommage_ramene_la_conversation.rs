//! t-613 : renommer un widget déplace sa conversation avec l'agent (t-598) ;
//! annuler ce renommage (⌘Z) ne la ramenait pas sous l'ancien nom, et le
//! nettoyage de la reprise l'effaçait (son widget n'existait plus). Annuler et
//! rétablir suivent désormais le renommage, sans jamais écraser une autre
//! conversation.

use oku_slides_core::Op;
use oku_slides_session::agent::{AgentScope, ScopeMove};
use oku_slides_session::{ESSENTIEL_PACK, Session};
use speculoos::prelude::*;

use crate::common::{Data, data_dir, session_in};

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

fn widget(name: &str) -> AgentScope {
    AgentScope::Widget { pack: ESSENTIEL_PACK, name: name.into() }
}

/// La session ACP gardée pour `scope`, s'il y en a une.
fn session_of(s: &Session, scope: &AgentScope) -> Option<String> {
    s.conversation(scope).and_then(|c| c.session.clone())
}

/// Une session reprise dans `resume/`, en édition du pack par défaut, avec le widget `kpi`.
fn editing_with_kpi(data: &Data) -> Session {
    let mut s = session_in(data);
    assert!(s.resume(data.join("resume")));
    s.edit_pack(ESSENTIEL_PACK).unwrap_or_else(|e| panic!("édition : {}", e.one_line()));
    s.set_widget(None, "kpi", KPI.into()).unwrap_or_else(|e| panic!("widget : {}", e.one_line()));
    s
}

/// Une autre modification du document des modèles : l'écriture de widget
/// suivante fait sa propre étape d'annulation (les écritures d'une même séance
/// d'édition, consécutives, n'en font qu'une).
fn other_step(s: &mut Session, value: &str) {
    let op = Op::SetVar { key: "client".into(), value: serde_json::json!(value) };
    s.apply("Variable", vec![op], None).unwrap_or_else(|e| panic!("variable : {}", e.one_line()));
}

/// `kpi` renommé `indicateur` (une étape à lui), une conversation ouverte sous le nouveau nom.
fn renamed_with_a_conversation(data: &Data) -> Session {
    let mut s = editing_with_kpi(data);
    other_step(&mut s, "a");
    s.rename_widget(None, "kpi", "indicateur").unwrap_or_else(|e| panic!("renommage : {}", e.one_line()));
    s.keep_agent_session(widget("indicateur"), "sess-b".into(), "agent".into());
    drop(s.take_scope_moves());
    s
}

#[test]
fn undo_of_a_rename_brings_the_conversation_back_and_redo_moves_it_again() {
    let data = data_dir();
    let mut s = renamed_with_a_conversation(&data);

    s.undo().unwrap_or_else(|e| panic!("annuler : {}", e.one_line()));
    assert_that!(session_of(&s, &widget("kpi"))).is_equal_to(Some("sess-b".to_owned()));
    assert_that!(s.conversation(&widget("indicateur"))).is_none();
    assert_that!(s.take_scope_moves())
        .is_equal_to(vec![ScopeMove::Renamed { from: widget("indicateur"), to: widget("kpi") }]);

    s.redo().unwrap_or_else(|e| panic!("rétablir : {}", e.one_line()));
    assert_that!(session_of(&s, &widget("indicateur"))).is_equal_to(Some("sess-b".to_owned()));
    assert_that!(s.conversation(&widget("kpi"))).is_none();
    assert_that!(s.take_scope_moves())
        .is_equal_to(vec![ScopeMove::Renamed { from: widget("kpi"), to: widget("indicateur") }]);
}

#[test]
fn undo_of_a_rename_onto_the_conversation_of_a_still_known_widget_overwrites_nothing() {
    let data = data_dir();
    let mut s = renamed_with_a_conversation(&data);
    // Un nouveau `kpi`, et sa conversation ; puis il est annulé : sa conversation
    // reste, et ce `kpi` reste dans un fichier connu du pack (la version courante
    // de la bibliothèque, qu'annuler ne change pas) : vraie collision, pas une
    // conversation périmée.
    other_step(&mut s, "b");
    s.set_widget(None, "kpi", KPI.into()).unwrap_or_else(|e| panic!("widget : {}", e.one_line()));
    s.keep_agent_session(widget("kpi"), "sess-a".into(), "agent".into());
    s.undo().unwrap_or_else(|e| panic!("annuler le nouveau kpi : {}", e.one_line()));
    s.undo().unwrap_or_else(|e| panic!("annuler la variable : {}", e.one_line()));
    assert_that!(session_of(&s, &widget("kpi"))).is_equal_to(Some("sess-a".to_owned()));
    assert_that!(s.take_scope_moves()).is_empty();

    s.undo().unwrap_or_else(|e| panic!("annuler le renommage : {}", e.one_line()));
    assert_that!(session_of(&s, &widget("kpi"))).is_equal_to(Some("sess-a".to_owned()));
    assert_that!(session_of(&s, &widget("indicateur"))).is_equal_to(Some("sess-b".to_owned()));
    assert_that!(s.take_scope_moves()).is_empty();
}

/// Un autre widget, au corps différent de `kpi` : rien ne le confond avec `kpi` renommé.
const AUTRE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("autre", params: (texte: p.plain()), (args, ctx) => field(ctx, "texte", args.texte))
"#;

#[test]
fn undo_of_a_step_creating_one_widget_and_deleting_another_moves_no_conversation() {
    let data = data_dir();
    let mut s = editing_with_kpi(&data);
    other_step(&mut s, "a");
    // Une seule étape (même séance) : `autre` créé, `kpi` supprimé.
    s.set_widget(None, "autre", AUTRE.into()).unwrap_or_else(|e| panic!("widget : {}", e.one_line()));
    s.remove_widget(None, "kpi").unwrap_or_else(|e| panic!("suppression : {}", e.one_line()));
    s.keep_agent_session(widget("autre"), "sess-x".into(), "agent".into());
    drop(s.take_scope_moves());

    s.undo().unwrap_or_else(|e| panic!("annuler : {}", e.one_line()));
    assert_that!(session_of(&s, &widget("autre"))).is_equal_to(Some("sess-x".to_owned()));
    assert_that!(s.conversation(&widget("kpi"))).is_none();
    assert_that!(s.take_scope_moves()).is_empty();
}

/// La conversation d'un widget supprimé est périmée : un renommage vers son nom la remplace.
#[test]
fn rename_onto_the_stale_conversation_of_a_deleted_widget_replaces_it() {
    let data = data_dir();
    let mut s = editing_with_kpi(&data);
    let ancien = AUTRE.replace("\"autre\"", "\"indicateur\"");
    s.set_widget(None, "indicateur", ancien).unwrap_or_else(|e| panic!("widget : {}", e.one_line()));
    s.keep_agent_session(widget("indicateur"), "sess-perimee".into(), "agent".into());
    s.remove_widget(None, "indicateur").unwrap_or_else(|e| panic!("suppression : {}", e.one_line()));
    s.keep_agent_session(widget("kpi"), "sess-k".into(), "agent".into());
    drop(s.take_scope_moves());

    s.rename_widget(None, "kpi", "indicateur").unwrap_or_else(|e| panic!("renommage : {}", e.one_line()));
    assert_that!(session_of(&s, &widget("indicateur"))).is_equal_to(Some("sess-k".to_owned()));
    assert_that!(s.conversation(&widget("kpi"))).is_none();
    assert_that!(s.take_scope_moves())
        .is_equal_to(vec![ScopeMove::Renamed { from: widget("kpi"), to: widget("indicateur") }]);
}

/// Sans conversation à déplacer, celle, périmée, du widget supprimé ne passe pas au widget renommé.
#[test]
fn rename_without_conversation_onto_a_stale_one_drops_the_stale_one() {
    let data = data_dir();
    let mut s = editing_with_kpi(&data);
    let ancien = AUTRE.replace("\"autre\"", "\"indicateur\"");
    s.set_widget(None, "indicateur", ancien).unwrap_or_else(|e| panic!("widget : {}", e.one_line()));
    s.keep_agent_session(widget("indicateur"), "sess-perimee".into(), "agent".into());
    s.remove_widget(None, "indicateur").unwrap_or_else(|e| panic!("suppression : {}", e.one_line()));
    drop(s.take_scope_moves());

    s.rename_widget(None, "kpi", "indicateur").unwrap_or_else(|e| panic!("renommage : {}", e.one_line()));
    assert_that!(s.conversation(&widget("indicateur"))).is_none();
    assert_that!(s.take_scope_moves())
        .is_equal_to(vec![ScopeMove::Renamed { from: widget("kpi"), to: widget("indicateur") }]);
    drop(s);

    let mut b = session_in(&data);
    assert!(b.resume(data.join("resume")));
    assert_that!(b.conversation(&widget("indicateur"))).is_none();
}

#[test]
fn conversation_brought_back_by_undo_survives_the_resume() {
    let data = data_dir();
    let mut a = renamed_with_a_conversation(&data);
    a.undo().unwrap_or_else(|e| panic!("annuler : {}", e.one_line()));
    drop(a);

    let mut b = session_in(&data);
    assert!(b.resume(data.join("resume")));
    assert_that!(session_of(&b, &widget("kpi"))).is_equal_to(Some("sess-b".to_owned()));
}
