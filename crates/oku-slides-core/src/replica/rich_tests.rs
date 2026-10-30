use loro::LoroDoc;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;

fn doc() -> LoroDoc {
    let d = LoroDoc::new();
    d.config_text_style(styles());
    d
}

#[test]
fn every_run_and_paragraph_field_has_a_mark() {
    let run = serde_json::to_value(crate::Run {
        t: "x".into(),
        bold: true,
        italic: true,
        color: Some("#000".into()),
        size: Some(crate::TokenValue::Number(1.0)),
        link: Some("https://a".into()),
        highlight: Some(1),
    })
    .unwrap();
    let keys: Vec<&str> = run.as_object().unwrap().keys().map(String::as_str).filter(|k| *k != "t").collect();
    assert_that!(keys.iter().all(|k| RUN_MARKS.contains(k))).is_true();
    let p = serde_json::to_value(crate::Paragraph {
        runs: vec![],
        list: Some(crate::ListKind::Bullet),
        heading: crate::HeadingLevel::new(2),
    })
    .unwrap();
    let keys: Vec<&str> = p.as_object().unwrap().keys().map(String::as_str).filter(|k| *k != "runs").collect();
    assert_that!(keys.iter().all(|k| PARA_MARKS.contains(k))).is_true();
}

#[test]
fn styles_configure_exactly_the_run_and_paragraph_marks() {
    let s = styles();
    for key in RUN_MARKS.iter().chain(PARA_MARKS) {
        assert_that!(s.get(&(*key).into())).is_some();
    }
    assert_that!(NOT_EXPANDING.iter().all(|k| RUN_MARKS.contains(k) || PARA_MARKS.contains(k))).is_true();
    // Pas de style par défaut : une clé hors des marques n'est pas configurée.
    assert_that!(s.get(&"underline".into())).is_none();
    assert_that!(s.get(&"bold".into()).map(|c| c.expand)).is_equal_to(Some(ExpandType::After));
    assert_that!(s.get(&"link".into()).map(|c| c.expand)).is_equal_to(Some(ExpandType::None));
}

#[test]
fn sync_emoji_with_bold_then_read_returns_same_paragraphs() {
    let d = doc();
    let t = d.get_text("t");
    let plain = json!([{ "runs": [{ "t": "😀 a 👩‍👩‍👧 b" }] }]);
    sync(&t, &plain).unwrap();
    d.commit();
    let v = json!([
        { "runs": [{ "t": "😀 " }, { "t": "a 👩‍👩‍👧", "bold": true }, { "t": " b" }] },
        { "runs": [{ "t": "🎉", "italic": true }] }
    ]);
    sync(&t, &v).unwrap();
    d.commit();
    assert_that!(Value::Array(read(&t))).is_equal_to(&v);
}

#[test]
fn sync_then_read_returns_same_paragraphs() {
    let d = doc();
    let t = d.get_text("t");
    let v = json!([
        { "runs": [{ "t": "Le " }, { "t": "marché", "bold": true }, { "t": " a doublé\nen deux ans." }] },
        { "runs": [{ "t": "lien", "link": "https://oku.test" }], "list": "bullet" }
    ]);
    sync(&t, &v).unwrap();
    d.commit();
    assert_that!(Value::Array(read(&t))).is_equal_to(&v);
}

#[test]
fn read_merges_adjacent_runs_with_same_format() {
    let d = doc();
    let t = d.get_text("t");
    sync(&t, &json!([{ "runs": [{ "t": "a" }, { "t": "b" }] }])).unwrap();
    assert_that!(Value::Array(read(&t))).is_equal_to(json!([{ "runs": [{ "t": "ab" }] }]));
}

#[test]
fn empty_paragraph_list_reads_as_empty() {
    let d = doc();
    let t = d.get_text("t");
    sync(&t, &json!([])).unwrap();
    assert_that!(read(&t)).is_empty();
}

#[test]
fn concurrent_bold_and_typing_keep_both() {
    let base = doc();
    base.set_peer_id(0).unwrap();
    sync(&base.get_text("t"), &json!([{ "runs": [{ "t": "Le marché a doublé." }] }])).unwrap();
    base.commit();
    let (a, b) = (base.fork(), base.fork());
    a.config_text_style(styles());
    b.config_text_style(styles());
    sync(
        &a.get_text("t"),
        &json!([{ "runs": [{ "t": "Le " }, { "t": "marché", "bold": true }, { "t": " a doublé." }] }]),
    )
    .unwrap();
    a.commit();
    sync(&b.get_text("t"), &json!([{ "runs": [{ "t": "Le marché a doublé. [B]" }] }])).unwrap();
    b.commit();
    a.import(&b.export(loro::ExportMode::all_updates()).unwrap()).unwrap();
    assert_that!(Value::Array(read(&a.get_text("t")))).is_equal_to(json!([
        { "runs": [{ "t": "Le " }, { "t": "marché", "bold": true }, { "t": " a doublé. [B]" }] }
    ]));
}

#[test]
fn typing_after_link_does_not_extend_link() {
    let d = doc();
    let t = d.get_text("t");
    sync(&t, &json!([{ "runs": [{ "t": "lien", "link": "https://oku.test" }] }])).unwrap();
    d.commit();
    t.insert(4, " suite").unwrap();
    assert_that!(Value::Array(read(&t))).is_equal_to(json!([
        { "runs": [{ "t": "lien", "link": "https://oku.test" }, { "t": " suite" }] }
    ]));
}

#[test]
fn sync_without_bold_removes_bold_and_list() {
    let d = doc();
    let t = d.get_text("t");
    sync(&t, &json!([{ "runs": [{ "t": "a" }, { "t": "b", "bold": true }], "list": "number" }])).unwrap();
    d.commit();
    let plain = json!([{ "runs": [{ "t": "ab" }] }]);
    sync(&t, &plain).unwrap();
    assert_that!(Value::Array(read(&t))).is_equal_to(&plain);
}

#[test]
fn sync_heading_then_read_returns_same_paragraphs() {
    let d = doc();
    let t = d.get_text("t");
    let v = json!([{ "runs": [{ "t": "Plan" }], "heading": 2 }, { "runs": [{ "t": "corps" }] }]);
    sync(&t, &v).unwrap();
    d.commit();
    assert_that!(Value::Array(read(&t))).is_equal_to(&v);
}

#[test]
fn styles_heading_does_not_expand() {
    assert_that!(styles().get(&"heading".into()).map(|c| c.expand)).is_equal_to(Some(ExpandType::None));
}

#[test]
fn concurrent_heading_and_list_keep_both_marks() {
    let base = doc();
    base.set_peer_id(0).unwrap();
    sync(&base.get_text("t"), &json!([{ "runs": [{ "t": "Plan" }] }])).unwrap();
    base.commit();
    let (a, b) = (base.fork(), base.fork());
    a.config_text_style(styles());
    b.config_text_style(styles());
    sync(&a.get_text("t"), &json!([{ "runs": [{ "t": "Plan" }], "heading": 1 }])).unwrap();
    a.commit();
    sync(&b.get_text("t"), &json!([{ "runs": [{ "t": "Plan" }], "list": "bullet" }])).unwrap();
    b.commit();
    a.import(&b.export(loro::ExportMode::all_updates()).unwrap()).unwrap();
    assert_that!(Value::Array(read(&a.get_text("t"))))
        .is_equal_to(json!([{ "runs": [{ "t": "Plan" }], "list": "bullet", "heading": 1 }]));
}
