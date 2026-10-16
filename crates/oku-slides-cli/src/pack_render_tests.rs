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
