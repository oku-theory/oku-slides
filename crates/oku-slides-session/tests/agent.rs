use oku_slides_core::{Op, Paragraph, Run};
use oku_slides_session::agent::{
    AgentUi, AskRewrite, FieldKind, ParamField, Place, ShowParams, ShowVariants, Surface, Variant,
};
use oku_slides_session::{Fonts, Platform, Session, SessionErrorKind, Settings};
use serde_json::json;
use speculoos::prelude::*;

/// Une session isolée, en français (comme `tests/session.rs`).
fn session() -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    let data = std::env::temp_dir().join(format!("oku-slides-agent-session-{}-{}", std::process::id(), unique()));
    settings.paths.data = Some(data);
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()))
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

#[test]
fn png_of_slide_returns_png_bytes() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let png = s.png("s-libre").unwrap();
    assert_that!(png[..4].to_vec()).is_equal_to(b"\x89PNG".to_vec());
}

#[test]
fn png_of_unknown_slide_returns_no_slide() {
    let s = session();
    assert_that!(s.png("inconnue").unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn rewrite_surface_of_text_element_diffs_each_proposal() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = AskRewrite {
        slide: "s-libre".into(),
        element: "e-texte".into(),
        proposals: vec!["Un bloc posé tout près du coin".into(), "Un bloc libre".into()],
    };
    let surface = s.rewrite_surface("ask-9", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Sheet);
    let json = serde_json::to_string(&surface).unwrap();
    assert_that!(json.matches("\"DiffText\"").count()).is_equal_to(2);
    assert_that!(json.contains("\"draft\":\"Un bloc posé tout près du coin\"")).is_true();
}

#[test]
fn rewrite_surface_of_title_field_reads_slide_title() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let title = s.deck().slide("s-constat").and_then(|sl| sl.title.clone()).unwrap();
    let spec =
        AskRewrite { slide: "s-constat".into(), element: "@title".into(), proposals: vec!["A".into(), "B".into()] };
    let json = serde_json::to_string(&s.rewrite_surface("ask-10", &spec).unwrap()).unwrap();
    assert_that!(json.contains(&title)).is_true();
}

#[test]
fn rewrite_with_one_proposal_returns_agent_form_error() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = AskRewrite { slide: "s-libre".into(), element: "e-texte".into(), proposals: vec!["Seule".into()] };
    assert_that!(s.rewrite_surface("ask-11", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn rewrite_ops_of_element_with_paragraphs_carries_their_runs() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec =
        AskRewrite { slide: "s-libre".into(), element: "e-texte".into(), proposals: vec!["a".into(), "b".into()] };
    let bold = Paragraph { runs: vec![Run { bold: true, ..Run::plain("Gras") }], list: None };
    let ops = s.rewrite_ops(&spec, "**Gras**", Some(vec![bold])).unwrap();
    s.apply("Reformulation", ops).unwrap();
    let deck = s.deck();
    let located = deck.slide("s-libre").unwrap().locate("e-texte").unwrap();
    let run = &located.element.body.text().unwrap()[0].runs[0];
    assert_that!(run.bold).is_true();
    assert_that!(run.t.as_str()).is_equal_to("Gras");
}

#[test]
fn rewrite_ops_of_element_without_paragraphs_returns_agent_form_error() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec =
        AskRewrite { slide: "s-libre".into(), element: "e-texte".into(), proposals: vec!["a".into(), "b".into()] };
    assert_that!(s.rewrite_ops(&spec, "Nouveau", None).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn rewrite_ops_of_field_ignores_missing_paragraphs() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec =
        AskRewrite { slide: "s-constat".into(), element: "@title".into(), proposals: vec!["A".into(), "B".into()] };
    let ops = s.rewrite_ops(&spec, "Nouveau titre", None).unwrap();
    s.apply("Reformulation", ops).unwrap();
    assert_that!(s.deck().slide("s-constat").unwrap().title.as_deref()).is_equal_to(Some("Nouveau titre"));
}

fn theme_field(template: serde_json::Value) -> ParamField {
    ParamField {
        label: "Thème".into(),
        kind: FieldKind::Select,
        value: json!("papier"),
        op_template: template,
        min: None,
        max: None,
        step: None,
        options: vec![],
    }
}

#[test]
fn params_surface_valid_template_goes_to_postit() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "Apparence".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "set_theme", "theme": "$value" }))],
    };
    let surface = s.params_surface("show-1", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Postit);
    let json = serde_json::to_value(&surface).unwrap();
    let components = json["messages"][1]["updateComponents"]["components"].as_array().unwrap();
    let f0 = components.iter().find(|c| c["id"] == "f0").unwrap();
    assert_that!(f0["component"].as_str()).is_some().is_equal_to("Select");
    assert_that!(f0["action"]["event"]["name"].as_str()).is_some().is_equal_to("param");
    assert_that!(f0["action"]["event"]["context"]["field"].as_i64()).is_some().is_equal_to(0);
    assert_that!(json["messages"][2]["updateDataModel"]["value"]["f0"].as_str()).is_some().is_equal_to("papier");
}

#[test]
fn params_surface_no_fields_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![] };
    assert_that!(s.params_surface("show-1b", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn params_surface_missing_slide_returns_no_slide() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "x".into(),
        slide: "inconnue".into(),
        fields: vec![theme_field(json!({ "op": "set_theme", "theme": "$value" }))],
    };
    assert_that!(s.params_surface("show-1c", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn params_surface_does_not_mutate_the_real_deck() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let before = s.deck().clone();
    let spec = ShowParams {
        title: "x".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "set_theme", "theme": "$value" }))],
    };
    s.params_surface("show-1d", &spec).unwrap();
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn params_surface_slider_with_inverted_bounds_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = ParamField {
        label: "Espacement".into(),
        kind: FieldKind::Slider,
        value: json!(4.0),
        op_template: json!({ "op": "set_theme", "theme": "$value" }),
        min: Some(10.0),
        max: Some(2.0),
        step: None,
        options: vec![],
    };
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] };
    let err = s.params_surface("show-1e", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
}

#[test]
fn params_surface_slider_with_only_one_bound_still_rejects_an_implicit_inverted_range() {
    // `forms::params` retombe sur 0..100 quand une borne manque : fournir
    // seul un `min` au-delà de 100 (ou seul un `max` en-dessous de 0)
    // produirait, sans cette vérification, une plage inversée à l'affichage.
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = ParamField {
        label: "Espacement".into(),
        kind: FieldKind::Slider,
        value: json!(4.0),
        op_template: json!({ "op": "set_theme", "theme": "$value" }),
        min: Some(150.0),
        max: None,
        step: None,
        options: vec![],
    };
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] };
    let err = s.params_surface("show-1g", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
}

#[test]
fn params_surface_stepper_with_non_finite_bound_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = ParamField {
        label: "Colonnes".into(),
        kind: FieldKind::Stepper,
        value: json!(4),
        op_template: json!({ "op": "set_theme", "theme": "$value" }),
        min: Some(f64::NAN),
        max: None,
        step: None,
        options: vec![],
    };
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] };
    let err = s.params_surface("show-1f", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
}

#[test]
fn params_surface_template_that_is_not_an_op_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "x".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "no_such_op", "v": "$value" }))],
    };
    let err = s.params_surface("show-2", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
    // La raison ne doit pas répéter le préfixe « invalid surface » : `param_op`
    // et `params_surface` construisaient chacun leur propre `AgentForm`, en
    // enveloppant le message déjà rendu de l'autre.
    assert_that!(err.one_line().matches("invalid surface").count()).is_equal_to(1);
}

#[test]
fn params_surface_template_on_missing_element_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let t =
        json!({ "op": "set_text", "slide": "s-libre", "element": "absent", "text": [{ "runs": [{ "t": "$value" }] }] });
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![theme_field(t)] };
    assert_that!(s.params_surface("show-3", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn param_op_fills_template_with_chosen_value() {
    let s = session();
    let field = theme_field(json!({ "op": "set_theme", "theme": "$value" }));
    assert_that!(s.param_op(&field, &json!("pro")).unwrap()).is_equal_to(Op::SetTheme { theme: Some("pro".into()) });
}

fn retitle(title: &str) -> Vec<Op> {
    vec![Op::SetText { slide: "s-libre".into(), element: "e-texte".into(), text: vec![Paragraph::plain(title)] }]
}

#[test]
fn variants_surface_renders_each_variant_without_touching_deck() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let before = s.deck().clone();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    let surface = s.variants_surface("show-4", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Drawer);
    let json = serde_json::to_value(&surface).unwrap();
    let components = json["messages"][1]["updateComponents"]["components"].as_array().unwrap();
    assert_that!(components.iter().filter(|c| c["component"] == "SlideThumb").count()).is_equal_to(2);
    let v0 = components.iter().find(|c| c["id"] == "v0").unwrap();
    let v1 = components.iter().find(|c| c["id"] == "v1").unwrap();
    assert_that!(v0["svg"].as_str().unwrap().contains("<svg")).is_true();
    assert_that!(v0["label"].as_str()).is_some().is_equal_to("1 · Un");
    assert_that!(v1["label"].as_str()).is_some().is_equal_to("2 · Deux");
    assert_that!(v0["action"]["event"]["context"]["index"].as_i64()).is_some().is_equal_to(0);
    assert_that!(v1["action"]["event"]["context"]["index"].as_i64()).is_some().is_equal_to(1);
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn variants_surface_with_one_variant_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec =
        ShowVariants { slide: "s-libre".into(), variants: vec![Variant { label: "Seule".into(), ops: retitle("Un") }] };
    assert_that!(s.variants_surface("show-4b", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn variants_surface_missing_slide_returns_no_slide() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "inconnue".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    assert_that!(s.variants_surface("show-4c", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn variants_surface_variant_that_removes_the_target_slide_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Ok".into(), ops: retitle("Ok") },
            Variant { label: "Supprime".into(), ops: vec![Op::RemoveSlide { slide: "s-libre".into() }] },
        ],
    };
    let err = s.variants_surface("show-4d", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn variants_surface_variant_with_too_many_ops_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let too_many = (0..=oku_slides_session::agent::forms::OPS.1).map(|_| retitle("x").remove(0)).collect();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Bonne".into(), ops: retitle("Ok") },
            Variant { label: "Trop".into(), ops: too_many },
        ],
    };
    let err = s.variants_surface("show-4e", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn variants_surface_with_invalid_ops_fails_whole_call() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let bad = vec![Op::SetText { slide: "s-libre".into(), element: "absent".into(), text: vec![] }];
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Bonne".into(), ops: retitle("Ok") },
            Variant { label: "Mauvaise".into(), ops: bad },
        ],
    };
    let err = s.variants_surface("show-5", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn variants_surface_with_too_many_variants_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let variants = (0..=oku_slides_session::agent::forms::VARIANTS.1)
        .map(|i| Variant { label: format!("v{i}"), ops: retitle("x") })
        .collect();
    let spec = ShowVariants { slide: "s-libre".into(), variants };
    assert_that!(s.variants_surface("show-6", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn variants_surface_with_empty_ops_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Vide".into(), ops: vec![] },
        ],
    };
    let err = s.variants_surface("show-7", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn params_surface_with_too_many_fields_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let fields = (0..=oku_slides_session::agent::forms::FIELDS.1)
        .map(|_| theme_field(json!({ "op": "set_theme", "theme": "$value" })))
        .collect();
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields };
    assert_that!(s.params_surface("show-8", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn variants_surface_called_twice_in_a_row_gives_the_same_result() {
    // Ne prouve pas, en boîte noire, que le second appel réutilise
    // effectivement `variant_renderer` plutôt que d'en reconstruire un
    // (les deux chemins produiraient le même rendu) : ce test garde surtout
    // la branche « réutilisation » du `match &mut self.variant_renderer` de
    // `variants_surface` exercée et sans régression observable (pas de
    // panique, pas de changement de résultat), pas la preuve de la
    // réutilisation elle-même.
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    let first = s.variants_surface("show-9a", &spec).unwrap();
    let second = s.variants_surface("show-9b", &spec).unwrap();
    let json = |s: &Surface| serde_json::to_value(&s.messages[1]).unwrap()["updateComponents"]["components"].clone();
    assert_that!(json(&second)).is_equal_to(json(&first));
}

#[test]
fn variants_surface_still_works_after_reopening_the_document() {
    // Même limite que ci-dessus : rouvrir le même fichier ne change pas les
    // polices de rendu ici (`render_fonts` identique), donc la branche de
    // remise à zéro de `variant_renderer` (`lib.rs`, sur changement de
    // polices) n'est pas déclenchée par ce test — il n'y a pas de fixture à
    // polices distinctes dans ce dépôt pour l'exercer directement. Ce test
    // garde seulement la non-régression : un appel après réouverture reste
    // sans erreur.
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    s.variants_surface("show-10a", &spec).unwrap();
    s.open(&fixture_path()).unwrap();
    let surface = s.variants_surface("show-10b", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Drawer);
}

#[test]
fn agent_context_describes_mode_slide_selection_and_drains_notes() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.note("variant picked: \"image à gauche\" (s-libre)");
    let ui = AgentUi { slide: Some("s-libre".into()), element: Some("e-texte".into()), field: None, editing: false };
    let ctx = s.agent_context(&ui);
    insta::assert_snapshot!(ctx);
    assert_that!(s.agent_context(&ui).contains("since_last_turn")).is_false();
}

#[test]
fn turn_groups_every_apply_into_one_undo() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.begin_turn("titre plus percutant");
    s.apply("a", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    s.apply("b", vec![Op::SetTheme { theme: Some("papier".into()) }]).unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : titre plus percutant"));
    s.undo().unwrap();
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn end_turn_after_edit_templates_closes_stashed_group_too() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.begin_turn("modèles");
    s.apply("a", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    s.edit_templates(oku_slides_session::DEFAULT_SET).unwrap();
    s.end_turn();
    s.close_templates().unwrap();
    s.apply("Seul", vec![Op::SetTheme { theme: Some("papier".into()) }]).unwrap();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Seul"));
}

#[test]
fn document_dir_of_unsaved_deck_is_none() {
    assert_that!(session().document_dir()).is_none();
}
