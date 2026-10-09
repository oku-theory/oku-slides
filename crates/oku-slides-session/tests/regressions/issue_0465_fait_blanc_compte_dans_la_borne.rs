//! t-465 : `report` écarte les faits blancs, mais la borne (4 faits au plus) était vérifiée
//! avant ce filtre : quatre faits et un blanc étaient refusés, alors que la carte n'en
//! aurait montré que quatre.

use oku_slides_session::agent::Report;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::session;

#[test]
fn issue_0465_blank_fact_does_not_count_against_the_bound() {
    let mut s = session();
    let spec = Report {
        title: "Fait".into(),
        facts: ["a", "b", "  ", "c", "d"].iter().map(|f| (*f).to_owned()).collect(),
        slide: None,
        swatch: None,
        why: None,
        actions: Vec::new(),
    };
    let r = s.report_surface("report-1", &spec).unwrap_or_else(|e| panic!("quatre faits et un blanc : {e}"));
    let json = serde_json::to_value(&r.surface).unwrap_or_else(|e| panic!("{e}"));
    let facts = json["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["updateComponents"]["components"].as_array())
        .flatten()
        .find(|c| c["component"] == "Receipt")
        .map(|c| c["facts"].clone())
        .unwrap_or(Value::Null);
    assert_that!(facts).is_equal_to(json!(["a", "b", "c", "d"]));
}
