use oku_slides_core::{Op, Target as OpTarget, WidgetFile};
use serde_json::Value;
use speculoos::prelude::*;

use super::super::{ImportTarget, SlideState, TuneErrorKind};
use crate::test_support::{WhiteWitness, fresh_dir, imported_fixture, session, witnessed};
use crate::{Request, TokenOp};

/// Le hi-fi de `kitchen` : des régions non suspectes (formes, image, tableau), donc des slides à ajuster.
fn hifi(data: &std::path::Path) -> crate::Session {
    imported_fixture(data, "kitchen", crate::test_support::hifi(), |_| {})
}

/// Le rapport de `s` en anglais, sur une référence blanche faite d'abord par `witness`.
fn report(s: &mut crate::Session, witness: &WhiteWitness) -> Value {
    witnessed(s, witness);
    s.import_report(&crate::Localizer::new(Some("en"))).unwrap()
}

/// Retire le premier élément des slots de `slide` : sa page change.
fn change(s: &mut crate::Session, slide: &str) {
    let slide_ref = s.deck().slide(slide).unwrap();
    let element = slide_ref.params.slots().flat_map(|(_, list)| list.iter().map(|e| e.id.clone())).next().unwrap();
    let ops = vec![Op::RemoveElement { slide: slide.to_owned(), element }];
    s.handle(Request::Apply { label: "Retirer".into(), ops, merge: None }).unwrap();
}

/// Change la marge haute de la page du thème du deck (le thème généré du hi-fi) : toutes les pages bougent.
fn edit_theme(s: &mut crate::Session, margin: f64) {
    let id = s.deck().theme.as_ref().unwrap().id;
    let op = TokenOp::SetToken { path: vec!["page".into(), "margin-top".into()], value: serde_json::json!(margin) };
    assert_that!(s.edit_theme(id, None, vec![op]).unwrap().changed).is_true();
}

/// Les essais restants de chaque slide de la file.
fn tries(report: &Value) -> Vec<(String, u64)> {
    let slides = report["slides"].as_array().unwrap();
    slides.iter().map(|e| (e["slide"].as_str().unwrap().to_owned(), e["tries_left"].as_u64().unwrap())).collect()
}

/// Les essais restants des slides `pending` de la file ; jamais vide.
fn pending_tries(report: &Value) -> Vec<(String, u64)> {
    let pending: Vec<(String, u64)> = tries(report)
        .into_iter()
        .filter(|(slide, _)| entry(report, slide).is_some_and(|e| e["state"] == "pending"))
        .collect();
    assert_that!(pending).is_not_empty();
    pending
}

/// La slide de la file `queue` d'id `id`.
fn entry<'v>(report: &'v Value, id: &str) -> Option<&'v Value> {
    report["slides"].as_array()?.iter().find(|s| s["slide"] == id)
}

#[test]
fn import_report_without_an_import_is_refused() {
    let mut s = session();
    let e = s.import_report(&crate::Localizer::new(Some("en"))).unwrap_err();
    assert_that!(e.tune_kind()).is_equal_to(Some(TuneErrorKind::NoImport));
}

#[test]
fn import_report_hifi_lists_the_slides_worst_first_with_their_tries() {
    let data = fresh_dir("oku-slides-import-queue-hifi-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    let first = report(&mut s, &witness);
    assert_that!(first["mode"].as_str()).is_equal_to(Some("hifi"));
    assert_that!(first["witness"].as_str()).is_equal_to(Some("témoin 1.0"));
    assert_that!(first["tries_per_slide"].as_u64()).is_equal_to(Some(3));
    let slides = first["slides"].as_array().unwrap();
    let listed = slides.len() as u64 + first["done"].as_u64().unwrap();
    assert_that!(listed).is_equal_to(3);
    let worst: Vec<f64> = slides.iter().filter_map(|s| s["worst"].as_f64()).collect();
    assert_that!(worst.windows(2).all(|w| w[0] >= w[1])).is_true();
    let pending = slides.iter().find(|s| s["state"] == "pending").unwrap();
    assert_that!(pending["tries_left"].as_u64()).is_equal_to(Some(3));
    assert_that!(pending["excess"].as_f64().unwrap()).is_greater_than(0.0);
    assert_that!(first["global_tries_left"].as_u64()).is_equal_to(Some(3));
    // La première note est la base : la moyenne de départ est la moyenne, rien n'a empiré.
    assert_that!(first["mean_before"].as_f64()).is_equal_to(first["mean"].as_f64());
    assert_that!(first["worse"].as_array().map(Vec::len)).is_equal_to(Some(0));
    // Le témoin ne rend qu'une fois ; un rapport sans changement ne compte rien.
    let again = report(&mut s, &witness);
    assert_that!(witness.calls()).is_equal_to(1);
    assert_that!(tries(&again)).is_equal_to(tries(&first));
}

#[test]
fn import_report_rescores_a_changed_slide_and_names_it_when_worse() {
    let data = fresh_dir("oku-slides-import-queue-worse-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    let first = report(&mut s, &witness);
    let slide = first["slides"][0]["slide"].as_str().unwrap().to_owned();
    // Sur une référence blanche, retirer les éléments de la pire région rapproche la slide ;
    // l'annuler l'en éloigne.
    let diff = s.import_diff(&slide, None).unwrap();
    let elements = diff.json["regions"][0]["elements"].as_array().unwrap().clone();
    assert_that!(elements).is_not_empty();
    let ops: Vec<Op> = elements
        .iter()
        .map(|e| Op::RemoveElement { slide: slide.clone(), element: e.as_str().unwrap().to_owned() })
        .collect();
    s.handle(Request::Apply { label: "Retirer".into(), ops, merge: None }).unwrap();
    let better = report(&mut s, &witness);
    assert_that!(better["worse"].as_array().map(Vec::len)).is_equal_to(Some(0));
    assert_that!(better["mean_before"].as_f64()).is_equal_to(first["mean"].as_f64());
    assert_that!(better["mean"].as_f64().unwrap()).is_less_than(first["mean"].as_f64().unwrap());
    s.handle(Request::Undo).unwrap();
    let worse = report(&mut s, &witness);
    assert_that!(worse["worse"].as_array().unwrap().iter().any(|w| w == slide.as_str())).is_true();
    assert_that!(worse["mean_before"].as_f64()).is_equal_to(first["mean"].as_f64());
    // Chaque note du rapport d'une page changée a compté un essai : la retouche, puis l'annulation.
    assert_that!(entry(&worse, &slide).and_then(|e| e["tries_left"].as_u64())).is_equal_to(Some(1));
}

#[test]
fn import_report_leaves_a_changed_slide_without_tries_stale() {
    let data = fresh_dir("oku-slides-import-queue-stale-");
    let mut s = imported_fixture(data.path(), "kitchen", crate::test_support::hifi(), |settings| {
        settings.import.tune_iterations = std::num::NonZeroU32::MIN;
    });
    let witness = WhiteWitness::default();
    let first = report(&mut s, &witness);
    let slide = first["slides"][0]["slide"].as_str().unwrap().to_owned();
    change(&mut s, &slide);
    let tried = report(&mut s, &witness);
    assert_that!(entry(&tried, &slide).and_then(|e| e["tries_left"].as_u64())).is_equal_to(Some(0));
    change(&mut s, &slide);
    let stale = report(&mut s, &witness);
    assert_that!(stale["stale"].as_array().unwrap().iter().any(|w| w == slide.as_str())).is_true();
    let score = |r: &Value| entry(r, &slide).map(|e| e["score"].clone());
    assert_that!(score(&stale)).is_equal_to(score(&tried));
    assert_that!(entry(&stale, &slide).map(|e| e["stale"].clone())).is_equal_to(Some(Value::Bool(true)));
}

#[test]
fn import_report_leaves_a_stale_slide_out_of_the_means_and_worse() {
    let data = fresh_dir("oku-slides-import-queue-stale-mean-");
    let mut s = imported_fixture(data.path(), "kitchen", crate::test_support::hifi(), |settings| {
        settings.import.tune_iterations = std::num::NonZeroU32::MIN;
    });
    let witness = WhiteWitness::default();
    let first = report(&mut s, &witness);
    let slide = first["slides"][0]["slide"].as_str().unwrap().to_owned();
    change(&mut s, &slide);
    report(&mut s, &witness);
    change(&mut s, &slide);
    let stale = report(&mut s, &witness);
    // Les autres slides n'ont pas changé : leur note est leur première ; la moyenne de départ
    // est celle des mêmes slides.
    let state = s.import.as_ref().unwrap();
    let others: Vec<f64> = state
        .report()
        .slides
        .iter()
        .filter(|r| r.slide != slide)
        .filter_map(|r| r.score.as_ref().map(|s| s.score))
        .collect();
    let expected = (others.iter().sum::<f64>() / others.len() as f64 * 100.0).round() / 100.0;
    assert_that!(stale["mean"].as_f64()).is_equal_to(Some(expected));
    assert_that!(stale["mean_before"].as_f64()).is_equal_to(Some(expected));
    assert_that!(stale["worse"].as_array().unwrap().iter().any(|w| w == slide.as_str())).is_false();
}

#[test]
fn import_report_after_a_theme_edit_draws_one_global_try_and_no_slide_try() {
    let data = fresh_dir("oku-slides-import-queue-theme-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    let first = report(&mut s, &witness);
    edit_theme(&mut s, 30.0);
    let edited = report(&mut s, &witness);
    assert_that!(edited["global_tries_left"].as_u64()).is_equal_to(Some(2));
    assert_that!(edited["stale"].as_array().map(Vec::len)).is_equal_to(Some(0));
    assert_that!(edited["mean"].as_f64()).is_not_equal_to(first["mean"].as_f64());
    for (slide, left) in pending_tries(&edited) {
        assert_that!(left).named(&slide).is_equal_to(3);
    }
    // Revenir à l'état du deck de l'import (déjà noté) ne coûte rien.
    s.handle(Request::Undo).unwrap();
    let undone = report(&mut s, &witness);
    assert_that!(undone["global_tries_left"].as_u64()).is_equal_to(Some(2));
    assert_that!(undone["mean"].as_f64()).is_equal_to(first["mean"].as_f64());
}

/// Les essais faits de chaque slide de l'import de `s`.
fn tries_made(s: &crate::Session) -> Vec<(String, u32)> {
    let state = s.import.as_ref().unwrap();
    let slides = state.report().slides.iter();
    slides.map(|r| (r.slide.clone(), state.tuning(&r.slide).map_or(0, |t| t.tries))).collect()
}

#[test]
fn import_report_after_a_theme_restore_with_a_slide_edit_charges_that_slide_a_try() {
    let data = fresh_dir("oku-slides-import-queue-theme-toggle-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    let first = report(&mut s, &witness);
    let slide = first["slides"][0]["slide"].as_str().unwrap().to_owned();
    edit_theme(&mut s, 30.0);
    assert_that!(report(&mut s, &witness)["global_tries_left"].as_u64()).is_equal_to(Some(2));
    // Le thème revient à l'état de l'import (déjà noté) et la slide est retouchée : elle paie un essai.
    s.handle(Request::Undo).unwrap();
    change(&mut s, &slide);
    let restored = report(&mut s, &witness);
    assert_that!(restored["global_tries_left"].as_u64()).is_equal_to(Some(2));
    for (id, made) in tries_made(&s) {
        assert_that!(made).named(&id).is_equal_to(u32::from(id == slide));
    }
}

#[test]
fn import_report_after_edit_theme_restores_a_theme_already_scored_is_free() {
    let data = fresh_dir("oku-slides-import-queue-theme-restore-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    report(&mut s, &witness);
    edit_theme(&mut s, 30.0);
    let at_30 = report(&mut s, &witness);
    assert_that!(at_30["global_tries_left"].as_u64()).is_equal_to(Some(2));
    edit_theme(&mut s, 60.0);
    assert_that!(report(&mut s, &witness)["global_tries_left"].as_u64()).is_equal_to(Some(1));
    // Une nouvelle version (autre parent, autre date) au contenu déjà noté : un état du deck déjà payé.
    edit_theme(&mut s, 30.0);
    let restored = report(&mut s, &witness);
    assert_that!(restored["global_tries_left"].as_u64()).is_equal_to(Some(1));
    assert_that!(restored["mean"].as_f64()).is_equal_to(at_30["mean"].as_f64());
    for (id, made) in tries_made(&s) {
        assert_that!(made).named(&id).is_equal_to(0);
    }
}

/// Écrit `_commun` du pack du deck, son bandeau du bas posé à `dy`, dans une séance de widgets à lui.
fn set_banner(s: &mut crate::Session, original: &str, dy: &str) {
    let moved = original.replace("dy: 1040pt", &format!("dy: {dy}"));
    assert_that!(moved.as_str()).is_not_equal_to(original);
    // Hors d'une même séance, chaque écriture fait une version de parent différent.
    s.end_widget_session();
    s.set_widget(None, "_commun", moved).unwrap();
}

#[test]
fn import_report_after_set_widget_restores_a_widget_already_scored_is_free() {
    let data = fresh_dir("oku-slides-import-queue-widget-restore-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    report(&mut s, &witness);
    let pack = crate::test_support::pin(&s, "deck-layouts").unwrap();
    let original = s.doc.sources.packs[&pack.version].widget(&WidgetFile::common()).unwrap().to_owned();
    // Le décor du master : son bandeau du bas monté plus haut change toutes les pages.
    set_banner(&mut s, &original, "0pt");
    let at_top = report(&mut s, &witness);
    assert_that!(at_top["global_tries_left"].as_u64()).is_equal_to(Some(2));
    set_banner(&mut s, &original, "500pt");
    assert_that!(report(&mut s, &witness)["global_tries_left"].as_u64()).is_equal_to(Some(1));
    set_banner(&mut s, &original, "0pt");
    let restored = report(&mut s, &witness);
    assert_that!(restored["global_tries_left"].as_u64()).is_equal_to(Some(1));
    assert_that!(restored["mean"].as_f64()).is_equal_to(at_top["mean"].as_f64());
    for (id, made) in tries_made(&s) {
        assert_that!(made).named(&id).is_equal_to(0);
    }
}

#[test]
fn import_report_after_fixing_a_slide_with_a_suspect_region_above_counts_it_done() {
    let data = fresh_dir("oku-slides-import-queue-fixed-witness-");
    // Un titre vide garde un reste (le cadre de son champ) sous ce seuil.
    let mut s = imported_fixture(data.path(), "kitchen", crate::test_support::hifi(), |settings| {
        settings.import.score_threshold = 10.0;
    });
    let witness = WhiteWitness::default();
    // kitchen s3 : le SmartArt (suspect) dépasse le seuil, comme la forme s3-5 et le titre (non suspects).
    let first = report(&mut s, &witness);
    assert_that!(entry(&first, "s3").map(|e| e["state"].clone())).is_equal_to(Some(Value::from("pending")));
    let target = OpTarget { slide: "s3".into(), element: None };
    let ops = vec![
        Op::RemoveElement { slide: "s3".into(), element: "s3-5".into() },
        Op::SetParam { target, param: "titre".into(), value: Value::Null },
    ];
    s.handle(Request::Apply { label: "Corriger".into(), ops, merge: None }).unwrap();
    let fixed = report(&mut s, &witness);
    // Changée depuis l'import, elle n'est plus « laissée telle qu'importée » : `done`, pas `witness`.
    assert_that!(s.import.as_ref().unwrap().slide_state("s3")).is_equal_to(SlideState::Done);
    assert_that!(entry(&fixed, "s3")).is_none();
    assert_that!(fixed["done"].as_u64()).is_equal_to(first["done"].as_u64().map(|d| d + 1));
}

#[test]
fn import_report_after_the_global_budget_charges_each_changed_slide() {
    let data = fresh_dir("oku-slides-import-queue-global-spent-");
    let mut s = imported_fixture(data.path(), "kitchen", crate::test_support::hifi(), |settings| {
        settings.import.tune_global = std::num::NonZeroU32::MIN;
    });
    let witness = WhiteWitness::default();
    report(&mut s, &witness);
    edit_theme(&mut s, 30.0);
    assert_that!(report(&mut s, &witness)["global_tries_left"].as_u64()).is_equal_to(Some(0));
    edit_theme(&mut s, 60.0);
    let spent = report(&mut s, &witness);
    for (slide, left) in pending_tries(&spent) {
        assert_that!(left).named(&slide).is_equal_to(2);
    }
}

#[test]
fn import_report_retrofit_gives_the_widget_requests_and_no_queue() {
    let data = fresh_dir("oku-slides-import-queue-retrofit-");
    let target = ImportTarget::Retrofit { layouts: Vec::new(), theme: None };
    let mut s = imported_fixture(data.path(), "minimal", target, |_| {});
    let witness = WhiteWitness::default();
    let report = report(&mut s, &witness);
    assert_that!(report["mode"].as_str()).is_equal_to(Some("retrofit"));
    assert_that!(report["widget_requests"].is_array()).is_true();
    assert_that!(report.get("slides")).is_none();
    assert_that!(report["losses"]["total"].is_u64()).is_true();
    assert_that!(witness.calls()).is_equal_to(0);
}

#[test]
fn import_report_retrofit_samples_each_widget_request_without_its_text() {
    let data = fresh_dir("oku-slides-import-queue-sample-");
    let target = ImportTarget::Retrofit { layouts: Vec::new(), theme: None };
    let mut s = imported_fixture(data.path(), "kitchen", target, |_| {});
    let report = report(&mut s, &WhiteWitness::default());
    // kitchen : le SmartArt de la slide 3 fait la seule demande de widget.
    let request = &report["widget_requests"][0];
    let items = request["sample"]["items"].as_array().cloned().unwrap_or_default();
    assert_that!(items).is_not_empty();
    for item in &items {
        assert_that!(item["kind"].is_string()).is_true();
        assert_that!(item["frame"].as_array().map(Vec::len)).is_equal_to(Some(4));
    }
    assert_that!(request["sample"].to_string().contains("\"text\":\"")).is_false();
}

#[test]
fn import_reference_job_is_none_in_retrofit_and_once_done_in_hifi() {
    let data = fresh_dir("oku-slides-import-queue-job-");
    let mut s = hifi(data.path());
    let job = s.import_reference_job().unwrap().unwrap();
    job.run_with(&WhiteWitness::default());
    assert_that!(s.import_reference_job().unwrap()).is_none();
    let target = ImportTarget::Retrofit { layouts: Vec::new(), theme: None };
    let mut r = imported_fixture(data.path(), "minimal", target, |_| {});
    assert_that!(r.import_reference_job().unwrap()).is_none();
}

/// Retire de la référence de `s` la page du témoin de `slide` : sa comparaison échoue.
fn drop_reference_page(s: &mut crate::Session, slide: &str) {
    let state = s.import.as_mut().unwrap();
    let reference = std::sync::Arc::get_mut(&mut state.reference).unwrap().get_mut().unwrap();
    reference.pages.as_mut().unwrap().remove(slide);
}

#[test]
fn import_report_after_deleting_a_slide_leaves_it_out_of_the_queue_and_the_mean() {
    let data = fresh_dir("oku-slides-import-queue-deleted-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    let first = report(&mut s, &witness);
    let slide = first["slides"][0]["slide"].as_str().unwrap().to_owned();
    s.handle(Request::Apply {
        label: "Supprimer".into(),
        ops: vec![Op::RemoveSlide { slide: slide.clone() }],
        merge: None,
    })
    .unwrap();
    let after = report(&mut s, &witness);
    assert_that!(entry(&after, &slide)).is_none();
    let listed = after["slides"].as_array().unwrap().len() as u64 + after["done"].as_u64().unwrap();
    assert_that!(listed).is_equal_to(2);
    assert_that!(after["mean"].as_f64()).is_not_equal_to(first["mean"].as_f64());
}

#[test]
fn import_report_after_undoing_the_import_has_an_empty_queue() {
    let data = fresh_dir("oku-slides-import-queue-undone-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    report(&mut s, &witness);
    s.handle(Request::Undo).unwrap();
    let undone = report(&mut s, &witness);
    assert_that!(undone["slides"].as_array().map(Vec::len)).is_equal_to(Some(0));
    assert_that!(undone["done"].as_u64()).is_equal_to(Some(0));
    assert_that!(undone["mean"].is_null()).is_true();
}

#[test]
fn import_report_with_one_slide_failing_marks_it_unscored_and_scores_the_others() {
    let data = fresh_dir("oku-slides-import-queue-failing-");
    let mut s = hifi(data.path());
    let witness = WhiteWitness::default();
    witnessed(&mut s, &witness);
    let failing = s.deck().slides[0].id.clone();
    drop_reference_page(&mut s, &failing);
    let report = s.import_report(&crate::Localizer::new(Some("en"))).unwrap();
    assert_that!(entry(&report, &failing).map(|e| e["state"].clone())).is_equal_to(Some(Value::from("unscored")));
    let scored = report["slides"].as_array().unwrap().iter().filter(|e| e["score"].is_number()).count() as u64;
    assert_that!(scored + report["done"].as_u64().unwrap()).is_equal_to(2);
    assert_that!(report["mean"].is_number()).is_true();
}

/// Fait passer `s` dans un autre mode que le deck.
fn enter(s: &mut crate::Session, mode: &str) {
    match mode {
        "pack" => s.edit_pack(crate::ESSENTIEL_PACK).unwrap(),
        "history" => s.history_open().unwrap(),
        "help" => s.open_help("fr", None).unwrap(),
        other => panic!("mode inconnu : {other}"),
    }
}

#[rstest::rstest]
#[case::pack_report("pack", "import_report", crate::SessionErrorKind::EditingPack)]
#[case::pack_diff("pack", "import_diff", crate::SessionErrorKind::EditingPack)]
#[case::pack_job("pack", "import_reference_job", crate::SessionErrorKind::EditingPack)]
#[case::history_report("history", "import_report", crate::SessionErrorKind::InHistory)]
#[case::history_diff("history", "import_diff", crate::SessionErrorKind::InHistory)]
#[case::history_job("history", "import_reference_job", crate::SessionErrorKind::InHistory)]
#[case::help_report("help", "import_report", crate::SessionErrorKind::ReadOnly)]
#[case::help_diff("help", "import_diff", crate::SessionErrorKind::ReadOnly)]
#[case::help_job("help", "import_reference_job", crate::SessionErrorKind::ReadOnly)]
fn import_tools_outside_deck_mode_are_refused_by_mode(
    #[case] mode: &str,
    #[case] tool: &str,
    #[case] expected: crate::SessionErrorKind,
) {
    let data = fresh_dir("oku-slides-import-queue-mode-");
    let mut s = imported_fixture(data.path(), "minimal", crate::test_support::hifi(), |_| {});
    let slide = s.deck().slides[0].id.clone();
    enter(&mut s, mode);
    let err = match tool {
        "import_report" => s.import_report(&crate::Localizer::new(Some("en"))).map(|_| ()).unwrap_err(),
        "import_diff" => s.import_diff(&slide, None).map(|_| ()).unwrap_err(),
        _ => s.import_reference_job().map(|_| ()).unwrap_err(),
    };
    assert_that!(err.kind()).is_equal_to(expected);
    // L'aide remplace le document (l'import est oublié) ; les modèles et l'historique le mettent de côté.
    assert_that!(s.import_state().is_some()).is_equal_to(mode != "help");
}
