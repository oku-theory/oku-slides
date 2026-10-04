use oku_slides_a2ui::ChoiceOption;
use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use serde_json::json;

use super::{
    Anchor, AskChoice, AskConfirm, AskRewrite, FieldKind, ParamField, PermissionChoice, PermissionKind, Place,
    ShowParams, ShowVariants, Variant, choice, confirm, params, permission, rewrite, variants,
};

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn options() -> Vec<ChoiceOption> {
    vec![
        ChoiceOption { id: "a".into(), label: "Deux colonnes".into(), hint: None },
        ChoiceOption { id: "b".into(), label: "Citation".into(), hint: Some("sobre".into()) },
    ]
}

#[test]
fn choice_with_anchor_goes_to_sheet() {
    let spec = AskChoice {
        question: "Quelle mise en page ?".into(),
        options: options(),
        anchor: Some(Anchor { slide: "s-4".into(), element: "e-2".into() }),
    };
    let s = choice("ask-1", &spec, &fr());
    assert_that!(s.place).is_equal_to(Place::Sheet);
    insta::assert_json_snapshot!(s);
}

#[test]
fn choice_without_anchor_goes_to_thread() {
    let spec = AskChoice { question: "Laquelle ?".into(), options: options(), anchor: None };
    assert_that!(choice("ask-2", &spec, &fr()).place).is_equal_to(Place::Thread);
}

#[test]
fn params_surface_is_live_in_the_thread() {
    let s = every_form().into_iter().find(|s| s.id == "show-5").unwrap();
    assert_that!(s.place).is_equal_to(Place::Thread);
    assert_that!(s.live).is_true();
}

#[test]
fn frozen_choice_with_answer_shows_it() {
    let s = super::frozen_choice("ask-2", "Laquelle ?", Some("Bleu nuit"), &fr());
    assert_that!(s.live).is_false();
    insta::assert_json_snapshot!(s);
}

#[test]
fn frozen_choice_without_answer_says_cancelled() {
    let s = super::frozen_choice("ask-2", "Laquelle ?", None, &fr());
    assert_that!(serde_json::to_string(&s).unwrap()).contains("Question annulée");
}

#[test]
fn confirm_is_a_modal_with_two_buttons() {
    let s = confirm("ask-3", &AskConfirm { message: "Remplacer le fichier ?".into(), danger: true }, &fr());
    assert_that!(s.place).is_equal_to(Place::Modal);
    insta::assert_json_snapshot!(s);
}

fn choices(kinds: &[(&str, PermissionKind)]) -> Vec<PermissionChoice> {
    kinds.iter().map(|(name, kind)| PermissionChoice { name: (*name).into(), kind: *kind }).collect()
}

/// Un bouton d'une surface, tel que l'interface le reçoit.
#[derive(Debug, PartialEq)]
struct Btn {
    label: String,
    primary: bool,
    danger: bool,
    /// `index` de l'action d'une permission.
    index: Option<u64>,
}

const fn btn(label: String, primary: bool, danger: bool, index: Option<u64>) -> Btn {
    Btn { label, primary, danger, index }
}

/// Les boutons d'une surface, dans l'ordre d'affichage (enfants de `buttons`, sinon ordre des composants).
fn buttons(s: &super::Surface) -> Vec<Btn> {
    let json = serde_json::to_value(s).expect("surface en JSON");
    let components = json["messages"]
        .as_array()
        .and_then(|m| m.iter().find_map(|m| m.get("updateComponents")))
        .and_then(|u| u["components"].as_array())
        .expect("updateComponents")
        .clone();
    let by_id = |id: &str| components.iter().find(|c| c["id"] == id).cloned();
    let row: Vec<serde_json::Value> = components
        .iter()
        .find(|c| c["id"] == "buttons")
        .and_then(|r| r["children"].as_array())
        .map(|ids| ids.iter().filter_map(|id| id.as_str().and_then(by_id)).collect())
        .unwrap_or_else(|| components.iter().filter(|c| c["component"] == "Button").cloned().collect());
    row.iter()
        .map(|c| Btn {
            label: c["label"].as_str().unwrap_or_default().to_owned(),
            primary: c["primary"] == true,
            danger: c["danger"] == true,
            index: c["action"]["event"]["context"]["index"].as_u64(),
        })
        .collect()
}

fn title_of(s: &super::Surface) -> String {
    let json = serde_json::to_value(s).expect("surface en JSON");
    json["messages"][1]["updateComponents"]["components"]
        .as_array()
        .and_then(|cs| cs.iter().find(|c| c["id"] == "title"))
        .and_then(|c| c["text"].as_str())
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn permission_orders_buttons_from_safest_and_keeps_the_agent_index() {
    let s = permission(
        "perm-1",
        "Écrire ?",
        &choices(&[
            ("Toujours autoriser", PermissionKind::AllowAlways),
            ("Toujours refuser", PermissionKind::RejectAlways),
            ("Autoriser une fois", PermissionKind::AllowOnce),
            ("Refuser", PermissionKind::RejectOnce),
        ]),
        &fr(),
    );
    assert_that!(buttons(&s)).is_equal_to(vec![
        btn("Autoriser une fois".into(), true, false, Some(2)),
        btn("Refuser".into(), false, false, Some(3)),
        btn("Toujours refuser".into(), false, false, Some(1)),
        btn("Toujours autoriser · pour toujours".into(), false, true, Some(0)),
    ]);
}

#[test]
fn permission_without_allow_once_has_no_primary() {
    let s = permission(
        "perm-1",
        "Écrire ?",
        &choices(&[("Toujours autoriser", PermissionKind::AllowAlways), ("Refuser", PermissionKind::RejectOnce)]),
        &fr(),
    );
    assert_that!(buttons(&s)).is_equal_to(vec![
        btn("Refuser".into(), false, false, Some(1)),
        btn("Toujours autoriser · pour toujours".into(), false, true, Some(0)),
    ]);
}

#[test]
fn permission_misleading_label_still_gets_the_always_tag() {
    // Un agent qui présente « toujours » comme « une fois » : la mention de l'app le trahit.
    let s = permission("perm-1", "Écrire ?", &choices(&[("Autoriser une fois", PermissionKind::AllowAlways)]), &fr());
    assert_that!(buttons(&s)).is_equal_to(vec![btn("Autoriser une fois · pour toujours".into(), false, true, Some(0))]);
}

#[test]
fn permission_unknown_kind_is_engaging_and_never_primary() {
    let s = permission("perm-1", "Écrire ?", &choices(&[("Pour la session", PermissionKind::Other)]), &fr());
    assert_that!(buttons(&s)).is_equal_to(vec![btn("Pour la session".into(), false, true, Some(0))]);
}

#[test]
fn permission_primary_is_the_first_of_several_allow_once() {
    let s = permission(
        "perm-1",
        "Écrire ?",
        &choices(&[("Ce fichier", PermissionKind::AllowOnce), ("Ce dossier", PermissionKind::AllowOnce)]),
        &fr(),
    );
    assert_that!(buttons(&s).iter().map(|b| b.primary).collect::<Vec<_>>()).is_equal_to(vec![true, false]);
}

#[test]
fn permission_without_title_says_what_it_asks() {
    let s = permission("perm-1", " ", &choices(&[("Refuser", PermissionKind::RejectOnce)]), &fr());
    assert_that!(title_of(&s)).is_equal_to("L'agent demande une permission".to_owned());
}

#[test]
fn danger_confirm_marks_its_yes_button_danger() {
    let s = confirm("ask-3", &AskConfirm { message: "Remplacer ?".into(), danger: true }, &fr());
    assert_that!(buttons(&s).iter().map(|b| (b.primary, b.danger)).collect::<Vec<_>>())
        .is_equal_to(vec![(true, true), (false, false)]);
}

#[test]
fn calm_confirm_has_no_danger_button() {
    let s = confirm("ask-4", &AskConfirm { message: "Continuer ?".into(), danger: false }, &fr());
    assert_that!(buttons(&s).iter().any(|b| b.danger)).is_false();
}

/// Une surface de chaque forme, telle que l'interface la reçoit.
fn every_form() -> Vec<super::Surface> {
    let l = fr();
    let anchor = Some(Anchor { slide: "s-4".into(), element: "e-2".into() });
    let field = |label: &str, kind, value| ParamField {
        label: label.into(),
        kind,
        value,
        op_template: json!({ "op": "set_style", "slide": "s-4", "value": "$value" }),
        min: None,
        max: None,
        step: None,
        options: vec![],
    };
    let show = ShowParams {
        title: "Réglages".into(),
        slide: "s-4".into(),
        fields: vec![
            ParamField { min: Some(1.0), max: Some(6.0), ..field("Colonnes", FieldKind::Stepper, json!(2)) },
            ParamField {
                min: Some(0.0),
                max: Some(64.0),
                step: Some(4.0),
                ..field("Espacement", FieldKind::Slider, json!(16))
            },
            field("Accent", FieldKind::Color, json!("#c2573a")),
            ParamField { options: options(), ..field("Mise en page", FieldKind::Select, json!("a")) },
            field("Légende", FieldKind::Text, json!("Source : Insee")),
        ],
    };
    let variants_spec = ShowVariants {
        slide: "s-4".into(),
        variants: vec![Variant { label: "Aérée".into(), ops: vec![] }, Variant { label: "Dense".into(), ops: vec![] }],
    };
    let svgs =
        ["<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into(), "<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into()];
    let turn = crate::agent::turn_view::TurnView {
        n: 3,
        prompt: "Change le fond de la slide quatre.".into(),
        plan: Vec::new(),
        steps: vec![
            oku_slides_a2ui::TimelineStep {
                label: "Bleu nuit choisi".into(),
                status: oku_slides_a2ui::StepStatus::Completed,
                count: None,
            },
            oku_slides_a2ui::TimelineStep {
                label: "apply".into(),
                status: oku_slides_a2ui::StepStatus::Completed,
                count: Some(6),
            },
        ],
        tools: 6,
        notes: 1,
        narration: String::new(),
        state: crate::agent::turn_view::TurnState::Done { secs: 38 },
    };
    // Une question du fil en attente (vivante, au dock) et une question répondue (figée, dans son tour).
    let mut ask8 =
        choice("ask-8", &AskChoice { question: "Quelle couleur ?".into(), options: options(), anchor: None }, &l);
    ask8.turn = Some(3);
    let mut ask7 = super::frozen_choice("ask-7", "Quelle mise en page ?", Some("Deux colonnes"), &l);
    ask7.turn = Some(3);
    let report = crate::agent::receipt::Report {
        title: "Slide 4 : fond bleu nuit".into(),
        facts: vec!["titre en blanc".into(), "surtitre bleu clair".into()],
        slide: Some("s-4".into()),
        swatch: Some("#00154e".into()),
        why: Some("Le modèle `content` n'a pas de fond : rectangle pleine page ajouté.".into()),
        actions: vec![crate::agent::receipt::NextStep {
            label: "Autre couleur".into(),
            prompt: Some("Essaie un vert sapin".into()),
        }],
    };
    let mut report8 = crate::agent::receipt::receipt(
        "report-8",
        &report,
        Some(&oku_slides_a2ui::SlideRef { id: "s-4".into(), hash: "0123456789abcdef0123456789abcdef".into() }),
        &l,
    );
    report8.turn = Some(3);
    let render_spec = crate::agent::Render {
        components: serde_json::from_value(json!([
            {"id": "root", "component": "Card", "child": "col", "tone": "accent"},
            {"id": "col", "component": "Column", "children": ["t", "b", "go"]},
            {"id": "t", "component": "Text", "text": "Trois palettes possibles", "variant": "h4"},
            {"id": "b", "component": "Badge", "text": "nouveau", "tone": "accent"},
            {"id": "go", "component": "Button", "label": "Essaie la deuxième", "action": {"event": {"name": "suggest", "context": {"prompt": "Essaie la deuxième"}}}}
        ]))
        .unwrap(),
    };
    let mut render9 = crate::agent::render::render("render-9", &render_spec).unwrap();
    render9.turn = Some(3);
    vec![
        choice("ask-1", &AskChoice { question: "Quelle mise en page ?".into(), options: options(), anchor }, &l),
        confirm("ask-2", &AskConfirm { message: "Remplacer le fichier ?".into(), danger: true }, &l),
        permission(
            "perm-3",
            "Écrire le fichier ?",
            &choices(&[
                ("Toujours autoriser", PermissionKind::AllowAlways),
                ("Autoriser une fois", PermissionKind::AllowOnce),
                ("Refuser", PermissionKind::RejectOnce),
            ]),
            &l,
        ),
        rewrite(
            "ask-4",
            &AskRewrite {
                slide: "s-4".into(),
                element: "@title".into(),
                proposals: vec!["Nos paris pour 2027".into(), "Ce que nous visons".into()],
            },
            "Nos objectifs pour 2027",
            &l,
        ),
        params("show-5", &show, &["#c2573a".into(), "#1f4e79".into()], &l),
        variants("show-6", &variants_spec, &svgs, &l),
        crate::agent::turn_view::turn_surface(&turn, &l),
        ask7,
        ask8,
        report8,
        render9,
    ]
}

/// Contrat avec l'interface : `ui/src/a2ui/fixtures/agent-surfaces.json`, que
/// `catalog.test.tsx` fait valider par le catalogue TypeScript.
/// `OKU_UPDATE_FIXTURES=1` le réécrit.
#[test]
fn every_form_matches_the_ui_fixture() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/a2ui/fixtures/agent-surfaces.json");
    let json = serde_json::to_string_pretty(&every_form()).unwrap() + "\n";
    if std::env::var_os("OKU_UPDATE_FIXTURES").is_some() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        std::fs::write(path, &json).unwrap();
    }
    let fixture = std::fs::read_to_string(path).unwrap_or_default();
    assert_that!(fixture).named("OKU_UPDATE_FIXTURES=1 pour régénérer").is_equal_to(json);
}

#[test]
fn set_value_updates_the_data_model_sent_on_resync() {
    let mut s = every_form().into_iter().find(|s| s.id == "show-5").unwrap();
    s.set_value("f2", json!("#1f4e79"));
    let json = serde_json::to_value(&s).unwrap();
    let model = json["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap();
    assert_that!(model["value"]["f2"].clone()).is_equal_to(json!("#1f4e79"));
    assert_that!(model["value"]["f1"].clone()).is_equal_to(json!(16));
}

/// `question` (et non le préfixe `ask-` de l'identifiant) désigne ce qui attend une réponse.
#[test]
fn question_marks_exactly_the_surfaces_awaiting_an_answer() {
    let questions: Vec<String> = every_form().into_iter().filter(|s| s.question).map(|s| s.id).collect();
    // `ask-7` est la version figée d'une question répondue : ce n'en est plus une.
    assert_that!(questions).is_equal_to(vec![
        "ask-1".to_owned(),
        "ask-2".to_owned(),
        "perm-3".to_owned(),
        "ask-4".to_owned(),
        "ask-8".to_owned(),
    ]);
}
