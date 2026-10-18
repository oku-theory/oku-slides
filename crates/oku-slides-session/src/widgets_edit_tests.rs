use speculoos::prelude::*;

use crate::pack::DEFAULT_PACK;
use crate::{Request, Session, SessionErrorKind};

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

const KPI_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

const FRAMED: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#import "_commun.typ": cadre
#let widget = widget("cadre", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
"#;

/// Une session sur un dossier neuf, en édition du pack par défaut.
fn editing(data: &std::path::Path) -> Session {
    let mut s = crate::test_support::session_on(data);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s
}

/// Une session dont les fichiers de widgets sont bornés.
fn bounded(data: &std::path::Path, max_widget_kib: usize, max_widgets: usize) -> Session {
    let mut settings = crate::test_support::settings_on(data);
    settings.render.max_widget_kib = max_widget_kib;
    settings.render.max_widgets = max_widgets;
    let mut s = crate::test_support::session_with(settings);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s
}

#[test]
fn set_widget_twice_then_save_makes_one_version_holding_the_file() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    s.save_templates(false).unwrap();
    let lib = crate::test_support::library_of(&s);
    // L'intégrée, puis la seule version de la séance.
    assert_that!(lib.versions(DEFAULT_PACK).len()).is_equal_to(2);
    let current = lib.current_version(DEFAULT_PACK).unwrap();
    let kpi = oku_slides_core::WidgetFile::parse("kpi").unwrap();
    assert_that!(current.files.widget(&kpi)).is_equal_to(Some(KPI_V2));
    assert_that!(current.manifest.parent.is_some()).is_true();
}

/// Mode modèles sur un pack du deck seul : l'écriture d'un widget reste dans le deck.
#[test]
fn set_widget_in_templates_mode_on_a_deck_only_pack_writes_no_library() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = crate::test_support::session_on(data.path());
    let r = crate::test_support::adopt_deck_only_pack(&mut s);
    let before = crate::test_support::library_files(data.path());
    s.edit_templates(r.id).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    assert_that!(crate::test_support::library_files(data.path())).is_equal_to(before);
    assert_that!(crate::test_support::library_of(&s).versions(r.id)).is_empty();
    let edit = s.snapshot().editing.map(|e| e.mode.clone()).unwrap();
    let version = edit.session_version.clone().unwrap();
    let crate::Mode::Templates { stash, .. } = &s.mode else { panic!("mode modèles attendu") };
    let files = stash.sources.packs.get(&version).cloned().unwrap();
    let kpi = oku_slides_core::WidgetFile::parse("kpi").unwrap();
    assert_that!(files.widget(&kpi)).is_equal_to(Some(KPI_V2));
    assert_that!(files.scope().unwrap()).is_equal_to(oku_slides_core::PackScope::Deck);
    assert_that!(edit.deck_only).is_true();
}

#[test]
fn set_widget_returns_the_status_with_its_schema() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let change = s.set_widget("kpi", KPI.into()).unwrap();
    assert_that!(change.status.is_valid()).is_true();
    assert_that!(change.status.schema.as_ref().map(|w| w.params.len())).is_equal_to(Some(1));
    assert_that!(change.broken).is_empty();
    let reply: serde_json::Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap()).unwrap();
    assert_that!(reply["widgets"].get("kpi").is_some()).is_true();
}

#[test]
fn set_widget_on_common_reports_the_widgets_it_broke() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("_commun", COMMON.into()).unwrap();
    assert_that!(s.set_widget("cadre", FRAMED.into()).unwrap().status.is_valid()).is_true();
    let change = s.set_widget("_commun", "#let cadre = (".into()).unwrap();
    let broken: Vec<&str> = change.broken.iter().map(|b| b.name.as_str()).collect();
    assert_that!(broken).is_equal_to(vec!["cadre"]);
    assert_that!(change.status.diagnostics.iter().any(|d| d.error && d.line == Some(1))).is_true();
}

#[test]
fn widget_status_lists_common_first() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("_commun", COMMON.into()).unwrap();
    let names: Vec<String> = s.widget_status().unwrap().into_iter().map(|w| w.name.to_string()).collect();
    assert_that!(names).is_equal_to(vec!["_commun".to_owned(), "kpi".to_owned()]);
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI.to_owned());
}

#[test]
fn remove_widget_used_by_a_template_is_refused_with_the_template_names() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    let slide = serde_json::json!({
        "op": "add_slide", "index": 0,
        "slide": { "id": "t-kpi", "kind": "blank", "name": "Indicateur", "params": { "elements": [
            { "id": "k", "kind": "kpi", "cell": { "col": 0, "row": 0, "cols": 4, "rows": 2 }, "params": { "value": "1" } }
        ] } }
    });
    s.apply("Modèle", vec![serde_json::from_value(slide).unwrap()], None).unwrap();
    let err = s.remove_widget("kpi").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WidgetInUse);
    assert_that!(err.one_line()).contains("Indicateur");
    let shown = oku_slides_i18n::Localize::localize(&err, &s.localizer);
    assert_that!(shown).contains("utilisé par ces modèles");
    assert_that!(shown).contains("Indicateur");
}

#[test]
fn set_widget_with_an_empty_source_removes_the_file() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("kpi", String::new()).unwrap();
    assert_that!(s.widget_status().unwrap()).is_empty();
}

#[test]
fn rename_widget_renames_the_file_and_its_declared_name() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    let statuses = s.rename_widget("kpi", "indicateur").unwrap();
    assert_that!(statuses.iter().map(|w| w.name.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["indicateur"]);
    assert_that!(statuses[0].is_valid()).is_true();
    assert_that!(s.widget_source("indicateur").unwrap().as_str()).contains("widget(\"indicateur\"");
}

#[test]
fn widget_operations_refuse_bad_names_unknown_files_and_limits() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = bounded(data.path(), 1, 1);
    assert_that!(s.set_widget("Kpi", KPI.into()).unwrap_err().kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    assert_that!(s.widget_source("absent").unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownWidget);
    assert_that!(s.set_widget("kpi", "x".repeat(2048)).unwrap_err().kind())
        .is_equal_to(SessionErrorKind::InvalidWidget);
    s.set_widget("kpi", KPI.into()).unwrap();
    assert_that!(s.set_widget("autre", KPI.into()).unwrap_err().kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    assert_that!(s.rename_widget("kpi", "_commun").unwrap_err().kind()).is_equal_to(SessionErrorKind::InvalidWidget);
}

#[test]
fn set_widget_that_breaks_a_widget_posts_no_broken_widgets_notice() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", "#let widget = (".into()).unwrap();
    assert_that!(s.doc.notice).is_none();
    // Hors écriture, l'avis revient : une session neuve sur la version cassée le pose.
    s.save_templates(false).unwrap();
    s.close_templates().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(s.doc.notice.as_deref().unwrap_or_default()).contains("kpi");
}

#[test]
fn set_widget_in_templates_mode_changes_the_pack_version_of_the_snapshot_each_time() {
    // L'interface relit le panneau Widgets quand `templates.version | editing.base` change.
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let signal = |s: &Session| {
        let snap = s.snapshot();
        (snap.templates.map(|r| r.version.clone()), snap.editing.map(|e| e.base.clone()))
    };
    let before = signal(&s);
    s.set_widget("kpi", KPI.into()).unwrap();
    let first = signal(&s);
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    let second = signal(&s);
    assert_that!(first).is_not_equal_to(before.clone());
    assert_that!(second).is_not_equal_to(first.clone());
    assert_that!(second.0).is_equal_to(second.1.clone());
}

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.plain()), (args, ctx) => field(ctx, "body", args.body))
"#;

const QUOTE_FAILING: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.plain()), (args, ctx) => panic("corps fautif"))
"#;

/// En édition du pack par défaut, un modèle de contenu porte un élément `quote`.
fn template_using_quote(data: &std::path::Path) -> Session {
    let mut s = editing(data);
    s.set_widget("quote", QUOTE.into()).unwrap();
    let slide = s.deck().slides.iter().find(|sl| sl.kind == "content").map(|sl| sl.id.clone()).unwrap();
    let add: oku_slides_core::Op = serde_json::from_value(serde_json::json!({
        "op": "add_element",
        "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "q", "kind": "quote", "params": { "body": "Tout coule." } }
    }))
    .unwrap();
    s.apply("Citation", vec![add], None).unwrap();
    s
}

#[test]
fn set_widget_whose_body_fails_in_a_template_keeps_the_version_and_returns_diagnostics() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = template_using_quote(data.path());
    let change = s.set_widget("quote", QUOTE_FAILING.into()).unwrap();
    // Choix du mode modèles : la version reste écrite, le rendu rend ses diagnostics.
    assert_that!(change.saved).is_true();
    assert_that!(change.status.diagnostics.iter().any(|d| d.error && d.message.contains("corps fautif"))).is_true();
    let current = crate::test_support::library_of(&s).current_version(DEFAULT_PACK).unwrap();
    let quote = oku_slides_core::WidgetFile::parse("quote").unwrap();
    assert_that!(current.files.widget(&quote)).is_equal_to(Some(QUOTE_FAILING));
    assert_that!(s.widget_source("quote").unwrap()).is_equal_to(QUOTE_FAILING.to_owned());
}

#[test]
fn set_widget_with_the_same_source_commits_nothing() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    let before = crate::test_support::library_of(&s).current(DEFAULT_PACK);
    let crate::Mode::Templates { stash, .. } = &mut s.mode else { panic!("mode modèles attendu") };
    stash.dirty = false;
    let change = s.set_widget("kpi", KPI.into()).unwrap();
    assert_that!(change.status.is_valid()).is_true();
    let crate::Mode::Templates { stash, .. } = &s.mode else { panic!("mode modèles attendu") };
    assert_that!(stash.dirty).is_false();
    assert_that!(crate::test_support::library_of(&s).current(DEFAULT_PACK)).is_equal_to(before);
}

#[test]
fn successive_writes_in_templates_mode_keep_only_the_session_version_in_the_stash_sources() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let count = |s: &Session| {
        let crate::Mode::Templates { stash, .. } = &s.mode else { panic!("mode modèles attendu") };
        // Le document des modèles garde ses versions (une étape peut y ramener) ;
        // le deck mis de côté, lui, n'a que celle de la séance.
        stash.sources.packs.len()
    };
    s.set_widget("kpi", KPI.into()).unwrap();
    let first = count(&s);
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
    assert_that!(count(&s)).is_equal_to(first);
}

#[test]
fn set_widget_with_a_nul_character_is_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let err = s.set_widget("kpi", format!("{KPI}\0")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("fr"))))
        .contains("caractère nul");
    assert_that!(s.widget_source("kpi").map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::UnknownWidget));
}

#[test]
fn save_templates_whose_render_fails_keeps_the_version_and_posts_the_failure() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = template_using_quote(data.path());
    s.set_widget("quote", QUOTE_FAILING.into()).unwrap();
    let rename: oku_slides_core::Op =
        serde_json::from_value(serde_json::json!({ "op": "set_deck_param", "key": "title", "value": "Cassé" }))
            .unwrap();
    // Le rendu du document des modèles échoue : l'op n'est pas appliquée,
    // mais l'enregistrement, lui, réussit.
    let _ = s.apply("Nom", vec![rename], None);
    s.doc.notice = None;
    s.save_templates(true).unwrap();
    let crate::Mode::Templates { edit, .. } = &s.mode else { panic!("mode modèles attendu") };
    let lib = crate::test_support::library_of(&s);
    assert_that!(edit.set).is_not_equal_to(DEFAULT_PACK);
    assert_that!(lib.current(edit.set)).is_equal_to(Some(edit.base.clone()));
    assert_that!(s.doc.dirty).is_false();
    assert_that!(s.doc.notice.as_deref().unwrap_or_default()).contains("corps fautif");
}

#[test]
fn rename_widget_to_an_existing_name_is_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let err = s.rename_widget("kpi", "quote").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    let shown = oku_slides_i18n::Localize::localize(&err, &s.localizer);
    assert_that!(shown).contains("« quote » existe déjà");
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI.to_owned());
    assert_that!(s.widget_source("quote").unwrap()).is_equal_to(QUOTE.to_owned());
}

#[test]
fn remove_widget_used_by_a_saved_template_is_refused_in_deck_mode() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = crate::test_support::opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
    let slide = serde_json::json!({
        "op": "add_slide", "index": 0,
        "slide": { "id": "t-kpi", "kind": "blank", "name": "Indicateur", "params": { "elements": [
            { "id": "k", "kind": "kpi", "cell": { "col": 0, "row": 0, "cols": 4, "rows": 2 }, "params": { "value": "1" } }
        ] } }
    });
    s.apply("Modèle", vec![serde_json::from_value(slide).unwrap()], None).unwrap();
    s.save_templates(false).unwrap();
    s.close_templates().unwrap();
    // Mode deck : aucun modèle en cours d'édition, seul le modèle enregistré l'utilise.
    let err = s.remove_widget("kpi").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WidgetInUse);
    assert_that!(err.one_line()).contains("Indicateur");
}

#[test]
fn a_widget_broken_before_a_common_change_is_not_listed_as_broken_by_it() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("_commun", COMMON.into()).unwrap();
    s.set_widget("cadre", FRAMED.into()).unwrap();
    let old = FRAMED.replace("widget(\"cadre\"", "widget(\"vieux\"").replace("cadre(field", "cadre(field(");
    assert_that!(s.set_widget("vieux", old).unwrap().status.is_valid()).is_false();
    let change = s.set_widget("_commun", "#let cadre = (".into()).unwrap();
    let broken: Vec<&str> = change.broken.iter().map(|b| b.name.as_str()).collect();
    assert_that!(broken).is_equal_to(vec!["cadre"]);
}

fn user_write(s: &mut Session, source: String) {
    s.handle(Request::SetWidget { name: "quote".into(), source }).unwrap();
}

fn notes(s: &mut Session) -> String {
    s.agent_context(&crate::agent::AgentUi::default())
}

#[test]
fn noted_user_write_after_agent_write_tells_the_agent() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget_by_agent("quote", QUOTE.into()).unwrap();
    user_write(&mut s, format!("{QUOTE}// v2\n"));
    assert_that!(notes(&mut s).as_str()).contains("user edited widget quote after you");
}

#[test]
fn noted_user_write_twice_tells_the_agent_once() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget_by_agent("quote", QUOTE.into()).unwrap();
    user_write(&mut s, format!("{QUOTE}// v2\n"));
    notes(&mut s);
    user_write(&mut s, format!("{QUOTE}// v3\n"));
    assert_that!(notes(&mut s).contains("after you")).is_false();
}

#[test]
fn noted_user_write_without_agent_write_says_nothing() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    user_write(&mut s, QUOTE.into());
    assert_that!(notes(&mut s).contains("after you")).is_false();
}

#[test]
fn set_widget_by_agent_deleting_leaves_no_mark() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget_by_agent("quote", QUOTE.into()).unwrap();
    s.set_widget_by_agent("quote", String::new()).unwrap();
    user_write(&mut s, QUOTE.into());
    assert_that!(notes(&mut s).contains("after you")).is_false();
}

#[test]
fn set_widget_by_agent_in_templates_turn_can_be_undone() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.begin_turn(1, "widget kpi");
    s.set_widget_by_agent("kpi", KPI.into()).unwrap();
    s.end_turn();
    assert_that!(s.snapshot().can_undo).is_true();
}

#[test]
fn two_widget_writes_in_templates_turn_are_one_step_labelled_by_the_turn() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.begin_turn(1, "deux écritures");
    s.set_widget_by_agent("kpi", KPI.into()).unwrap();
    s.set_widget_by_agent("kpi", KPI_V2.into()).unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : deux écritures"));
}

#[test]
fn two_widget_writes_in_templates_mode_outside_a_turn_merge_into_one_step() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    let steps = s.doc.sources.undo_steps();
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    assert_that!(s.doc.sources.undo_steps()).is_equal_to(steps);
}

/// Une écriture hors tour (V1), puis un tour de l'agent (V2) ; rend la base V1.
fn written_then_agent_turn(s: &mut Session) -> oku_slides_core::PackVersion {
    s.set_widget("kpi", KPI.into()).unwrap();
    let v1 = s.snapshot().editing.unwrap().base.clone();
    s.begin_turn(1, "v2");
    s.set_widget_by_agent("kpi", KPI_V2.into()).unwrap();
    s.end_turn();
    v1
}

#[test]
fn undo_of_a_templates_turn_restores_the_widget_source() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    written_then_agent_turn(&mut s);
    s.undo().unwrap();
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI.to_owned());
}

#[test]
fn undo_of_a_templates_turn_brings_the_replaced_version_back_to_the_library() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let v1 = written_then_agent_turn(&mut s);
    s.undo().unwrap();
    assert_that!(crate::test_support::library_of(&s).version(DEFAULT_PACK, &v1).is_some()).is_true();
}

#[test]
fn redo_of_a_templates_turn_restores_the_turn_source() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    written_then_agent_turn(&mut s);
    s.undo().unwrap();
    s.redo().unwrap();
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI_V2.to_owned());
}

#[test]
fn undo_to_the_parent_then_close_leaves_the_deck_on_the_parent_version() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let parent = s.snapshot().editing.unwrap().parent.clone();
    s.begin_turn(1, "kpi");
    s.set_widget_by_agent("kpi", KPI.into()).unwrap();
    s.end_turn();
    s.undo().unwrap();
    s.close_templates().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(parent);
}

#[test]
fn undo_to_the_parent_clears_the_session_version() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.begin_turn(1, "kpi");
    s.set_widget_by_agent("kpi", KPI.into()).unwrap();
    s.end_turn();
    s.undo().unwrap();
    assert_that!(s.snapshot().editing.unwrap().session_version).is_none();
}

#[test]
fn undo_of_a_widget_write_made_before_a_silent_save_restores_its_source() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.begin_turn(1, "v2");
    s.set_widget_by_agent("kpi", KPI_V2.into()).unwrap();
    s.end_turn();
    s.save_templates(false).unwrap();
    s.undo().unwrap();
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI.to_owned());
}

#[test]
fn save_as_copy_starts_the_templates_document_without_undo_history() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.save_templates(true).unwrap();
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn undo_of_a_write_on_the_copy_keeps_editing_the_copy() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.save_templates(true).unwrap();
    let copy = s.snapshot().editing.unwrap().set;
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    s.undo().unwrap();
    assert_that!((s.snapshot().editing.unwrap().set, s.widget_source("kpi").unwrap()))
        .is_equal_to((copy, KPI.to_owned()));
}
