use speculoos::prelude::*;

use super::*;

/// Rejoue `left` (fichiers non sondés après chaque appel) ; rend le résultat et le nombre d'appels.
fn rounds(left: &[usize]) -> (Result<usize, &'static str>, usize) {
    let mut calls = 0;
    let out = probe_rounds(
        || {
            calls += 1;
            Ok(left[(calls - 1).min(left.len() - 1)])
        },
        |n| *n,
        || false,
    );
    (out, calls)
}

#[test]
fn probe_rounds_stops_once_every_file_is_probed() {
    let (out, calls) = rounds(&[3, 1, 0, 0]);
    assert_that!(out).is_equal_to(Ok(0));
    assert_that!(calls).is_equal_to(3);
}

#[test]
fn probe_rounds_stops_when_a_round_probes_nothing() {
    // Processus de rendu indisponible : le deuxième appel ne sonde rien, on s'arrête.
    let (out, calls) = rounds(&[5, 5, 5]);
    assert_that!(out).is_equal_to(Ok(5));
    assert_that!(calls).is_equal_to(2);
}

#[test]
fn probe_rounds_makes_at_most_the_bounded_number_of_calls() {
    let left: Vec<usize> = (0..20).rev().collect();
    let (out, calls) = rounds(&left[..19]);
    assert_that!(calls).is_equal_to(PROBE_ROUNDS);
    assert_that!(out).is_equal_to(Ok(19 - PROBE_ROUNDS + 1));
}

#[test]
fn probe_rounds_stops_at_the_first_error() {
    let mut calls = 0;
    let out: Result<usize, &str> = probe_rounds(
        || {
            calls += 1;
            if calls == 1 { Ok(4) } else { Err("lien rompu") }
        },
        |n| *n,
        || false,
    );
    assert_that!(out).is_equal_to(Err("lien rompu"));
    assert_that!(calls).is_equal_to(2);
}

#[test]
fn probe_rounds_stops_once_the_total_time_is_spent() {
    // Chaque appel épuise son budget : le temps total est écoulé après le troisième.
    let calls = std::cell::Cell::new(0);
    let out: Result<usize, &str> = probe_rounds(
        || {
            calls.set(calls.get() + 1);
            Ok(10 - calls.get())
        },
        |n| *n,
        || calls.get() >= PROBE_BUDGETS as usize,
    );
    assert_that!(calls.get()).is_equal_to(PROBE_BUDGETS as usize);
    assert_that!(out).is_equal_to(Ok(10 - PROBE_BUDGETS as usize));
}

// ---- Assemblage pack par pack ------------------------------------------------------------

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

/// Les épingles `(espace, besoins)` d'un même pack (seuls les besoins comptent ici).
fn pins_of(entries: &[(&str, &[&str])]) -> BTreeMap<Namespace, PackPin> {
    let version = PackVersion::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    entries
        .iter()
        .map(|(ns, requires)| {
            let requires = requires.iter().map(|r| crate::test_support::ns(r)).collect();
            (crate::test_support::ns(ns), PackPin { id: uuid::Uuid::nil(), version: version.clone(), requires })
        })
        .collect()
}

fn widgets_of(ns: &str) -> PackWidgets {
    let files = crate::test_support::widget_files(&[("quote", QUOTE)]);
    PackWidgets::new(format!("id-{ns}"), crate::test_support::ns(ns), files).unwrap()
}

/// Les ensembles essayés par un assemblage, par leurs espaces de noms.
type Tried = std::rc::Rc<std::cell::RefCell<Vec<Vec<String>>>>;

/// Un assemblage qui échoue dès que l'ensemble contient `bad` ; les ensembles essayés.
fn failing_on(bad: &str) -> (impl FnMut(&PackSet) -> Result<Vec<String>, RegistryError>, Tried) {
    let tried = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let seen = tried.clone();
    let bad = bad.to_owned();
    let assemble = move |set: &PackSet| {
        let names: Vec<String> = set.packs().iter().map(|p| p.namespace().to_string()).collect();
        seen.borrow_mut().push(names.clone());
        if names.contains(&bad) {
            let e = oku_slides_render::RenderError::worker_lost("panne simulée");
            return Err(RegistryError::worker(e));
        }
        Ok(names)
    };
    (assemble, tried)
}

#[test]
fn assemble_each_of_a_set_that_assembles_tries_it_once() {
    let pins = pins_of(&[("acme", &[]), ("beta", &[])]);
    let (assemble, tried) = failing_on("none");
    let (set, registry, error) = assemble_each(&pins, vec![widgets_of("acme"), widgets_of("beta")], assemble);
    assert_that!(set.packs().len()).is_equal_to(2);
    assert_that!(registry).is_equal_to(Some(vec!["acme".to_owned(), "beta".to_owned()]));
    assert_that!(error.is_none()).is_true();
    assert_that!(tried.borrow().len()).is_equal_to(1);
}

#[test]
fn assemble_each_drops_only_the_failing_pack_and_its_dependents() {
    let pins = pins_of(&[("acme", &[]), ("bad", &[]), ("top", &["bad"]), ("zed", &[])]);
    let packs = vec![widgets_of("top"), widgets_of("acme"), widgets_of("bad"), widgets_of("zed")];
    let (assemble, tried) = failing_on("bad");
    let (set, registry, error) = assemble_each(&pins, packs, assemble);
    let kept: Vec<&str> = set.packs().iter().map(|p| p.namespace().as_str()).collect();
    assert_that!(kept).is_equal_to(vec!["acme", "zed"]);
    assert_that!(registry).is_equal_to(Some(vec!["acme".to_owned(), "zed".to_owned()]));
    assert_that!(error.map(|e| e.kind())).is_equal_to(Some(oku_slides_render::RegistryErrorKind::Setup));
    // `top` requiert `bad`, écarté : il n'est jamais essayé.
    assert_that!(tried.borrow().iter().any(|t| t.len() < 4 && t.contains(&"top".to_owned()))).is_false();
}

#[test]
fn assemble_each_with_no_pack_assembling_has_no_registry() {
    let pins = pins_of(&[("bad", &[])]);
    let (assemble, _) = failing_on("bad");
    let (set, registry, error) = assemble_each(&pins, vec![widgets_of("bad")], assemble);
    assert_that!(set.is_empty()).is_true();
    assert_that!(registry).is_none();
    assert_that!(error.is_some()).is_true();
}

// ---- Sonde hors session : épingles et besoins ----------------------------------------------

/// Les fichiers d'un pack `namespace` qui requiert `requires`, au widget `quote`.
fn pack_files(namespace: &str, requires: &[&str]) -> (PackPin, PackFiles) {
    let m = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), namespace, requires);
    let files = m.to_files(crate::test_support::widget_files(&[("quote", QUOTE)])).unwrap();
    (PackPin { id: m.id, version: files.version(), requires: m.requires.clone() }, files)
}

fn probe_of(pins: BTreeMap<Namespace, PackPin>, packs: Vec<PackFiles>) -> ProbedWidgets {
    let data = crate::test_support::fresh_dir("oku-slides-probed-");
    let platform = crate::test_support::workspace_platform(data.path());
    let base = oku_slides_render::base_registry(&Fonts::default()).unwrap();
    let mut deck = Deck::empty();
    deck.widgets = pins;
    let packs = packs.into_iter().map(|f| (f.version(), f)).collect();
    ProbedWidgets::probe(&platform, &base, &deck, &packs)
}

#[test]
fn probe_disables_a_pack_requiring_one_whose_version_is_missing() {
    let (acme, acme_files) = pack_files("acme", &["beta"]);
    let (beta, _) = pack_files("beta", &[]);
    let pins = BTreeMap::from([(crate::test_support::ns("acme"), acme), (crate::test_support::ns("beta"), beta)]);
    let probed = probe_of(pins, vec![acme_files]);
    assert_that!(probed.registry.get("acme/quote")).is_none();
    assert_that!(probed.has_packs()).is_false();
}

#[test]
fn probe_disables_a_pin_under_another_namespace_than_its_manifest_with_a_localized_error() {
    let (acme, files) = pack_files("acme", &[]);
    let pins =
        BTreeMap::from([(crate::test_support::ns("acme"), acme.clone()), (crate::test_support::ns("autre"), acme)]);
    let probed = probe_of(pins, vec![files]);
    assert_that!(probed.registry.get("acme/quote")).is_some();
    assert_that!(probed.registry.get("autre/quote")).is_none();
    let expected =
        oku_slides_i18n::Localizer::new(Some("fr")).tr_with("issue-pack-pin-mismatch", &[("pack", "autre".into())]);
    assert_that!(probed.probe_failed).is_equal_to(Some(expected));
}
