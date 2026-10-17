use oku_slides_import::metrics::{RegionScore, WitnessLimit};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

const LIMITS: TuneLimits = TuneLimits { tries: 3, stall: 2, global: 2 };
const THRESHOLD: f64 = 7.0;
const SLIDE: &str = "s1";

/// Une slide notée : ses régions `(score, suspecte)`, déjà classées.
fn scored(regions: &[(f64, bool)]) -> SlideScore {
    let regions = regions
        .iter()
        .enumerate()
        .map(|(i, &(score, suspect))| RegionScore {
            item: format!("s1/{i}"),
            score,
            shift: [0, 0],
            suspect: suspect.then_some(WitnessLimit::Chart),
        })
        .collect();
    SlideScore { score: 1.0, regions }
}

/// Une slide notée d'une seule région non suspecte, de score `worst`.
fn worst(worst: f64) -> SlideScore {
    scored(&[(worst, false)])
}

fn tuning(tries: u32, without_gain: u32) -> Tuning {
    Tuning {
        tries,
        best: Some(2.0),
        without_gain,
        seen: Some(seen(1, 1)),
        first: Some(1.0),
        imported: Some(1),
        best_raster: Some(1),
    }
}

/// Ce qu'a vu une note : le raster `raster`, l'état du deck `deck`, le modèle de la slide inchangé.
fn seen(raster: u128, deck: u128) -> Seen {
    Seen { raster, deck, slide: 1 }
}

/// Ce qu'a vu une note : le raster `raster`, l'état du deck `deck`, le modèle de la slide `model`.
fn edited(raster: u128, deck: u128, model: u128) -> Seen {
    Seen { raster, deck, slide: model }
}

/// Note `score` sur le raster `now` de `slide`, une slide `pending` avant la note : la charge.
fn observe(t: &mut Tuner, slide: &str, now: Seen, score: &SlideScore) -> Charge {
    let charge = t.charge(slide, now, SlideState::Pending);
    t.record(slide, now, charge, score, THRESHOLD);
    charge
}

/// Un tuner dont la slide [`SLIDE`] a sa base : raster 1, deck 1, pire région `base`.
fn based(base: f64) -> Tuner {
    let mut t = Tuner::new(LIMITS);
    observe(&mut t, SLIDE, seen(1, 1), &worst(base));
    t
}

#[rstest]
#[case::one_region_above(&[(9.0, false)], Some(2.0))]
#[case::every_region_above_counts(&[(9.0, false), (8.0, false), (3.0, false)], Some(3.0))]
#[case::suspects_never_count(&[(9.0, false), (20.0, true)], Some(2.0))]
#[case::all_under_is_zero(&[(6.0, false)], Some(0.0))]
#[case::only_suspects_is_none(&[(20.0, true)], None)]
#[case::no_region_is_none(&[], None)]
fn excess_sums_the_non_suspect_regions_above_the_threshold(
    #[case] regions: &[(f64, bool)],
    #[case] expected: Option<f64>,
) {
    assert_that!(excess(&scored(regions), THRESHOLD)).is_equal_to(expected);
}

#[rstest]
#[case::above_with_tries_left(&[(9.0, false)], None, SlideState::Pending)]
#[case::under_the_threshold(&[(6.0, false), (3.0, true)], None, SlideState::Done)]
#[case::only_a_suspect_above(&[(6.0, false), (12.0, true)], None, SlideState::Witness)]
#[case::only_suspects_all_above(&[(12.0, true)], None, SlideState::Witness)]
#[case::no_region(&[], None, SlideState::Done)]
#[case::every_try_used(&[(9.0, false)], Some(tuning(3, 0)), SlideState::Exhausted)]
#[case::two_tries_without_gain(&[(9.0, false)], Some(tuning(2, 2)), SlideState::Stalled)]
#[case::stalled_wins_over_exhausted(&[(9.0, false)], Some(tuning(3, 2)), SlideState::Stalled)]
#[case::done_after_every_try(&[(5.0, false)], Some(tuning(3, 2)), SlideState::Done)]
fn state_of_a_scored_slide_follows_its_excess_and_tries(
    #[case] regions: &[(f64, bool)],
    #[case] tries: Option<Tuning>,
    #[case] expected: SlideState,
) {
    let score = scored(regions);
    assert_that!(SlideState::of(Some(&score), tries.as_ref(), THRESHOLD, LIMITS)).is_equal_to(expected);
}

#[rstest]
#[case::never_scored_by_a_tool(None, SlideState::Witness)]
#[case::back_as_imported(Some(1), SlideState::Witness)]
#[case::changed_since_the_import(Some(2), SlideState::Done)]
fn state_with_only_a_suspect_above_is_witness_only_while_as_imported(
    #[case] raster: Option<u128>,
    #[case] expected: SlideState,
) {
    let tries = raster.map(|r| Tuning { seen: Some(seen(r, 1)), ..tuning(1, 0) });
    let score = scored(&[(6.0, false), (12.0, true)]);
    assert_that!(SlideState::of(Some(&score), tries.as_ref(), THRESHOLD, LIMITS)).is_equal_to(expected);
}

#[test]
fn state_without_a_score_is_unscored() {
    assert_that!(SlideState::of(None, None, THRESHOLD, LIMITS)).is_equal_to(SlideState::Unscored);
}

#[rstest]
#[case::fresh(SlideState::Pending, None, 3)]
#[case::one_try(SlideState::Pending, Some(tuning(1, 0)), 2)]
#[case::stalled(SlideState::Stalled, Some(tuning(2, 2)), 0)]
#[case::exhausted(SlideState::Exhausted, Some(tuning(3, 1)), 0)]
#[case::done(SlideState::Done, None, 0)]
#[case::witness(SlideState::Witness, None, 0)]
#[case::unscored(SlideState::Unscored, None, 0)]
fn tries_left_counts_down_while_pending_and_is_zero_otherwise(
    #[case] state: SlideState,
    #[case] tries: Option<Tuning>,
    #[case] expected: u32,
) {
    assert_that!(state.tries_left(tries.as_ref(), LIMITS)).is_equal_to(expected);
}

#[test]
fn observe_first_scoring_is_the_free_baseline() {
    let mut t = Tuner::new(LIMITS);
    let charge = observe(&mut t, SLIDE, seen(1, 1), &worst(9.0));
    assert_that!(charge).is_equal_to(Charge::Baseline);
    assert_that!(t.tuning(SLIDE).cloned()).is_equal_to(Some(Tuning {
        tries: 0,
        best: Some(2.0),
        without_gain: 0,
        seen: Some(seen(1, 1)),
        first: Some(1.0),
        imported: Some(1),
        best_raster: Some(1),
    }));
    assert_that!(t.global_tries_left()).is_equal_to(2);
}

#[test]
fn observe_same_raster_is_not_a_try() {
    let mut t = based(9.0);
    assert_that!(observe(&mut t, SLIDE, seen(1, 1), &worst(9.0))).is_equal_to(Charge::Unchanged);
    assert_that!(t.tuning(SLIDE).map(|t| (t.tries, t.without_gain))).is_equal_to(Some((0, 0)));
}

#[test]
fn observe_changed_raster_with_a_lower_excess_is_a_try_that_resets_the_streak() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 1), &worst(9.5));
    assert_that!(observe(&mut t, SLIDE, seen(3, 1), &worst(8.0))).is_equal_to(Charge::Slide);
    let tuning = t.tuning(SLIDE).cloned().unwrap();
    assert_that!((tuning.tries, tuning.best, tuning.without_gain)).is_equal_to((2, Some(1.0), 0));
}

#[test]
fn observe_gain_is_on_the_total_excess_not_the_worst_region() {
    let mut t = Tuner::new(LIMITS);
    observe(&mut t, SLIDE, seen(1, 1), &scored(&[(9.0, false), (9.0, false)]));
    // La pire région monte, mais l'excès total baisse (4 → 3) : c'est un gain.
    observe(&mut t, SLIDE, seen(2, 1), &scored(&[(10.0, false), (6.0, false)]));
    let tuning = t.tuning(SLIDE).cloned().unwrap();
    assert_that!((tuning.best, tuning.without_gain)).is_equal_to((Some(3.0), 0));
}

#[test]
fn observe_changed_raster_without_gain_extends_the_streak() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 1), &worst(9.0));
    observe(&mut t, SLIDE, seen(3, 1), &worst(9.4));
    let tuning = t.tuning(SLIDE).cloned().unwrap();
    assert_that!((tuning.tries, tuning.best, tuning.without_gain)).is_equal_to((2, Some(2.0), 2));
}

#[test]
fn charge_of_a_changed_raster_without_tries_left_is_refused_and_changes_nothing() {
    let mut t = based(9.0);
    let before = t.tuning(SLIDE).cloned();
    let score = worst(9.0);
    let charge = t.charge(SLIDE, seen(2, 1), SlideState::Done);
    assert_that!(charge).is_equal_to(Charge::Refused);
    t.record(SLIDE, seen(2, 1), charge, &score, THRESHOLD);
    assert_that!(t.tuning(SLIDE).cloned()).is_equal_to(before);
}

#[test]
fn observe_deck_wide_change_draws_one_global_try_and_no_slide_try() {
    let mut t = based(9.0);
    observe(&mut t, "s2", seen(10, 1), &worst(9.0));
    assert_that!(observe(&mut t, SLIDE, seen(2, 2), &worst(9.5))).is_equal_to(Charge::Global);
    assert_that!(observe(&mut t, "s2", seen(11, 2), &worst(9.5))).is_equal_to(Charge::Global);
    assert_that!(t.global_tries_left()).is_equal_to(1);
    assert_that!(t.tuning(SLIDE).map(|t| (t.tries, t.without_gain, t.best))).is_equal_to(Some((0, 0, Some(2.0))));
    assert_that!(t.tuning("s2").map(|t| t.tries)).is_equal_to(Some(0));
}

#[test]
fn observe_back_to_a_deck_state_already_scored_is_free() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 2), &worst(9.5));
    assert_that!(observe(&mut t, SLIDE, seen(3, 1), &worst(9.0))).is_equal_to(Charge::Global);
    assert_that!(t.global_tries_left()).is_equal_to(1);
}

#[test]
fn observe_deck_wide_change_once_the_global_budget_is_spent_costs_slide_tries() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 2), &worst(9.0));
    observe(&mut t, SLIDE, seen(3, 3), &worst(9.0));
    assert_that!(t.global_tries_left()).is_equal_to(0);
    assert_that!(observe(&mut t, SLIDE, seen(4, 4), &worst(8.0))).is_equal_to(Charge::Slide);
    assert_that!(t.tuning(SLIDE).map(|t| t.tries)).is_equal_to(Some(1));
}

#[test]
fn observe_deck_change_with_a_slide_edit_costs_a_slide_try() {
    let mut t = based(9.0);
    assert_that!(observe(&mut t, SLIDE, edited(2, 2, 2), &worst(9.0))).is_equal_to(Charge::Slide);
    assert_that!(t.tuning(SLIDE).map(|t| t.tries)).is_equal_to(Some(1));
    assert_that!(t.global_tries_left()).is_equal_to(2);
}

#[test]
fn observe_toggling_between_paid_deck_states_with_slide_edits_costs_slide_tries_until_stalled() {
    let mut t = based(9.0);
    // Un changement du deck entier paie l'état 2 ; la slide n'a pas changé.
    assert_that!(observe(&mut t, SLIDE, seen(2, 2), &worst(9.0))).is_equal_to(Charge::Global);
    // Retouche de la slide, retour à l'état 1 (déjà payé), puis de nouveau, retour à l'état 2.
    assert_that!(observe(&mut t, SLIDE, edited(3, 1, 2), &worst(9.0))).is_equal_to(Charge::Slide);
    assert_that!(observe(&mut t, SLIDE, edited(4, 2, 3), &worst(9.0))).is_equal_to(Charge::Slide);
    let tuning = t.tuning(SLIDE).cloned().unwrap();
    assert_that!((tuning.tries, tuning.without_gain)).is_equal_to((2, 2));
    assert_that!(SlideState::of(Some(&worst(9.0)), Some(&tuning), THRESHOLD, LIMITS)).is_equal_to(SlideState::Stalled);
    assert_that!(t.global_tries_left()).is_equal_to(1);
}

#[test]
fn observe_back_to_a_paid_deck_state_with_the_slide_unchanged_since_is_free() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, edited(2, 1, 2), &worst(9.0));
    observe(&mut t, SLIDE, edited(3, 2, 2), &worst(9.0));
    assert_that!(observe(&mut t, SLIDE, edited(2, 1, 2), &worst(9.0))).is_equal_to(Charge::Global);
    assert_that!(t.tuning(SLIDE).map(|t| t.tries)).is_equal_to(Some(1));
}

#[test]
fn touch_moves_the_slide_model_of_an_unchanged_page_on() {
    let mut t = based(9.0);
    // Une retouche de la slide sans effet sur sa page, puis un changement du deck entier seul.
    t.touch(SLIDE, 1, 2);
    assert_that!(observe(&mut t, SLIDE, edited(2, 2, 2), &worst(9.0))).is_equal_to(Charge::Global);
}

#[test]
fn touch_moves_the_deck_state_of_an_unchanged_page_on() {
    let mut t = based(9.0);
    observe(&mut t, "s2", seen(10, 1), &worst(9.0));
    // Un changement du deck entier qui ne change pas la page de SLIDE, puis une retouche de SLIDE seule.
    observe(&mut t, "s2", seen(11, 2), &worst(9.0));
    t.touch(SLIDE, 2, 1);
    assert_that!(observe(&mut t, SLIDE, seen(2, 2), &worst(8.0))).is_equal_to(Charge::Slide);
}

#[test]
fn worse_compares_the_current_excess_with_the_best_seen() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 1), &worst(8.0));
    assert_that!(t.worse(SLIDE, &worst(8.0), THRESHOLD)).is_false();
    assert_that!(t.worse(SLIDE, &worst(9.0), THRESHOLD)).is_true();
    assert_that!(t.worse("s9", &worst(9.0), THRESHOLD)).is_false();
}

#[test]
fn mean_before_is_the_mean_of_the_first_scores() {
    let mut t = Tuner::new(LIMITS);
    observe(&mut t, "s1", seen(1, 1), &SlideScore { score: 2.0, regions: Vec::new() });
    observe(&mut t, "s2", seen(2, 1), &SlideScore { score: 4.0, regions: Vec::new() });
    observe(&mut t, "s1", seen(3, 1), &SlideScore { score: 9.0, regions: Vec::new() });
    assert_that!(t.mean_before(["s1", "s2"])).is_equal_to(Some(3.0));
    assert_that!(Tuner::new(LIMITS).mean_before(["s1"])).is_none();
}

#[test]
fn observe_back_to_the_best_raster_is_free() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 1), &worst(9.5));
    assert_that!(observe(&mut t, SLIDE, seen(1, 1), &worst(9.0))).is_equal_to(Charge::Revert);
    assert_that!(t.tuning(SLIDE).map(|t| (t.tries, t.without_gain))).is_equal_to(Some((1, 1)));
}

#[test]
fn observe_best_raster_follows_the_last_gain() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 1), &worst(8.0));
    observe(&mut t, SLIDE, seen(3, 1), &worst(8.5));
    assert_that!(observe(&mut t, SLIDE, seen(2, 1), &worst(8.0))).is_equal_to(Charge::Revert);
    assert_that!(observe(&mut t, SLIDE, seen(1, 1), &worst(9.0))).is_equal_to(Charge::Slide);
}

#[test]
fn charge_back_to_the_best_raster_without_tries_left_is_free() {
    let mut t = based(9.0);
    observe(&mut t, SLIDE, seen(2, 1), &worst(9.5));
    assert_that!(t.charge(SLIDE, seen(1, 1), SlideState::Stalled)).is_equal_to(Charge::Revert);
}

#[test]
fn mean_before_is_rounded_like_the_mean() {
    let mut t = Tuner::new(LIMITS);
    observe(&mut t, "s1", seen(1, 1), &SlideScore { score: 0.9, regions: Vec::new() });
    observe(&mut t, "s2", seen(2, 1), &SlideScore { score: 0.915, regions: Vec::new() });
    assert_that!(t.mean_before(["s1", "s2"])).is_equal_to(Some(0.91));
}
