use oku_slides_a2ui::{ChoiceOption, ColumnKind, ListColumn};
use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use serde_json::{Value, json};

use super::{
    Anchor, AskChoice, AskConfirm, AskRewrite, FieldKind, FormExtras, ParamField, PermissionChoice, PermissionKind,
    Place, ShowParams, ShowVariants, Surface, Variant, choice, confirm, params, params_with, permission, rewrite,
    variants,
};

#[test]
fn surface_json_round_trips() {
    let l = Localizer::new(Some("fr"));
    let s = confirm("c-1", &AskConfirm { message: "Sûr ?".into(), danger: false }, &l);
    let back: Surface = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_that!(back).is_equal_to(&s);
}

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

fn rich_spec() -> ShowParams {
    serde_json::from_value(json!({ "title": "T", "slide": "s-1", "fields": [{
        "label": "Corps", "kind": "rich", "value": [{ "runs": [{ "t": "Un", "bold": true }] }],
        "op_template": { "op": "set_param", "target": { "slide": "s-1" }, "param": "body", "value": "$value" }
    }] }))
    .unwrap()
}

#[test]
fn params_with_a_rich_field_shows_its_markdown() {
    let s = params("show-1", &rich_spec(), &[], &fr());
    assert_that!(serde_json::to_string(&s.messages).unwrap().as_str()).contains("**Un**");
}

#[test]
fn params_with_a_rich_field_sends_its_source_paragraphs() {
    let s = params("show-1", &rich_spec(), &[], &fr());
    assert_that!(serde_json::to_string(&s.messages).unwrap().as_str()).contains("\"path\":\"/f0-source\"");
}

fn host_spec() -> ShowParams {
    serde_json::from_value(json!({ "title": "T", "slide": "", "fields": [
        { "label": "Titre", "kind": "text", "value": "x", "op_template": null }
    ] }))
    .unwrap()
}

#[test]
fn params_with_keys_puts_the_key_in_the_action() {
    let extras = FormExtras { keys: &["title".into()], notes: &[], closable: false };
    let json = serde_json::to_string(&params_with("host-widget", &host_spec(), &extras, &[], &fr()).messages).unwrap();
    assert_that!(json.as_str()).contains("\"key\"");
}

#[test]
fn params_with_notes_shows_them() {
    let extras = FormExtras { keys: &[], notes: &["n1".into()], closable: false };
    let json = serde_json::to_string(&params_with("host-widget", &host_spec(), &extras, &[], &fr()).messages).unwrap();
    assert_that!(json.as_str()).contains("n1");
}

#[test]
fn params_with_not_closable_has_no_close_button() {
    let extras = FormExtras { keys: &[], notes: &[], closable: false };
    let json = serde_json::to_string(&params_with("host-widget", &host_spec(), &extras, &[], &fr()).messages).unwrap();
    assert_that!(json.contains("dismiss")).is_false();
}

fn list_spec() -> ShowParams {
    serde_json::from_value(json!({ "title": "T", "slide": "", "fields": [{
        "label": "Étapes", "kind": "list", "value": [{ "id": "d0", "date": "2019" }], "op_template": null,
        "columns": [{ "key": "date", "label": "Date", "kind": "text" }], "item_label": "étape", "min": 1.0, "max": 8.0
    }] }))
    .unwrap()
}

#[test]
fn params_with_a_list_field_shows_a_list_table_whose_action_carries_the_change() {
    let extras = FormExtras { keys: &["etapes".into()], notes: &[], closable: false };
    let s = params_with("host-widget", &list_spec(), &extras, &[], &fr());
    let messages = serde_json::to_value(&s.messages).unwrap();
    let components = messages[1]["updateComponents"]["components"].as_array().cloned().unwrap_or_default();
    let table = components.into_iter().find(|c| c["id"] == "f0").unwrap();
    assert_that!(table["component"].clone()).is_equal_to(json!("ListTable"));
    assert_that!(table["rows"].clone()).is_equal_to(json!({ "path": "/f0" }));
    assert_that!(table["change"].clone()).is_equal_to(json!({ "path": "/f0-change" }));
    assert_that!((table["min"].clone(), table["max"].clone(), table["item_label"].clone())).is_equal_to((
        json!(1),
        json!(8),
        json!("étape"),
    ));
    assert_that!(table["columns"][0]["kind"].clone()).is_equal_to(json!("text"));
    assert_that!(table["action"]["event"]["context"].clone()).is_equal_to(json!({
        "field": 0, "change": { "path": "/f0-change" }, "key": "etapes"
    }));
    let data = &messages[2]["updateDataModel"]["value"];
    assert_that!((data["f0"].clone(), data["f0-change"].clone()))
        .is_equal_to((json!([{ "id": "d0", "date": "2019" }]), Value::Null));
}

#[test]
fn params_with_a_list_field_gives_the_markdown_of_its_rich_cells() {
    let spec: ShowParams = serde_json::from_value(json!({ "title": "T", "slide": "", "fields": [{
        "label": "Étapes", "kind": "list", "op_template": null,
        "value": [{ "id": "d0", "date": "2019", "texte": [{ "runs": [{ "t": "Un", "bold": true }] }] }],
        "columns": [{ "key": "date", "label": "Date", "kind": "text" }, { "key": "texte", "label": "Texte", "kind": "rich" }]
    }] }))
    .unwrap();
    let s = params("show-1", &spec, &[], &fr());
    let messages = serde_json::to_value(&s.messages).unwrap();
    assert_that!(messages[2]["updateDataModel"]["value"]["f0-markdown"].clone())
        .is_equal_to(json!({ "d0": { "texte": "**Un**" } }));
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
        columns: Vec::new(),
        item_label: None,
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
            field("Corps", FieldKind::Rich, json!([{ "runs": [{ "t": "Un", "bold": true }] }])),
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
    // Réglages de l'hôte : champs adressés par clé, une note, sans « × ».
    let host = ShowParams {
        title: "Paramètres de citation".into(),
        slide: String::new(),
        fields: vec![
            ParamField { op_template: serde_json::Value::Null, ..field("Source", FieldKind::Text, json!("Insee")) },
            ParamField {
                op_template: serde_json::Value::Null,
                ..field("Corps", FieldKind::Rich, json!([{ "runs": [{ "t": "Un" }] }]))
            },
            ParamField {
                op_template: serde_json::Value::Null,
                min: Some(0.0),
                max: Some(8.0),
                item_label: Some("étape".into()),
                columns: vec![
                    ListColumn { key: "date".into(), label: "Date".into(), kind: ColumnKind::Text, options: vec![] },
                    ListColumn { key: "texte".into(), label: "Texte".into(), kind: ColumnKind::Rich, options: vec![] },
                    ListColumn { key: "ton".into(), label: "Ton".into(), kind: ColumnKind::Select, options: options() },
                ],
                ..field(
                    "Étapes",
                    FieldKind::List,
                    json!([{ "id": "d0", "date": "2019", "texte": [{ "runs": [{ "t": "Création" }] }], "ton": "a" }]),
                )
            },
        ],
    };
    let host_keys = ["source".to_owned(), "body".to_owned(), "etapes".to_owned()];
    let host_notes = ["logo : non réglable dans l'aperçu".to_owned()];
    let host_extras = FormExtras { keys: &host_keys, notes: &host_notes, closable: false };
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
        params_with("host-widget", &host, &host_extras, &[], &l),
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
