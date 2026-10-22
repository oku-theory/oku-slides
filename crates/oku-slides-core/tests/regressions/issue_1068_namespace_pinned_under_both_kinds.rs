//! t-1068 : un deck qui épinglait un même espace de noms sous les deux sortes (fusion de deux
//! pairs, deck écrit à la main) n'était pas signalé : le rendu gardait le pack de layouts sans
//! un mot, et chaque élément de cet espace de noms devenait inconnu.

use std::collections::BTreeMap;

use oku_slides_core::{Deck, Namespace, PackFiles, PackKind, PackPin, SourceBundle, SourceLimits};
use serde_json::json;
use speculoos::prelude::*;

use crate::common;

const PACK_ID: &str = "6a4f1c2e-1d3b-4e5f-8a9b-0c1d2e3f4a5b";
const LIMITS: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26, compact_after_secs: 0 };

/// Un pack vide de sorte `kind` sous l'espace de noms `p`.
fn pack(kind: PackKind) -> PackFiles {
    let manifest = json!({ "id": PACK_ID, "name": "P", "namespace": "p", "requires": [],
        "created": "2026-10-19T00:00:00Z" });
    PackFiles::new(kind, manifest.as_object().cloned().unwrap_or_default(), BTreeMap::new())
}

fn pin(files: &PackFiles) -> PackPin {
    PackPin { id: PACK_ID.parse().unwrap_or_else(|e| panic!("{e}")), version: files.version(), requires: Vec::new() }
}

#[test]
fn issue_1068_namespace_pinned_under_both_kinds_is_reported_on_reading() {
    let (widgets, layouts) = (pack(PackKind::Widgets), pack(PackKind::Layouts));
    let mut deck = Deck::from_json(r#"{ "slides": [] }"#).unwrap_or_else(|e| panic!("deck : {e}"));
    let ns = Namespace::parse("p").unwrap_or_else(|| panic!("espace de noms"));
    deck.widgets.insert(ns.clone(), pin(&widgets));
    deck.layouts.insert(ns, pin(&layouts));
    let mut bundle = SourceBundle::new(deck, common::shared()).unwrap_or_else(|e| panic!("bundle : {e}"));
    bundle.add_pack(widgets);
    bundle.add_pack(layouts);
    let zip = bundle.to_zip().unwrap_or_else(|e| panic!("zip : {e}"));
    let (_, issues) = SourceBundle::from_zip(&zip, LIMITS, common::shared()).unwrap_or_else(|e| panic!("relu : {e}"));
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).contains("issue-pack-kind-conflict");
}
