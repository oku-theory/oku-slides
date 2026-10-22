//! Le retrofit de bout en bout : les fixtures `minimal`, `kitchen` et le vrai
//! deck `alto-wealth.pptx`, projetés sur les profils des packs de layouts
//! choisis (`essentiel`, et des packs de test) et les thèmes intégrés (Papier,
//! Pro). Le plan s'applique en une transaction et le deck obtenu ne lève
//! aucune `Issue`.

mod common;

use std::collections::HashSet;

use oku_slides_core::kinds::{self, base};
use oku_slides_core::{Deck, Element, LayoutProfile, Namespace, Op, apply_all, validate};
use oku_slides_import::report::LossKind;
use oku_slides_import::retrofit::{ColorTokens, PageTokens, project};
use oku_slides_import::source::SourceDeck;
use oku_slides_import::{ImportError, ImportErrorKind, ImportPlan, RetrofitTarget, read_pptx};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn source(fixture: &str) -> SourceDeck {
    let bytes = match fixture {
        "alto" => common::file("alto-wealth.pptx"),
        name => common::pptx(name),
    };
    read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture de {fixture} : {e}"))
}

/// Un pack de layouts de test : son espace de noms, ses layouts sondés, ses profils.
struct LayoutPack {
    namespace: &'static str,
    widgets: Vec<Value>,
    profiles: Vec<LayoutProfile>,
}

/// Le pack `acme` : un layout `grille` (copie d'`essentiel/content`) et son seul profil, une
/// grille 12 × 6 à quatre blocs, mieux notée que `libre` pour des contenus disposés librement.
fn acme_grid_pack() -> LayoutPack {
    acme_grid_pack_of(12, 6)
}

/// [`acme_grid_pack`], sur une grille de `columns` × `rows` cellules.
fn acme_grid_pack_of(columns: u32, rows: u32) -> LayoutPack {
    let probe = common::builtin_probe();
    let essentiel = probe["packs"].as_array().and_then(|p| p.iter().find(|p| p["namespace"] == "essentiel"));
    let content = essentiel
        .and_then(|p| p["widgets"].as_array())
        .and_then(|w| w.iter().find(|w| w["name"] == "content"))
        .unwrap_or_else(|| panic!("essentiel/content dans la fixture"));
    let mut grille = content.clone();
    grille["name"] = json!("grille");
    let profile = json!({
        "id": "grille", "name": "Grille", "layout": "grille", "role": "content", "arrangement": "grid",
        "blocks": 4, "title": true, "params": { "grid": { "columns": columns, "rows": rows } },
    });
    LayoutPack {
        namespace: "acme",
        widgets: vec![grille],
        profiles: vec![serde_json::from_value(profile).unwrap_or_else(|e| panic!("{e}"))],
    }
}

/// Un pack de layouts `namespace` sans layout ni profil.
fn empty_pack(namespace: &'static str) -> LayoutPack {
    LayoutPack { namespace, widgets: Vec::new(), profiles: Vec::new() }
}

/// Le plan de `fixture` sur les packs de layouts `packs` et le thème `theme`, et le deck obtenu.
fn import(fixture: &str, packs: &[&str], theme: &str) -> (ImportPlan, Deck) {
    import_source(fixture, &source(fixture), packs, theme)
}

/// [`import`], les packs `packs` cherchés aussi parmi le pack de test `extra`.
fn import_with_extra_pack(fixture: &str, packs: &[&str], theme: &str, extra: LayoutPack) -> (ImportPlan, Deck) {
    import_with(fixture, &source(fixture), packs, theme, &[extra]).unwrap_or_else(|e| panic!("{fixture} : {e}"))
}

/// L'erreur du retrofit de `fixture` sur `packs` (cherchés aussi parmi `extra`).
fn import_err(fixture: &str, packs: &[&str], extra: LayoutPack) -> ImportError {
    match import_with(fixture, &source(fixture), packs, "papier", &[extra]) {
        Ok(_) => panic!("{fixture} : retrofit accepté"),
        Err(e) => e,
    }
}

/// Le plan de la source `src` (nommée `fixture`) sur `packs` et `theme`, et le deck obtenu.
fn import_source(fixture: &str, src: &SourceDeck, packs: &[&str], theme: &str) -> (ImportPlan, Deck) {
    import_with(fixture, src, packs, theme, &[]).unwrap_or_else(|e| panic!("{fixture} : {e}"))
}

/// Le retrofit de `src` sur les profils des packs `packs` (`essentiel`, ou l'un de `extra`), dans
/// l'ordre, et le thème `theme` ; le deck obtenu, contrôlé.
fn import_with(
    fixture: &str,
    src: &SourceDeck,
    packs: &[&str],
    theme: &str,
    extra: &[LayoutPack],
) -> Result<(ImportPlan, Deck), ImportError> {
    let mut profiles: Vec<(Namespace, LayoutProfile)> = Vec::new();
    for name in packs {
        let ns = Namespace::parse(name).unwrap_or_else(|| panic!("{name}"));
        let list = match extra.iter().find(|p| p.namespace == *name) {
            Some(p) => p.profiles.clone(),
            None if *name == kinds::essentiel::NAMESPACE => common::profiles(name),
            None => panic!("pack de test inconnu : {name}"),
        };
        profiles.extend(list.into_iter().map(|p| (ns.clone(), p)));
    }
    let probed = extra.iter().filter(|p| !p.widgets.is_empty());
    let registry = common::registry_plus(
        probed.map(|p| json!({ "namespace": p.namespace, "kind": "layouts", "widgets": p.widgets })).collect(),
    );
    let tokens = common::tokens(theme);
    let target = RetrofitTarget {
        profiles: &profiles,
        registry: &registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let plan = project(src, &target)?;
    // La session épingle `base` et `essentiel` avant d'appliquer un retrofit.
    let mut ops = common::builtin_pins();
    ops.extend(plan.ops.clone());
    let mut deck = Deck::empty();
    let label = format!("{fixture}/{}/{theme}", packs.join("+"));
    apply_all(&mut deck, ops, &registry).unwrap_or_else(|e| panic!("{label} : {e:?}"));
    let assets: HashSet<String> = plan.assets.keys().cloned().collect();
    let issues = validate(&deck, &registry, Some(&assets));
    assert!(issues.is_empty(), "{label} : {issues:?}");
    Ok((plan, deck))
}

fn profiles_chosen(plan: &ImportPlan) -> Vec<&str> {
    plan.report.slides.iter().map(|s| s.profile.as_str()).collect()
}

#[rstest]
fn retrofit_applies_without_issue_and_covers_everything(
    #[values("minimal", "kitchen", "alto")] fixture: &str,
    #[values("papier", "pro")] theme: &str,
) {
    let (plan, deck) = import(fixture, &["essentiel"], theme);
    assert_that!(deck.slides.len()).is_equal_to(source(fixture).slides.len());
    assert_that!(plan.report.slides.len()).is_equal_to(deck.slides.len());
    let c = plan.report.coverage.unwrap_or_else(|| panic!("couverture mesurée"));
    assert_that!(c.missing).is_empty();
    assert_that!([c.runs, c.images, c.tables, c.charts].iter().all(|r| r.is_complete())).is_true();
}

#[test]
fn retrofit_kitchen_coverage_counts_every_content() {
    let (plan, _) = import("kitchen", &["essentiel"], "papier");
    let c = plan.report.coverage.unwrap_or_else(|| panic!("couverture mesurée"));
    let totals = (c.runs.total, c.images.total, c.tables.total, c.charts.total);
    assert_that!(totals).is_equal_to((12, 2, 1, 1));
}

#[test]
fn retrofit_minimal_fills_the_deck_title_and_picks_profiles() {
    let (plan, deck) = import("minimal", &["essentiel"], "papier");
    assert_that!((deck.params.title.as_deref(), deck.params.subtitle.as_deref()))
        .is_equal_to((Some("Rapport annuel"), Some("Exercice 2026")));
    assert_that!(profiles_chosen(&plan)).is_equal_to(vec![
        "essentiel/couverture",
        "essentiel/libre",
        "essentiel/texte",
    ]);
    assert_that!(deck.slides[1].notes.starts_with("Dire bonjour.")).is_true();
}

#[test]
fn retrofit_minimal_ops_match() {
    let (plan, _) = import("minimal", &["essentiel"], "papier");
    insta::assert_json_snapshot!("minimal_ops", plan.ops);
}

#[test]
fn retrofit_kitchen_keeps_table_chart_picture_and_fallbacks() {
    let (plan, deck) = import("kitchen", &["essentiel"], "papier");
    let kinds = common::deck_kinds(&deck);
    for k in [base::TABLE, base::CHART, base::IMAGE, base::STACK, base::CARD] {
        assert_that!(kinds.iter().any(|x| x == k)).named(k).is_true();
    }
    // L'image de la slide 2 et le repli de l'équation de la slide 3 sont le même PNG : un seul asset.
    assert_that!(plan.assets.len()).is_equal_to(1);
    let requests: Vec<(Vec<String>, &str)> =
        plan.widget_requests.iter().map(|r| (r.slides.clone(), r.fallback.as_str())).collect();
    // Le groupe de la slide 2 (deux boîtes, rien d'omis) se replie sans demande ; le SmartArt de la slide 3 en fait une.
    assert_that!(requests).is_equal_to(vec![(vec!["s3".to_owned()], base::STACK)]);
}

#[test]
fn retrofit_kitchen_ops_match() {
    let (plan, _) = import("kitchen", &["essentiel"], "papier");
    insta::assert_json_snapshot!("kitchen_ops", plan.ops);
}

#[test]
fn retrofit_alto_infers_the_cover_and_the_titles() {
    let (plan, deck) = import("alto", &["essentiel"], "pro");
    assert_that!(deck.params.title.as_deref()).is_equal_to(Some("The Agent Is the New Front Door"));
    assert_that!(deck.params.subtitle.as_deref()).is_equal_to(Some("From Software Vendor to Wealth Infrastructure"));
    let titled: Vec<bool> = deck.slides[1..].iter().map(|s| s.params.text("title").is_some()).collect();
    assert_that!(titled).is_equal_to(vec![true; 7]);
    // Les flèches de la slide 3 restent des images ; le logo répété est omis.
    assert_that!(common::deck_kinds(&deck).iter().filter(|k| *k == base::IMAGE).count()).is_equal_to(4);
    assert_that!(plan.assets.len()).is_equal_to(1);
}

/// Les éléments porteurs de texte (y compris leurs descendants) d'une liste libre.
fn texts_with_place(list: &[Element]) -> Vec<&Element> {
    list.iter()
        .filter(|e| matches!(e.kind.as_str(), base::TEXT | base::CARD | base::STACK | base::TABLE | base::CHART))
        .collect()
}

fn meet(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Aucun texte n'en recouvre un autre : cellules (grilles) ou cadres (`libre`) disjoints.
#[rstest]
fn retrofit_alto_places_no_text_over_another(#[values("papier", "pro")] theme: &str) {
    let (_, deck) = import("alto", &["essentiel"], theme);
    for s in &deck.slides {
        let placed = texts_with_place(s.params.slot("elements").unwrap_or_default());
        let boxes: Vec<(f64, f64, f64, f64)> = placed
            .iter()
            .map(|e| match (e.cell, e.frame) {
                (Some(c), _) => (f64::from(c.col), f64::from(c.row), f64::from(c.cols), f64::from(c.rows)),
                (None, Some(f)) => (f.x, f.y, f.w, f.h),
                (None, None) => panic!("{} : élément libre sans place", e.id),
            })
            .collect();
        for (i, a) in boxes.iter().enumerate() {
            for (j, b) in boxes.iter().enumerate().skip(i + 1) {
                assert_that!(meet(*a, *b))
                    .named(&format!("{} : {} et {}", s.id, placed[i].id, placed[j].id))
                    .is_false();
            }
        }
    }
}

#[test]
fn retrofit_alto_makes_cards_and_picks_card_profiles() {
    let (plan, deck) = import("alto", &["essentiel"], "papier");
    assert_that!(profiles_chosen(&plan)).is_equal_to(vec![
        "essentiel/couverture",
        "essentiel/cartes",
        "essentiel/libre",
        "essentiel/cartes",
        "essentiel/colonnes",
        "essentiel/cartes",
        "essentiel/cartes",
        "essentiel/cartes",
    ]);
    let cards = common::deck_kinds(&deck).iter().filter(|k| *k == base::CARD).count();
    // 3 + 6 + 4 + 3 + 4 + 4 + 4 cartes (slides 2 à 8).
    assert_that!(cards).is_equal_to(28);
}

/// La couverture d'alto n'a que des textes : elle reste une couverture, ses
/// textes sans paramètre vont dans ses notes, dits au rapport (t-817).
#[test]
fn retrofit_alto_cover_stays_a_cover_and_keeps_its_extra_texts_in_notes() {
    let (plan, deck) = import("alto", &["essentiel"], "papier");
    assert_that!(deck.slides[0].kind.as_str()).is_equal_to(kinds::essentiel::TITLE);
    assert_that!(deck.slides[0].notes.contains("FROM A SURVIVAL QUESTION TO A GENERATIONAL OPPORTUNITY")).is_true();
    let moved = plan.report.slides[0].losses.iter().filter(|l| l.kind == LossKind::MovedToNotes).count();
    assert_that!(moved).is_greater_than(0);
}

/// t-816 : les flux qui sortaient de la page n'en sortent plus. Alto, slides
/// 6 et 7 sur Papier : le flux reste, son texte prend une taille plus
/// petite du thème. Kitchen, slide 2 (graphique et image au-dessus de
/// « Gauche » et « Droite ») : sur Pro, la slide passe en grille, où chacun
/// tient ; sur Papier, la rangée du bas est trop basse pour la pile et le
/// flux ne tient pas même en petit : le débordement est dit au rapport.
#[test]
fn retrofit_known_overflowing_flows_are_reduced_or_placed_on_the_grid() {
    let small = |e: &Element| -> bool {
        let mut runs = Vec::new();
        fn walk<'e>(e: &'e Element, out: &mut Vec<&'e oku_slides_core::Run>) {
            out.extend(e.params.rich("body").unwrap_or_default().iter().flat_map(|p| &p.runs));
            for (_, c) in e.params.slots() {
                for child in c {
                    walk(child, out);
                }
            }
        }
        walk(e, &mut runs);
        !runs.is_empty() && runs.iter().all(|r| r.size.is_some())
    };
    let (plan, deck) = import("alto", &["essentiel"], "papier");
    for k in [5, 6] {
        let blocks = deck.slides[k].params.slot("blocks").unwrap_or_default();
        assert_that!(plan.report.slides[k].placement).is_equal_to(oku_slides_import::report::Placement::Flow);
        assert_that!(blocks.iter().all(small)).named(&deck.slides[k].id).is_true();
    }
    let overflows = |s: &oku_slides_import::report::SlideReport| {
        s.losses.iter().any(|l| matches!(l.kind, LossKind::Overflow { .. }))
    };
    let (plan, _) = import("kitchen", &["essentiel"], "pro");
    assert_that!(plan.report.slides[1].placement).is_equal_to(oku_slides_import::report::Placement::Grid);
    assert_that!(overflows(&plan.report.slides[1])).is_false();
    let (plan, _) = import("kitchen", &["essentiel"], "papier");
    assert_that!(plan.report.slides[1].placement).is_equal_to(oku_slides_import::report::Placement::Flow);
    assert_that!(overflows(&plan.report.slides[1])).is_true();
}

#[test]
fn retrofit_alto_reports_its_repeated_footer_once() {
    let (plan, _) = import("alto", &["essentiel"], "pro");
    let repeated: Vec<(String, u32)> = plan
        .report
        .slides
        .iter()
        .flat_map(|s| &s.losses)
        .filter_map(|l| match &l.kind {
            LossKind::RepeatedDecor { text, slides } => Some((text.clone(), *slides)),
            _ => None,
        })
        .collect();
    assert_that!(repeated).is_equal_to(vec![
        ("CONFIDENTIAL".to_owned(), 8),
        ("Amundi".to_owned(), 7),
        ("1 · Wealth Bank as an API · September 20, 2026 · Matthieu Dartiguenave".to_owned(), 6),
    ]);
}

/// Résumé d'une slide : profil, placement, titre, et chaque élément (sorte,
/// cellule ou cadre arrondi au pt, titre d'une carte, enfants d'une pile).
fn describe(e: &Element) -> String {
    let place = match (e.cell, e.frame) {
        (Some(c), _) => format!(" {}:{} {}×{}", c.col, c.row, c.cols, c.rows),
        (None, Some(f)) => format!(" @{:.0},{:.0} {:.0}×{:.0}", f.x, f.y, f.w, f.h),
        (None, None) => String::new(),
    };
    let title = e.params.text("title").map(|t| format!(" « {t} »")).unwrap_or_default();
    let children =
        e.params.slot("children").map(|c| format!(" [{}]", c.iter().map(describe).collect::<Vec<_>>().join(", ")));
    format!("{}{place}{title}{}", e.kind, children.unwrap_or_default())
}

fn summary(plan: &ImportPlan) -> Vec<Value> {
    plan.ops
        .iter()
        .filter_map(|op| match op {
            Op::AddSlide { slide, .. } => Some(slide),
            _ => None,
        })
        .zip(&plan.report.slides)
        .map(|(slide, report)| {
            let elements: Vec<String> = slide.params.slots().flat_map(|(_, list)| list).map(describe).collect();
            json!({
                "slide": slide.id,
                "profile": report.profile,
                "placement": report.placement,
                "kicker": slide.params.text("kicker"),
                "title": slide.params.text("title"),
                "elements": elements,
            })
        })
        .collect()
}

#[test]
fn retrofit_alto_summary_matches() {
    let (plan, _) = import("alto", &["essentiel"], "papier");
    insta::assert_json_snapshot!("alto_retrofit_summary", summary(&plan));
}

/// Les nœuds du SmartArt de kitchen (slide 3) sans dessin, comme l'écrivent LibreOffice ou Google.
const UNDRAWN_DATA: &str = r#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><dgm:ptLst><dgm:pt modelId="{D}" type="doc"><dgm:prSet loTypeId="urn:microsoft.com/office/officeart/2005/8/layout/process1" loCatId="process"/></dgm:pt><dgm:pt modelId="{2}"><dgm:t><a:bodyPr/><a:p><a:r><a:t>Livrer</a:t></a:r></a:p></dgm:t></dgm:pt><dgm:pt modelId="{1}"><dgm:t><a:bodyPr/><a:p><a:r><a:t>Concevoir</a:t></a:r></a:p></dgm:t></dgm:pt><dgm:pt modelId="{1a}"><dgm:t><a:bodyPr/><a:p><a:r><a:t>Maquettes</a:t></a:r></a:p></dgm:t></dgm:pt></dgm:ptLst><dgm:cxnLst><dgm:cxn modelId="{c1}" srcId="{D}" destId="{1}" srcOrd="0" destOrd="0"/><dgm:cxn modelId="{c2}" srcId="{D}" destId="{2}" srcOrd="1" destOrd="0"/><dgm:cxn modelId="{c3}" srcId="{1}" destId="{1a}" srcOrd="0" destOrd="0"/></dgm:cxnLst></dgm:dataModel>"#;

#[test]
fn retrofit_smartart_without_drawing_lays_out_and_covers_its_node_texts() {
    let bytes = common::pptx_with(
        "kitchen",
        &[("ppt/diagrams/data1.xml", Some(UNDRAWN_DATA)), ("ppt/diagrams/drawing1.xml", None)],
    );
    let src = read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let (plan, deck) = import_source("kitchen sans dessin", &src, &["essentiel"], "papier");
    let c = plan.report.coverage.unwrap_or_else(|| panic!("couverture mesurée"));
    assert_that!(c.missing).is_empty();
    // Kitchen compte 12 runs, dont les 2 du dessin ; sans dessin, les 3 nœuds les remplacent.
    assert_that!((c.runs.found, c.runs.total)).is_equal_to((13, 13));
    let mut texts = Vec::new();
    collect_texts(deck.slides[2].params.slot("elements").unwrap_or_default(), &mut texts);
    let order: Vec<&str> =
        texts.iter().map(String::as_str).filter(|t| ["Concevoir", "Maquettes", "Livrer"].contains(t)).collect();
    assert_that!(order).is_equal_to(vec!["Concevoir", "Maquettes", "Livrer"]);
    let lost = plan.report.slides[2].losses.iter().any(|l| l.kind == LossKind::SmartArtLayout);
    assert_that!(lost).is_true();
}

/// Titres et paragraphes de `list` et de leurs descendants, dans l'ordre.
fn collect_texts(list: &[Element], out: &mut Vec<String>) {
    for e in list {
        out.extend(e.params.text("title").map(str::to_owned));
        for p in e.params.rich("body").unwrap_or_default() {
            out.push(p.runs.iter().map(|r| r.t.as_str()).collect());
        }
        for (_, children) in e.params.slots() {
            collect_texts(children, out);
        }
    }
}

#[test]
fn retrofit_deck_element_kinds_are_all_base() {
    let (_, deck) = import("alto", &["essentiel"], "papier");
    let kinds = common::deck_kinds(&deck);
    let stray: Vec<&String> = kinds.iter().filter(|k| !kinds::base::ALL.contains(&k.as_str())).collect();
    assert_that!((
        stray.is_empty(),
        kinds.contains(&kinds::base::TEXT.to_owned()),
        kinds.contains(&kinds::base::STACK.to_owned())
    ))
    .is_equal_to((true, true, true));
}

#[test]
fn retrofit_slide_kinds_are_all_essentiel() {
    let (_, deck) = import("kitchen", &["essentiel"], "papier");
    let stray: Vec<String> =
        deck.slides.iter().map(|s| s.kind.clone()).filter(|k| !kinds::essentiel::ALL.contains(&k.as_str())).collect();
    assert_that!(stray).is_empty();
}

/// Le seul profil d'`acme` (une grille) l'emporte là où `essentiel` n'en propose
/// pas de meilleur : la slide 3 d'alto, aux contenus disposés librement.
#[test]
fn retrofit_on_two_layout_packs_may_use_the_second() {
    let (plan, deck) = import_with_extra_pack("alto", &["essentiel", "acme"], "papier", acme_grid_pack());
    assert_that!(plan.report.slides.iter().any(|s| s.profile.starts_with("acme/"))).is_true();
    assert_that!(plan.report.slides[2].profile.as_str()).is_equal_to("acme/grille");
    assert_that!(deck.slides[2].kind.as_str()).is_equal_to("acme/grille");
    let grid = deck.slides[2].params.get("grid").and_then(|g| serde_json::to_value(g).ok());
    assert_that!(grid).is_equal_to(Some(json!({ "columns": 12, "rows": 6 })));
}

/// Les éléments du slot libre de chaque slide : (accroché à une cellule, posé en cadre).
fn placements(deck: &Deck) -> Vec<Vec<(bool, bool)>> {
    deck.slides
        .iter()
        .map(|s| {
            let elements = s.params.slot("elements").unwrap_or_default();
            elements.iter().map(|e| (e.cell.is_some(), e.frame.is_some())).collect()
        })
        .collect()
}

#[test]
fn retrofit_kitchen_on_acme_grid_places_on_cells_unless_they_collide() {
    let (plan, deck) = import_with_extra_pack("kitchen", &["acme"], "pro", acme_grid_pack_of(3, 2));
    // Slide 2 : graphique, image et pile se recouvriraient sur la grille 3 × 2 : cadres.
    assert_that!(placements(&deck)).is_equal_to(vec![
        vec![(true, false)],
        vec![(false, true); 3],
        vec![(true, false); 2],
    ]);
    assert_that!(plan.report.slides.iter().all(|s| s.profile == "acme/grille")).is_true();
}

#[test]
fn retrofit_without_profiles_is_refused() {
    let err = import_err("minimal", &["acme-empty"], empty_pack("acme-empty"));
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::NoProfiles);
}
