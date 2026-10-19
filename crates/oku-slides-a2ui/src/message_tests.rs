use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{
    Action, CATALOG_ID, ChoiceOption, ColumnKind, Component, ComponentKind, DiffKind, DiffSegment, Dynamic, ListColumn,
    ServerBody, ServerMessage, SlideRef, StepStatus, SubColumn, SubList, SubListFault, TextVariant, TimelineStep,
    ToolCallDetail, UpdateDataModel, Version,
};

#[test]
fn server_message_json_round_trips() {
    let m = ServerMessage {
        version: Version::V0_9,
        body: ServerBody::UpdateDataModel(UpdateDataModel {
            surface_id: "s-1".into(),
            path: Some("/narration".into()),
            value: Some(serde_json::json!("bonjour")),
        }),
    };
    let back: ServerMessage = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_that!(back).is_equal_to(&m);
}

fn to_json(msg: &ServerMessage) -> Option<Value> {
    serde_json::to_value(msg).ok()
}

fn component_json(kind: ComponentKind) -> Option<Value> {
    serde_json::to_value(Component::new("c", kind)).ok()
}

#[test]
fn create_surface_names_the_oku_slides_catalog() {
    assert_that!(to_json(&ServerMessage::create_surface("panel"))).is_some().is_equal_to(json!({
        "version": "v0.9",
        "createSurface": { "surfaceId": "panel", "catalogId": CATALOG_ID }
    }));
}

#[test]
fn update_components_is_flat_with_component_tag() {
    let msg = ServerMessage::update_components(
        "panel",
        vec![
            Component::new("root", ComponentKind::Column { children: vec!["title".into()] }),
            Component::new(
                "title",
                ComponentKind::Text { text: Dynamic::Literal("Grille".into()), variant: Some(TextVariant::H4) },
            ),
        ],
    );
    assert_that!(to_json(&msg)).is_some().is_equal_to(json!({
        "version": "v0.9",
        "updateComponents": {
            "surfaceId": "panel",
            "components": [
                { "id": "root", "component": "Column", "children": ["title"] },
                { "id": "title", "component": "Text", "text": "Grille", "variant": "h4" }
            ]
        }
    }));
}

#[test]
fn card_has_a_single_child() {
    let msg = ServerMessage::update_components(
        "p",
        vec![Component::new("root", ComponentKind::Card { child: "c".into(), tone: None })],
    );
    assert_that!(to_json(&msg).map(|v| v["updateComponents"]["components"][0].clone()))
        .is_some()
        .is_equal_to(json!({ "id": "root", "component": "Card", "child": "c" }));
}

#[test]
fn text_field_binds_value_and_sends_action_with_path_context() {
    let field = Component::new(
        "name",
        ComponentKind::TextField {
            label: Dynamic::Literal("Nom".into()),
            value: Dynamic::path("/template/name"),
            action: Some(Action::event("rename_template", [("value", Dynamic::path("/template/name"))])),
        },
    );
    let msg = ServerMessage::update_components("p", vec![field]);
    assert_that!(to_json(&msg).map(|v| v["updateComponents"]["components"][0].clone())).is_some().is_equal_to(json!({
        "id": "name", "component": "TextField", "label": "Nom", "value": { "path": "/template/name" },
        "action": { "event": { "name": "rename_template", "context": { "value": { "path": "/template/name" } } } }
    }));
}

#[test]
fn rich_text_field_binds_markdown_and_source() {
    let c = Component::new(
        "b",
        ComponentKind::RichTextField {
            label: Dynamic::Literal("Corps".into()),
            value: Dynamic::path("/v"),
            source: Dynamic::path("/v-source"),
            palette: vec!["#112233".into()],
            action: None,
        },
    );
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "id": "b", "component": "RichTextField", "label": "Corps",
        "value": { "path": "/v" }, "source": { "path": "/v-source" }, "palette": ["#112233"]
    }));
}

#[test]
fn stepper_leaves_out_missing_bounds_and_action() {
    let stepper = Component::new(
        "cols",
        ComponentKind::Stepper {
            label: Dynamic::Literal("Colonnes".into()),
            value: Dynamic::path("/grid/columns"),
            min: Some(1),
            max: None,
            action: None,
        },
    );
    let msg = ServerMessage::update_components("p", vec![stepper]);
    assert_that!(to_json(&msg).map(|v| v["updateComponents"]["components"][0].clone())).is_some().is_equal_to(json!({
        "id": "cols", "component": "Stepper", "label": "Colonnes", "value": { "path": "/grid/columns" }, "min": 1
    }));
}

#[test]
fn set_data_model_writes_the_root_without_path() {
    let msg = ServerMessage::set_data_model("panel", json!({ "grid": { "columns": 12 } }));
    assert_that!(to_json(&msg)).is_some().is_equal_to(json!({
        "version": "v0.9",
        "updateDataModel": { "surfaceId": "panel", "value": { "grid": { "columns": 12 } } }
    }));
}

#[test]
fn button_serializes_label_primary_and_action() {
    let action = Action::event("confirm", [("confirmed", Dynamic::Literal(json!(true)))]);
    assert_that!(component_json(ComponentKind::Button {
        label: Dynamic::Literal("Autoriser".into()),
        primary: Some(true),
        danger: None,
        action,
    }))
    .is_some()
    .is_equal_to(json!({
        "id": "c", "component": "Button", "label": "Autoriser", "primary": true,
        "action": { "event": { "name": "confirm", "context": { "confirmed": true } } }
    }));
}

#[test]
fn danger_button_serializes_its_danger_flag() {
    let c = Component::button("always", "Toujours autoriser", false, Action::named("x")).danger(true);
    let json = serde_json::to_value(&c).unwrap();
    assert_that!(json["danger"].clone()).is_equal_to(json!(true));
    let plain =
        serde_json::to_value(Component::button("once", "Une fois", true, Action::named("x")).danger(false)).unwrap();
    assert_that!(plain.get("danger")).is_none();
}

#[test]
fn choice_chips_serializes_options_with_optional_hint() {
    let options = vec![
        ChoiceOption { id: "a".into(), label: "Deux colonnes".into(), hint: None },
        ChoiceOption { id: "b".into(), label: "Citation".into(), hint: Some("sobre".into()) },
    ];
    assert_that!(component_json(ComponentKind::ChoiceChips { options, value: Dynamic::path("/choice"), action: None }))
        .is_some()
        .is_equal_to(json!({
            "id": "c", "component": "ChoiceChips",
            "options": [{ "id": "a", "label": "Deux colonnes" }, { "id": "b", "label": "Citation", "hint": "sobre" }],
            "value": { "path": "/choice" }
        }));
}

#[test]
fn diff_text_serializes_segment_kinds_in_lowercase() {
    let segments = vec![
        DiffSegment { kind: DiffKind::Del, text: "Nos".into() },
        DiffSegment { kind: DiffKind::Ins, text: "Trois".into() },
        DiffSegment { kind: DiffKind::Keep, text: " objectifs".into() },
    ];
    assert_that!(component_json(ComponentKind::DiffText { segments })).is_some().is_equal_to(json!({
        "id": "c", "component": "DiffText",
        "segments": [
            { "kind": "del", "text": "Nos" }, { "kind": "ins", "text": "Trois" }, { "kind": "keep", "text": " objectifs" }
        ]
    }));
}

#[test]
fn slider_color_select_thumb_and_row_serialize_their_fields() {
    let slider = component_json(ComponentKind::Slider {
        label: Dynamic::Literal("Espacement".into()),
        value: Dynamic::path("/f0"),
        min: 0.0,
        max: 64.0,
        step: Some(4.0),
        action: None,
    });
    assert_that!(slider).is_some().is_equal_to(json!({
        "id": "c", "component": "Slider", "label": "Espacement", "value": { "path": "/f0" },
        "min": 0.0, "max": 64.0, "step": 4.0
    }));
    let color = component_json(ComponentKind::ColorField {
        label: Dynamic::Literal("Accent".into()),
        value: Dynamic::path("/f1"),
        palette: vec!["#c2573a".into()],
        action: None,
    });
    assert_that!(color.and_then(|v| v.get("palette").cloned())).is_some().is_equal_to(json!(["#c2573a"]));
    let select = component_json(ComponentKind::Select {
        label: Dynamic::Literal("Mise en page".into()),
        value: Dynamic::path("/f2"),
        options: vec![ChoiceOption { id: "columns".into(), label: "Colonnes".into(), hint: None }],
        action: None,
    });
    assert_that!(select.and_then(|v| v.get("component").cloned())).is_some().is_equal_to(json!("Select"));
    let thumb = component_json(ComponentKind::SlideThumb {
        svg: "<svg/>".into(),
        label: Dynamic::Literal("1 · deux colonnes".into()),
        action: Some(Action::event("variant", [("index", Dynamic::Literal(json!(0)))])),
    });
    assert_that!(thumb.and_then(|v| v.get("svg").cloned())).is_some().is_equal_to(json!("<svg/>"));
    let row = component_json(ComponentKind::Row { children: vec!["a".into(), "b".into()] });
    assert_that!(row).is_some().is_equal_to(json!({ "id": "c", "component": "Row", "children": ["a", "b"] }));
}

#[test]
fn shared_constructors_build_the_same_components() {
    assert_that!(Component::column("b", &["x", "y"]))
        .is_equal_to(Component::new("b", ComponentKind::Column { children: vec!["x".into(), "y".into()] }));
    assert_that!(Component::card("r", "b"))
        .is_equal_to(Component::new("r", ComponentKind::Card { child: "b".into(), tone: None }));
    assert_that!(Component::text("t", "Grille", Some(TextVariant::H4))).is_equal_to(Component::new(
        "t",
        ComponentKind::Text { text: Dynamic::Literal("Grille".into()), variant: Some(TextVariant::H4) },
    ));
    assert_that!(serde_json::to_value(Action::named("dismiss")).ok())
        .is_some()
        .is_equal_to(json!({ "event": { "name": "dismiss" } }));
}

#[test]
fn delete_surface_names_the_surface() {
    assert_that!(to_json(&ServerMessage::delete_surface("panel"))).is_some().is_equal_to(json!({
        "version": "v0.9",
        "deleteSurface": { "surfaceId": "panel" }
    }));
}

#[test]
fn catalog_id_v3_is_the_current_urn() {
    assert_that!(CATALOG_ID).is_equal_to("urn:oku-slides:a2ui:catalog:v3");
}

#[test]
fn timeline_with_paths_serializes_them() {
    let c = Component::new(
        "timeline",
        ComponentKind::Timeline {
            steps: Dynamic::path("/steps"),
            collapsed: Dynamic::path("/collapsed"),
            summary: Dynamic::path("/summary"),
        },
    );
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "id": "timeline", "component": "Timeline",
        "steps": {"path": "/steps"}, "collapsed": {"path": "/collapsed"}, "summary": {"path": "/summary"}
    }));
}

#[test]
fn receipt_without_slide_and_swatch_omits_them() {
    let c = Component::new(
        "r",
        ComponentKind::Receipt {
            title: Dynamic::Literal("Slide 4 : fond bleu nuit".into()),
            facts: vec!["titre en blanc".into()],
            slide: None,
            swatch: None,
        },
    );
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "id": "r", "component": "Receipt", "title": "Slide 4 : fond bleu nuit", "facts": ["titre en blanc"]
    }));
}

#[test]
fn receipt_with_slide_serializes_its_reference_only() {
    let slide = SlideRef { id: "s-4".into(), hash: "0123456789abcdef0123456789abcdef".into() };
    let c = Component::new(
        "r",
        ComponentKind::Receipt { title: Dynamic::Literal("x".into()), facts: vec![], slide: Some(slide), swatch: None },
    );
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "id": "r", "component": "Receipt", "title": "x", "facts": [],
        "slide": { "id": "s-4", "hash": "0123456789abcdef0123456789abcdef" }
    }));
}

/// Aller-retour Rust → JSON → Rust : ce que l'hôte émet se relit à l'identique.
fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap()
}

#[test]
fn timeline_step_round_trips_with_every_status_and_optional_count() {
    let steps: Vec<TimelineStep> =
        [StepStatus::Pending, StepStatus::InProgress, StepStatus::Completed, StepStatus::Failed]
            .into_iter()
            .zip([None, Some(1), Some(6), None])
            .map(|(status, count)| TimelineStep { count, ..TimelineStep::new("apply", status) })
            .collect();
    assert_that!(round_trip(&steps)).is_equal_to(&steps);
    let timeline = Component::new(
        "timeline",
        ComponentKind::Timeline {
            steps: Dynamic::Literal(steps),
            collapsed: Dynamic::Literal(true),
            summary: Dynamic::Literal("6 outils · 38 s".into()),
        },
    );
    assert_that!(round_trip(&timeline)).is_equal_to(&timeline);
}

#[test]
fn timeline_step_with_note_detail_and_calls_round_trips() {
    let call = ToolCallDetail {
        label: "widgets".into(),
        status: StepStatus::Failed,
        input: Some("{\n  \"slide\": 4\n}".into()),
        output: Some("aucun widget".into()),
    };
    let steps = vec![
        TimelineStep {
            detail: Some("Corrections en une transaction.\n\nPuis la grille.".into()),
            ..TimelineStep::new("Corrections", StepStatus::Completed)
        },
        TimelineStep { count: Some(1), calls: vec![call], ..TimelineStep::new("widgets", StepStatus::Failed) },
    ];
    assert_that!(round_trip(&steps)).is_equal_to(&steps);
}

#[test]
fn timeline_step_without_detail_or_calls_omits_them() {
    let v = serde_json::to_value(TimelineStep::new("apply", StepStatus::Completed)).unwrap();
    assert_that!(v).is_equal_to(json!({"label": "apply", "status": "completed"}));
}

#[test]
fn timeline_step_with_unknown_key_is_refused() {
    let v = json!({"label": "apply", "status": "completed", "svg": "<svg/>"});
    assert_that!(serde_json::from_value::<TimelineStep>(v)).is_err();
}

#[test]
fn receipt_round_trips_with_and_without_its_slide_reference() {
    let slide = SlideRef { id: "s-4".into(), hash: "0123456789abcdef0123456789abcdef".into() };
    for (slide, swatch) in [(Some(slide), Some("#00154e".to_owned())), (None, None)] {
        let c = Component::new(
            "card",
            ComponentKind::Receipt {
                title: Dynamic::Literal("Slide 4 : fond bleu nuit".into()),
                facts: vec!["titre en blanc".into(), "surtitre bleu clair".into()],
                slide,
                swatch,
            },
        );
        assert_that!(round_trip(&c)).is_equal_to(&c);
    }
}

/// t-450 : la slide d'un reçu est une référence ; l'ancienne forme (avec son rendu) ne se relit plus.
#[test]
fn receipt_slide_carrying_its_svg_is_refused() {
    let v = json!({"id": "card", "component": "Receipt", "title": "x", "facts": [],
        "slide": {"id": "s-4", "hash": "0123456789abcdef0123456789abcdef", "svg": "<svg/>"}});
    assert_that!(serde_json::from_value::<Component>(v)).is_err();
}

#[test]
fn component_json_round_trip_is_identity() {
    let v = json!({
        "id": "go", "component": "Button", "label": "Autre couleur",
        "action": {"event": {"name": "suggest", "context": {"prompt": "Essaie un vert sapin"}}}
    });
    let c: Component = serde_json::from_value(v.clone()).unwrap();
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(v);
}

#[test]
fn action_with_unknown_field_fails_to_deserialize() {
    let v = json!({"id": "go", "component": "Button", "label": "x",
        "action": {"event": {"name": "suggest"}, "extra": 1}});
    assert_that!(serde_json::from_value::<Component>(v)).is_err();
}

#[test]
fn choice_option_with_unknown_key_is_refused() {
    assert_that!(serde_json::from_value::<ChoiceOption>(json!({"id": "a", "label": "A", "color": "red"}))).is_err();
}

#[test]
fn children_of_containers_lists_their_ids() {
    assert_that!(ComponentKind::Column { children: vec!["a".into(), "b".into()] }.children())
        .is_equal_to(vec!["a", "b"]);
    assert_that!(ComponentKind::Card { child: "c".into(), tone: None }.children()).is_equal_to(vec!["c"]);
    let d = ComponentKind::Disclosure { summary: Dynamic::Literal("Pourquoi".into()), child: "w".into(), open: None };
    assert_that!(d.children()).is_equal_to(vec!["w"]);
}

#[test]
fn actions_of_a_button_lists_its_action() {
    let action = Action::named("suggest");
    let b = Component::button("b", "Ok", false, action.clone());
    assert_that!(b.kind.actions()).is_equal_to(vec![&action]);
}

#[test]
fn without_action_on_button_and_field_drops_or_strips() {
    let b = Component::button("b", "Ok", false, Action::named("suggest"));
    assert_that!(b.kind.without_action()).is_none();
    let field = ComponentKind::Stepper {
        label: Dynamic::Literal("Colonnes".into()),
        value: Dynamic::path("/f0"),
        min: None,
        max: None,
        action: Some(Action::named("param")),
    };
    assert_that!(field.without_action().unwrap().actions()).is_empty();
}

#[test]
fn agent_allowed_for_host_only_components_is_false() {
    let p = Component::new("p", ComponentKind::Prompt { text: Dynamic::Literal("x".into()) });
    let t = Component::new(
        "t",
        ComponentKind::Timeline {
            steps: Dynamic::path("/steps"),
            collapsed: Dynamic::path("/c"),
            summary: Dynamic::path("/s"),
        },
    );
    let r = Component::new(
        "r",
        ComponentKind::Receipt { title: Dynamic::Literal("x".into()), facts: vec![], slide: None, swatch: None },
    );
    assert_that!(p.agent_allowed()).is_false();
    assert_that!(t.agent_allowed()).is_false();
    let thumb = Component::new(
        "th",
        ComponentKind::SlideThumb { svg: "<svg/>".into(), label: Dynamic::Literal("x".into()), action: None },
    );
    assert_that!(r.agent_allowed()).is_false();
    assert_that!(thumb.agent_allowed()).is_false();
    assert_that!(Component::text("n", "x", Some(TextVariant::Note)).agent_allowed()).is_true();
}

#[test]
fn update_data_model_with_path_serializes_it() {
    let m = ServerMessage::update_data_model("turn-1", "/narration", json!("Je lis la slide"));
    assert_that!(serde_json::to_value(&m).unwrap()).is_equal_to(json!({
        "version": "v0.9",
        "updateDataModel": {"surfaceId": "turn-1", "path": "/narration", "value": "Je lis la slide"}
    }));
}

#[test]
fn list_table_carries_its_columns_and_leaves_out_missing_bounds() {
    let c = Component::new(
        "f0",
        ComponentKind::ListTable {
            label: Dynamic::Literal("Étapes".into()),
            rows: Dynamic::path("/f0"),
            change: Dynamic::path("/f0-change"),
            markdown: Dynamic::path("/f0-markdown"),
            columns: vec![ListColumn {
                key: "date".into(),
                label: "Date".into(),
                kind: ColumnKind::Text,
                options: vec![],
                list: None,
            }],
            min: 0,
            max: None,
            item_label: None,
            palette: vec![],
            action: None,
        },
    );
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "id": "f0", "component": "ListTable", "label": "Étapes", "rows": { "path": "/f0" },
        "change": { "path": "/f0-change" }, "markdown": { "path": "/f0-markdown" }, "min": 0, "palette": [],
        "columns": [{ "key": "date", "label": "Date", "kind": "text", "options": [] }]
    }));
}

#[test]
fn list_column_of_a_nested_list_carries_its_sub_columns_and_bounds() {
    let texte = SubColumn { key: "texte".into(), label: "Tâche".into(), kind: ColumnKind::Text, options: vec![] };
    let c = ListColumn {
        key: "taches".into(),
        label: "Tâches".into(),
        kind: ColumnKind::List,
        options: vec![],
        list: Some(Box::new(SubList::new(vec![texte], 0, Some(8), Some("tâche".into())).unwrap())),
    };
    let v = serde_json::to_value(&c).unwrap();
    assert_that!(v.clone()).is_equal_to(json!({
        "key": "taches", "label": "Tâches", "kind": "list", "options": [],
        "list": { "columns": [{ "key": "texte", "label": "Tâche", "kind": "text", "options": [] }],
            "min": 0, "max": 8, "item_label": "tâche" }
    }));
    assert_that!(serde_json::from_value::<ListColumn>(v).unwrap()).is_equal_to(c);
}

#[test]
fn sub_list_with_a_list_column_is_refused() {
    let deep = json!({ "columns": [{ "key": "x", "label": "x", "kind": "list", "options": [] }], "min": 0 });
    assert_that!(serde_json::from_value::<SubList>(deep).is_err()).is_true();
    let col = SubColumn { key: "x".into(), label: "x".into(), kind: ColumnKind::List, options: vec![] };
    assert_that!(SubList::new(vec![col], 0, None, None)).is_equal_to(Err(SubListFault::NestedList));
}

#[test]
fn sub_column_with_its_own_list_is_refused() {
    let deep = json!({ "columns": [{ "key": "x", "label": "x", "kind": "text", "options": [],
        "list": { "columns": [], "min": 0 } }], "min": 0 });
    assert_that!(serde_json::from_value::<SubList>(deep).is_err()).is_true();
}

#[test]
fn sub_list_with_max_below_min_is_refused() {
    assert_that!(serde_json::from_value::<SubList>(json!({ "columns": [], "min": 3, "max": 2 })).is_err()).is_true();
    assert_that!(SubList::new(vec![], 3, Some(2), None)).is_equal_to(Err(SubListFault::MaxBelowMin));
}

fn notice() -> Component {
    Component::new(
        "n",
        ComponentKind::Notice {
            text: Dynamic::Literal("Une autre version".into()),
            children: vec!["mine".into()],
            close: Action::named("close"),
            close_label: Dynamic::Literal("Fermer".into()),
        },
    )
}

#[test]
fn notice_serializes_text_children_and_close() {
    assert_that!(serde_json::to_value(notice()).ok()).is_some().is_equal_to(json!({
        "id": "n", "component": "Notice", "text": "Une autre version", "children": ["mine"],
        "close": { "event": { "name": "close" } }, "close_label": "Fermer"
    }));
}

#[test]
fn notice_children_are_its_action_buttons() {
    assert_that!(notice().kind.children()).is_equal_to(vec!["mine"]);
}

#[test]
fn notice_actions_are_its_close_action() {
    assert_that!(notice().kind.actions()).is_equal_to(vec![&Action::named("close")]);
}

#[test]
fn notice_agent_allowed_is_false() {
    assert_that!(notice().agent_allowed()).is_false();
}

#[test]
fn notice_without_action_is_none() {
    assert_that!(notice().kind.without_action()).is_none();
}

#[test]
fn notice_json_round_trip_is_identity() {
    let json = serde_json::to_value(notice()).unwrap();
    assert_that!(serde_json::from_value::<Component>(json).ok()).is_equal_to(Some(notice()));
}

#[test]
fn notice_without_close_fails_to_deserialize() {
    let json = json!({"id": "n", "component": "Notice", "text": "x", "children": [], "close_label": "Fermer"});
    assert_that!(serde_json::from_value::<Component>(json).is_err()).is_true();
}

fn theme_choice() -> Component {
    Component::new(
        "c0",
        ComponentKind::ThemeChoice {
            svg: Some("<svg/>".into()),
            name: "Papier".into(),
            slide: "s-1".into(),
            swatches: vec!["#ffffff".into(), "#2f5fb3".into()],
            active: true,
            action: Some(Action::event("theme", [("theme", Dynamic::Literal(json!("id-1")))])),
        },
    )
}

#[test]
fn theme_choice_serializes_with_its_catalog_shape() {
    let v = serde_json::to_value(theme_choice()).unwrap();
    assert_that!(v).is_equal_to(json!({
        "id": "c0", "component": "ThemeChoice", "svg": "<svg/>", "name": "Papier", "slide": "s-1",
        "swatches": ["#ffffff", "#2f5fb3"], "active": true,
        "action": { "event": { "name": "theme", "context": { "theme": "id-1" } } }
    }));
}

#[test]
fn theme_choice_is_reserved_to_the_host() {
    assert_that!(theme_choice().agent_allowed()).is_false();
}

#[test]
fn theme_choice_without_action_keeps_everything_else() {
    let plain = theme_choice().kind.without_action().unwrap();
    assert_that!(plain).is_equal_to(ComponentKind::ThemeChoice {
        svg: Some("<svg/>".into()),
        name: "Papier".into(),
        slide: "s-1".into(),
        swatches: vec!["#ffffff".into(), "#2f5fb3".into()],
        active: true,
        action: None,
    });
}
