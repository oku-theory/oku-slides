use proptest::prelude::*;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::model::*;
use crate::{DeckReplica, SourceBundle};

fn run(t: &str) -> Run {
    Run::plain(t)
}

fn bold(t: &str) -> Run {
    Run { t: t.into(), bold: true, ..Default::default() }
}

fn deck_with(elements: Vec<Element>) -> Deck {
    let mut slide = Slide::new("s", SlideKind::Content);
    slide.blocks = elements;
    Deck {
        version: DECK_VERSION,
        theme: None,
        params: Params::default(),
        vars: Default::default(),
        tokens: Value::Null,
        templates: None,
        slides: vec![slide],
    }
}

fn text(runs: Vec<Run>) -> Deck {
    deck_with(vec![Element::text("e", vec![Paragraph { runs, list: None }])])
}

fn runs(deck: &Deck) -> &[Run] {
    match &deck.slides[0].blocks[0].body {
        ElementBody::Text { text } => &text[0].runs,
        other => panic!("pas un texte : {other:?}"),
    }
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
    let card = Element {
        id: "c".into(),
        frame: None,
        cell: None,
        style: Style::default(),
        body: ElementBody::Card { title: Some(" ".into()), kicker: Some(" k ".into()), highlight: false, text: vec![] },
    };
    let mut d = deck_with(vec![card]);
    d.params.author = Some("\n ".into());
    d.params.title = Some(" Titre ".into());
    d.slides[0].title = Some("\t".into());
    d.slides[0].name = Some(" nom".into());
    let d = canonical(d);
    assert_that!(d.params.author).is_none();
    assert_that!(d.params.title).is_equal_to(Some("Titre".to_owned()));
    assert_that!(d.slides[0].title).is_none();
    assert_that!(d.slides[0].name).is_equal_to(Some("nom".to_owned()));
    let ElementBody::Card { title, kicker, .. } = d.slides[0].blocks[0].body.clone() else { panic!("pas une carte") };
    assert_that!(title).is_none();
    assert_that!(kicker).is_equal_to(Some("k".to_owned()));
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
fn canonicalize_group_children_are_canonicalized() {
    let child = Element::text("t", vec![Paragraph { runs: vec![run("a"), run("b")], list: None }]);
    let group = Element {
        id: "g".into(),
        frame: None,
        cell: None,
        style: Style::default(),
        body: ElementBody::Group { layout: GroupLayout::default(), children: vec![child] },
    };
    let d = canonical(deck_with(vec![group]));
    let ElementBody::Text { text } = &d.slides[0].blocks[0].children()[0].body else { panic!("pas un texte") };
    assert_that!(text[0].runs.to_vec()).is_equal_to(vec![run("ab")]);
}

/// Cas de l'UI : une ligne vide de l'éditeur donne `[{t:""}]`.
#[test]
fn source_bundle_new_with_empty_run_view_equals_replica_read() {
    let b = SourceBundle::new(text(vec![run("")])).unwrap();
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

fn leaf() -> impl Strategy<Value = ElementBody> {
    prop_oneof![
        paragraphs().prop_map(|text| ElementBody::Text { text }),
        (prose(), prose(), any::<bool>(), paragraphs())
            .prop_map(|(title, kicker, highlight, text)| ElementBody::Card { title, kicker, highlight, text }),
        Just(ElementBody::Shape { shape: ShapeKind::Ellipse }),
        Just(ElementBody::Image { asset: "sha256:00".into(), fit: None }),
    ]
}

fn element(body: ElementBody) -> Element {
    let frame = Some(Frame { x: 1.5, y: 2.0, w: 300.0, h: 40.25 });
    Element { id: String::new(), frame, cell: None, style: Style::default(), body }
}

fn body() -> impl Strategy<Value = ElementBody> {
    prop_oneof![
        3 => leaf(),
        1 => proptest::collection::vec(leaf().prop_map(element), 0..3)
            .prop_map(|children| ElementBody::Group { layout: GroupLayout::default(), children }),
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
        (prose(), prose(), prose(), prose(), prose(), prose()),
        "[ a\n]{0,3}",
        proptest::collection::vec(body().prop_map(element), 0..3),
    )
        .prop_map(|((name, description, title, subtitle, kicker, text), notes, blocks)| Slide {
            name,
            description,
            title,
            subtitle,
            kicker,
            text,
            notes,
            blocks,
            ..Slide::new("", SlideKind::Content)
        })
}

/// Ids uniques, dans l'ordre de parcours.
fn number(elements: &mut [Element], next: &mut usize) {
    for e in elements {
        e.id = format!("e{next}");
        *next += 1;
        if let Some(children) = e.children_mut() {
            number(children, next);
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
                number(&mut s.blocks, &mut next);
            }
            let params = Params { title, author, date, ..Default::default() };
            Deck { version: DECK_VERSION, theme: None, params, vars, tokens, templates: None, slides }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn canonical_deck_reads_back_unchanged(d in deck()) {
        let d = canonical(d);
        let (back, issues) = DeckReplica::new(&d).unwrap().deck();
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
        let mut bundle = SourceBundle::new(a).unwrap();
        bundle.apply(b.clone(), "test").unwrap();
        prop_assert_eq!(bundle.replica().deck().0, b);
    }
}
