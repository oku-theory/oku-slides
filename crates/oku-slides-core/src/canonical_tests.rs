use proptest::prelude::*;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::kinds::base;
use crate::model::*;
use crate::params::{ListItem, ParamValue, Params};
use crate::{DeckReplica, SourceBundle};

fn run(t: &str) -> Run {
    Run::plain(t)
}

fn bold(t: &str) -> Run {
    Run { t: t.into(), bold: true, ..Default::default() }
}

fn deck_with(elements: Vec<Element>) -> Deck {
    let mut slide = Slide::new("s", "content");
    slide.params.set("blocks", ParamValue::Slot(elements));
    Deck {
        version: DECK_VERSION,
        theme: None,
        params: DeckParams::default(),
        vars: Default::default(),
        tokens: Value::Null,
        widgets: Default::default(),
        slides: vec![slide],
    }
}

fn text(runs: Vec<Run>) -> Deck {
    deck_with(vec![Element::text("e", vec![Paragraph { runs, list: None }])])
}

fn blocks(deck: &Deck) -> &[Element] {
    deck.slides[0].params.slot("blocks").unwrap_or_default()
}

fn runs(deck: &Deck) -> &[Run] {
    let body = blocks(deck).first().and_then(|e| e.params.rich("body"));
    body.and_then(<[Paragraph]>::first).map(|p| p.runs.as_slice()).unwrap_or_default()
}

fn canonical(mut deck: Deck) -> Deck {
    canonicalize(&mut deck);
    deck
}

#[test]
fn canonicalize_empty_run_is_dropped() {
    let d = canonical(text(vec![run(""), bold("a"), run("")]));
    assert_that!(runs(&d).to_vec()).is_equal_to(vec![bold("a")]);
}

#[test]
fn canonicalize_neighbour_runs_with_same_format_are_merged() {
    let d = canonical(text(vec![run("ab"), run("cd"), bold("e"), run(""), bold("f"), run("g")]));
    assert_that!(runs(&d).to_vec()).is_equal_to(vec![run("abcd"), bold("ef"), run("g")]);
}

#[test]
fn canonicalize_paragraph_separator_in_run_becomes_newline() {
    let d = canonical(text(vec![run("a\u{2029}b")]));
    assert_that!(runs(&d).to_vec()).is_equal_to(vec![run("a\nb")]);
}

#[test]
fn canonicalize_blank_prose_becomes_none_and_other_prose_is_trimmed() {
    let card: Element = serde_json::from_value(json!({
        "id": "c", "kind": "base/card", "params": { "title": " ", "kicker": " k " }
    }))
    .unwrap();
    let mut d = deck_with(vec![card]);
    d.params.author = Some("\n ".into());
    d.params.title = Some(" Titre ".into());
    d.slides[0].params.set("title", ParamValue::Text("\t".into()));
    d.slides[0].name = Some(" nom".into());
    let d = canonical(d);
    assert_that!(d.params.author).is_none();
    assert_that!(d.params.title).is_equal_to(Some("Titre".to_owned()));
    assert_that!(d.slides[0].params.get("title")).is_none();
    assert_that!(d.slides[0].name).is_equal_to(Some("nom".to_owned()));
    let card = &blocks(&d)[0];
    assert_that!(card.params.get("title")).is_none();
    assert_that!(card.params.text("kicker")).is_equal_to(Some("k"));
}

#[test]
fn canonicalize_empty_rich_text_and_null_param_are_dropped() {
    let card: Element = serde_json::from_value(json!({
        "id": "c", "kind": "base/card", "params": { "body": [], "highlight": null }
    }))
    .unwrap();
    let d = canonical(deck_with(vec![card]));
    assert_that!(blocks(&d)[0].params.is_empty()).is_true();
}

#[test]
fn canonicalize_empty_slot_is_dropped() {
    let mut d = deck_with(vec![]);
    d.slides[0].params.set("elements", ParamValue::Slot(vec![]));
    assert_that!(canonical(d).slides[0].params.is_empty()).is_true();
}

#[test]
fn canonicalize_unknown_param_is_kept() {
    let mut d = text(vec![run("a")]);
    d.slides[0].params.set("inconnu", ParamValue::Value(json!({ "x": 1 })));
    let d = canonical(d);
    assert_that!(d.slides[0].params.value("inconnu")).is_equal_to(Some(&json!({ "x": 1 })));
}

#[test]
fn canonicalize_chart_option_numbers_beyond_i64_become_float() {
    let chart: Element = serde_json::from_value(json!({
        "id": "g", "kind": "base/chart", "params": { "option": { "series": [], "n": u64::MAX } }
    }))
    .unwrap();
    let d = canonical(deck_with(vec![chart]));
    let option = blocks(&d)[0].params.value("option").cloned();
    assert_that!(option).is_equal_to(Some(json!({ "series": [], "n": u64::MAX as f64 })));
}

#[test]
fn canonicalize_blank_notes_become_empty_other_notes_are_kept() {
    let mut d = text(vec![]);
    d.slides[0].notes = " \n ".into();
    assert_that!(canonical(d.clone()).slides[0].notes).is_empty();
    d.slides[0].notes = " à dire \n".into();
    assert_that!(canonical(d).slides[0].notes).is_equal_to(" à dire \n".to_owned());
}

#[test]
fn canonicalize_integer_beyond_i64_becomes_float() {
    let mut d = text(vec![]);
    d.vars.insert("n".into(), json!(u64::MAX));
    d.vars.insert("l".into(), json!([1, u64::MAX]));
    d.params.extra.insert("x".into(), json!({ "n": u64::MAX }));
    d.tokens = json!({ "n": u64::MAX });
    let d = canonical(d);
    let big = json!(u64::MAX as f64);
    assert_that!(d.vars["n"]).is_equal_to(&big);
    assert_that!(d.vars["l"]).is_equal_to(json!([1, big]));
    assert_that!(d.params.extra["x"]).is_equal_to(json!({ "n": big }));
    assert_that!(d.tokens).is_equal_to(json!({ "n": big }));
}

#[test]
fn canonicalize_empty_tokens_object_becomes_null() {
    let mut d = text(vec![]);
    d.tokens = json!({});
    assert_that!(canonical(d).tokens).is_equal_to(&Value::Null);
}

#[test]
fn canonicalize_slot_children_are_canonicalized() {
    let child = Element::text("t", vec![Paragraph { runs: vec![run("a"), run("b")], list: None }]);
    let mut group = Element::new("g", base::STACK);
    group.params.set("children", ParamValue::Slot(vec![child]));
    let d = canonical(deck_with(vec![group]));
    let children = blocks(&d)[0].params.slot("children").unwrap_or_default();
    let body = children[0].params.rich("body").unwrap_or_default();
    assert_that!(body[0].runs.to_vec()).is_equal_to(vec![run("ab")]);
}

#[test]
fn canonicalize_empty_list_is_dropped() {
    let mut d = deck_with(vec![]);
    d.slides[0].params.set("l", ParamValue::List(vec![]));
    assert_that!(canonical(d).slides[0].params.is_empty()).is_true();
}

#[test]
fn canonicalize_list_item_values_are_canonicalized() {
    let mut values = Params::default();
    values.set("date", ParamValue::Text("  2019 ".into()));
    values.set("vide", ParamValue::Text("  ".into()));
    values.set("texte", ParamValue::Rich(vec![Paragraph { runs: vec![run("a"), run("b")], list: None }]));
    let mut d = deck_with(vec![]);
    d.slides[0].params.set("l", ParamValue::List(vec![ListItem { id: "a".into(), values }]));
    let d = canonical(d);
    let item = &d.slides[0].params.list("l").unwrap_or_default()[0];
    assert_that!(item.values.text("date")).is_equal_to(Some("2019"));
    assert_that!(item.values.get("vide")).is_none();
    assert_that!(item.values.rich("texte").unwrap_or_default()[0].runs.to_vec()).is_equal_to(vec![run("ab")]);
}

/// Cas de l'UI : une ligne vide de l'éditeur donne `[{t:""}]`.
#[test]
fn source_bundle_new_with_empty_run_view_equals_replica_read() {
    let b = SourceBundle::new(text(vec![run("")]), crate::test_registry::shared()).unwrap();
    assert_that!(b.replica().deck().0).is_equal_to(b.deck());
    assert_that!(runs(b.deck()).to_vec()).is_empty();
}

// --- Propriété : un deck canonique se relit à l'identique. ---

fn prose() -> impl Strategy<Value = Option<String>> {
    proptest::option::of("[ ab\n\\x{2029}é😀]{0,5}")
}

fn a_run() -> impl Strategy<Value = Run> {
    (
        "[ ab\n\\x{2029}😀]{0,4}",
        any::<bool>(),
        any::<bool>(),
        proptest::option::of(prop_oneof![Just("#ff0000".to_owned()), Just("accent".to_owned())]),
        proptest::option::of(prop_oneof![Just(TokenValue::Number(12.5)), Just(TokenValue::Name("lg".into()))]),
        proptest::option::of(Just("https://oku.test".to_owned())),
        proptest::option::of(1u8..4),
    )
        .prop_map(|(t, bold, italic, color, size, link, highlight)| Run {
            t,
            bold,
            italic,
            color,
            size,
            link,
            highlight,
        })
}

fn paragraphs() -> impl Strategy<Value = Vec<Paragraph>> {
    let list = proptest::option::of(prop_oneof![Just(ListKind::Bullet), Just(ListKind::Number)]);
    proptest::collection::vec(
        (proptest::collection::vec(a_run(), 0..4), list).prop_map(|(runs, list)| Paragraph { runs, list }),
        0..3,
    )
}

fn text_param() -> impl Strategy<Value = Option<ParamValue>> {
    prose().prop_map(|p| p.map(ParamValue::Text))
}

fn with(kind: &str, params: Vec<(&str, Option<ParamValue>)>) -> Element {
    let mut e = Element::new("", kind);
    e.frame = Some(Frame { x: 1.5, y: 2.0, w: 300.0, h: 40.25 });
    for (name, value) in params {
        if let Some(v) = value {
            e.params.set(name, v);
        }
    }
    e
}

/// Le texte d'une cellule `plain` : vide compris, bords à rogner, bien en deçà de `PLAIN_MAX`.
fn cell_text() -> impl Strategy<Value = Option<ParamValue>> {
    proptest::option::of("[ ab\né😀]{0,6}".prop_map(ParamValue::Text))
}

/// Une liste aux ids uniques (de 0 à `max` items) dont chaque item prend ses valeurs de `columns` ;
/// `None` (paramètre omis) ou `Some` de zéro item (liste vide) en font partie.
fn list_param(max: usize, columns: BoxedStrategy<Params>) -> impl Strategy<Value = Option<ParamValue>> {
    proptest::option::of(proptest::collection::btree_set("[a-z0-9-]{1,6}", 0..=max).prop_flat_map(move |ids| {
        let n = ids.len();
        proptest::collection::vec(columns.clone(), n).prop_map(move |rows| {
            let items = ids.iter().cloned().zip(rows).map(|(id, values)| ListItem { id, values }).collect();
            ParamValue::List(items)
        })
    }))
}

fn row(cells: Vec<(&'static str, Option<ParamValue>)>) -> Params {
    let mut params = Params::default();
    for (name, value) in cells {
        if let Some(v) = value {
            params.set(name, v);
        }
    }
    params
}

fn steps() -> impl Strategy<Value = Option<ParamValue>> {
    let columns = (cell_text(), cell_text(), paragraphs(), any::<bool>(), any::<bool>()).prop_map(
        |(date, texte, note, with_note, fait)| {
            row(vec![
                ("date", date),
                ("texte", texte),
                ("note", with_note.then(|| ParamValue::Rich(note))),
                ("fait", fait.then(|| ParamValue::Value(Value::Bool(true)))),
            ])
        },
    );
    list_param(3, columns.boxed())
}

/// La liste `etapes` d'une slide `chrono` : deux colonnes riches.
fn chrono_steps() -> impl Strategy<Value = Option<ParamValue>> {
    let columns = (paragraphs(), paragraphs()).prop_map(|(note, suite)| {
        row(vec![("note", Some(ParamValue::Rich(note))), ("suite", Some(ParamValue::Rich(suite)))])
    });
    list_param(3, columns.boxed())
}

/// Une cellule `number` : entier, flottant ou entier au-delà d'`i64` (que la forme canonique rend flottant).
fn number_cell() -> impl Strategy<Value = Option<ParamValue>> {
    proptest::option::of(
        prop_oneof![
            any::<i64>().prop_map(Value::from),
            any::<u64>().prop_map(Value::from),
            (-1e6f64..1e6).prop_map(Value::from),
        ]
        .prop_map(ParamValue::Value),
    )
}

/// La liste `lignes` de `mesures` : une colonne `choice` et une colonne `number`.
fn measures() -> impl Strategy<Value = Option<ParamValue>> {
    let level = proptest::option::of(prop_oneof![Just("bas"), Just("haut")].prop_map(|v| ParamValue::Text(v.into())));
    let columns = (level, number_cell()).prop_map(|(niveau, valeur)| row(vec![("niveau", niveau), ("valeur", valeur)]));
    list_param(3, columns.boxed())
}

fn images() -> impl Strategy<Value = Option<ParamValue>> {
    let columns =
        proptest::option::of(Just(ParamValue::Text("sha256:00".into()))).prop_map(|img| row(vec![("img", img)]));
    list_param(2, columns.boxed())
}

fn leaf() -> impl Strategy<Value = Element> {
    prop_oneof![
        steps().prop_map(|etapes| with("liste", vec![("etapes", etapes)])),
        measures().prop_map(|lignes| with("mesures", vec![("lignes", lignes)])),
        (cell_text(), images()).prop_map(|(t, images)| {
            let bornee =
                t.map(|t| ParamValue::List(vec![ListItem { id: "b".into(), values: row(vec![("t", Some(t))]) }]));
            with("bornes", vec![("bornee", bornee), ("images", images)])
        }),
        paragraphs().prop_map(|p| with(base::TEXT, vec![("body", Some(ParamValue::Rich(p)))])),
        (text_param(), text_param(), any::<bool>(), paragraphs()).prop_map(|(title, kicker, highlight, body)| {
            let highlight = highlight.then(|| ParamValue::Value(Value::Bool(true)));
            with(
                base::CARD,
                vec![
                    ("title", title),
                    ("kicker", kicker),
                    ("highlight", highlight),
                    ("body", Some(ParamValue::Rich(body))),
                ],
            )
        }),
        Just(with(base::SHAPE, vec![("shape", Some(ParamValue::Text("ellipse".into())))])),
        Just(with(base::IMAGE, vec![("asset", Some(ParamValue::Text("sha256:00".into())))])),
    ]
}

fn element() -> impl Strategy<Value = Element> {
    prop_oneof![
        3 => leaf(),
        1 => proptest::collection::vec(leaf(), 0..3)
            .prop_map(|children| with(base::STACK, vec![("children", Some(ParamValue::Slot(children)))])),
    ]
}

fn json_value() -> impl Strategy<Value = Value> {
    let atom = prop_oneof![
        any::<i64>().prop_map(Value::from),
        any::<u64>().prop_map(Value::from),
        (-1e6f64..1e6).prop_map(Value::from),
        "[ a]{0,3}".prop_map(Value::from),
        any::<bool>().prop_map(Value::from),
        Just(Value::Null),
    ];
    atom.prop_recursive(2, 8, 3, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..3).prop_map(Value::from),
            proptest::collection::btree_map("[a-c]", inner, 0..3).prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

fn slide() -> impl Strategy<Value = Slide> {
    (
        (prose(), prose(), text_param(), text_param(), text_param()),
        "[ a\n]{0,3}",
        proptest::collection::vec(element(), 0..3),
        proptest::option::of(chrono_steps()),
    )
        .prop_map(|((name, description, title, subtitle, kicker), notes, blocks, chrono)| {
            let mut s = Slide { name, description, notes, ..Slide::new("", "content") };
            if let Some(etapes) = chrono {
                s.kind = "chrono".into();
                if let Some(etapes) = etapes {
                    s.params.set("etapes", etapes);
                }
            }
            for (k, v) in [("title", title), ("subtitle", subtitle), ("kicker", kicker)] {
                if let Some(v) = v {
                    s.params.set(k, v);
                }
            }
            s.params.set("blocks", ParamValue::Slot(blocks));
            s
        })
}

/// Ids uniques, dans l'ordre de parcours.
fn number(params: &mut Params, next: &mut usize) {
    for (_, list) in params.slots_mut() {
        for e in list {
            e.id = format!("e{next}");
            *next += 1;
            number(&mut e.params, next);
        }
    }
}

fn deck() -> impl Strategy<Value = Deck> {
    (
        (prose(), prose(), prose()),
        proptest::collection::btree_map("[a-c]", json_value(), 0..3),
        prop_oneof![Just(Value::Null), Just(json!({})), json_value()],
        proptest::collection::vec(slide(), 0..3),
    )
        .prop_map(|((title, author, date), vars, tokens, mut slides)| {
            let mut next = 0;
            for (i, s) in slides.iter_mut().enumerate() {
                s.id = format!("s{i}");
                number(&mut s.params, &mut next);
            }
            let params = DeckParams { title, author, date, ..Default::default() };
            Deck { version: DECK_VERSION, theme: None, params, vars, tokens, widgets: Default::default(), slides }
        })
}

/// Garde-fou : la stratégie produit bien des listes non vides, vides et absentes, et des cellules `number` et `choice`.
#[test]
fn deck_strategy_generates_list_states_and_number_and_choice_cells() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
    let mut runner = TestRunner::new_with_rng(Config::default(), TestRng::from_seed(RngAlgorithm::ChaCha, &[7; 32]));
    let (mut filled, mut empty, mut absent, mut numbers, mut big, mut floats, mut choices) = (0, 0, 0, 0, 0, 0, 0);
    for _ in 0..200 {
        let deck = deck().new_tree(&mut runner).expect("la stratégie produit un deck").current();
        for slide in &deck.slides {
            for e in slide.params.slot("blocks").expect("blocks est posé") {
                let state = match e.kind.as_str() {
                    "liste" => e.params.get("etapes"),
                    "mesures" => e.params.get("lignes"),
                    _ => continue,
                };
                match state {
                    Some(ParamValue::List(items)) if items.is_empty() => empty += 1,
                    Some(ParamValue::List(items)) => {
                        filled += 1;
                        for cell in items.iter().filter_map(|i| i.values.get("valeur")) {
                            let ParamValue::Value(v) = cell else { panic!("valeur non numérique : {cell:?}") };
                            numbers += 1;
                            big += usize::from(v.as_i64().is_none() && v.is_u64());
                            floats += usize::from(v.is_f64());
                        }
                        choices += items.iter().filter(|i| i.values.get("niveau").is_some()).count();
                    }
                    _ => absent += 1,
                }
            }
        }
    }
    let seen = format!("{filled} {empty} {absent} {numbers} {big} {floats} {choices}");
    assert!(
        filled > 0 && empty > 0 && absent > 0,
        "listes (pleines, vides, absentes, nombres, > i64, flottants, choix) : {seen}"
    );
    assert!(numbers > 0 && big > 0 && floats > 0 && choices > 0, "cellules : {seen}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn canonical_deck_reads_back_unchanged(d in deck()) {
        let d = canonical(d);
        let (back, issues) = DeckReplica::new(&d, crate::test_registry::with_list()).unwrap().deck();
        prop_assert!(issues.is_empty());
        prop_assert_eq!(back, d);
    }

    #[test]
    fn canonicalize_is_idempotent(d in deck()) {
        let once = canonical(d);
        prop_assert_eq!(canonical(once.clone()), once);
    }

    /// Chemin de `SourceBundle::apply` : réconciliation par diff d'un deck à l'autre.
    #[test]
    fn canonical_deck_reconciled_over_another_reads_back_unchanged(a in deck(), b in deck()) {
        let (a, b) = (canonical(a), canonical(b));
        let mut bundle = SourceBundle::new(a, crate::test_registry::with_list()).unwrap();
        bundle.apply(b.clone(), "test", None).unwrap();
        prop_assert_eq!(bundle.replica().deck().0, b);
    }
}

#[test]
fn ids_fresh_with_two_thousand_suffixes_taken_is_linear() {
    let mut ids = Ids::new((2..=2_000).map(|n| format!("x-{n}")).chain(["x".to_owned()]));
    let names: Vec<String> = (0..2_000).map(|_| ids.fresh("x")).collect();
    let expected: Vec<String> = (2_001..=4_000).map(|n| format!("x-{n}")).collect();
    assert_eq!(names, expected);
    // 1 999 suffixes pris sautés, plus 2 000 succès.
    assert_eq!(ids.tries, 3_999);
}

#[test]
fn ids_fresh_bases_do_not_interfere() {
    let mut ids = Ids::new(["x".to_owned(), "x-3".to_owned()]);
    assert_eq!(ids.fresh("x"), "x-2");
    assert_eq!(ids.fresh("x-2"), "x-2-2");
    assert_eq!(ids.fresh("x"), "x-4");
    assert_eq!(ids.fresh("x-2"), "x-2-3");
}

#[test]
fn unique_ids_after_a_taken_suffix_keeps_the_same_names() {
    let blocks = ["x", "x", "x-2", "x"]
        .into_iter()
        .zip(["a", "b", "c", "d"])
        .map(|(id, text)| Element::text(id, vec![Paragraph { runs: vec![run(text)], list: None }]))
        .collect();
    let mut deck = deck_with(blocks);
    unique_ids(&mut deck.slides);
    let ids: Vec<String> = match deck.slides[0].params.get("blocks") {
        Some(ParamValue::Slot(elements)) => elements.iter().map(|e| e.id.clone()).collect(),
        other => panic!("slot attendu : {other:?}"),
    };
    assert_eq!(ids, ["x", "x-3", "x-2", "x-4"]);
}

#[test]
fn canonicalize_nested_list_duplicate_ids_are_repaired_within_their_list() {
    let tasks: Vec<ListItem> =
        serde_json::from_value(serde_json::json!([{ "id": "t", "texte": "a" }, { "id": "t", "texte": "b" }])).unwrap();
    let mut values = Params::default();
    values.set("taches", ParamValue::List(tasks));
    let mut d = deck_with(vec![]);
    d.slides[0].params.set("etapes", ParamValue::List(vec![ListItem { id: "a".into(), values }]));
    let d = canonical(d);
    let step = &d.slides[0].params.list("etapes").unwrap_or_default()[0];
    let ids: Vec<&str> = step.values.list("taches").unwrap_or_default().iter().map(|t| t.id.as_str()).collect();
    assert_that!(ids.len()).is_equal_to(2);
    assert_that!(ids[0] != ids[1]).is_true();
}
