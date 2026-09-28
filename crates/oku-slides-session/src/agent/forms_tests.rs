use oku_slides_a2ui::ChoiceOption;
use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use serde_json::json;

use super::{
    Anchor, AskChoice, AskConfirm, AskRewrite, FieldKind, ParamField, PermissionChoice, PermissionKind, Place,
    ShowParams, ShowVariants, Suggestion, SuggestNext, Variant, choice, confirm, params, permission, rewrite,
    suggestions, variants,
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

fn suggest_next() -> SuggestNext {
    SuggestNext {
        suggestions: vec![
            Suggestion { label: "Mettre en barres".into(), prompt: Some("Affiche la moyenne en barres.".into()) },
            Suggestion { label: "Ajouter une légende".into(), prompt: None },
        ],
    }
}

#[test]
fn suggestions_have_their_own_place_and_one_button_each() {
    let s = suggestions("show-7", &suggest_next());
    assert_that!(s.place).is_equal_to(Place::Suggestions);
    let labels: Vec<String> = buttons(&s).into_iter().map(|b| b.label).collect();
    assert_that!(labels).is_equal_to(vec!["Mettre en barres".to_owned(), "Ajouter une légende".to_owned()]);
}

#[test]
fn suggestion_button_sends_its_prompt_or_its_label() {
    let json = serde_json::to_value(suggestions("show-7", &suggest_next())).unwrap();
    let components = json["messages"][1]["updateComponents"]["components"].as_array().unwrap().clone();
    let prompts: Vec<serde_json::Value> = components
        .iter()
        .filter(|c| c["component"] == "Button")
        .map(|c| c["action"]["event"]["context"]["prompt"].clone())
        .collect();
    assert_that!(prompts).is_equal_to(vec![json!("Affiche la moyenne en barres."), json!("Ajouter une légende")]);
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
        suggestions("show-7", &suggest_next()),
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
