// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adversarial soundness tests for hybrid reachability enclosures (TC-040).
//!
//! Every test attacks one FR-040 acceptance criterion and is named after it.
//! The soundness oracle is an independent enumeration of concrete executions
//! over explicit integer states, computed in `i128` so it cannot overflow.
//!
//! FR-041-AC-1 "a reset that leaves an invariant" is not applicable: the request
//! type has no invariant (FR-040-AC-7), so no such request can be built.
//!
//! A test that asserts the sound behaviour and currently fails is marked
//! `#[ignore = "VER-94 finding Fn: ..."]`. Run them with
//! `cargo test --test hybrid_enclosure_soundness -- --ignored`.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use proptest::prelude::*;
use quire_analyze::{
    hybrid_method_outcome, reach, HybridIncompleteCode, HybridOutcome, HybridRefusalCode,
    HybridRequest, HybridTransition, Interval, ModeEnclosure,
};
use quire_analyze_method_contract::{AnalyzeMethod, AnalyzeOutcome};

type States = BTreeMap<String, BTreeSet<i128>>;

/// Which states an enclosure is judged against.
#[derive(Clone, Copy)]
enum Claim {
    /// Only states at the horizon step.
    Horizon,
    /// Every state at every step up to and including the horizon.
    EveryStep,
}

/// Enumerates concrete executions: per step each state flows by any integer
/// delta in range, then either stays in its mode or takes any enabled transition.
fn oracle(request: &HybridRequest) -> Vec<States> {
    let mut steps = vec![BTreeMap::from([(
        request.initial_mode.0.clone(),
        (i128::from(request.initial_set.lower)..=i128::from(request.initial_set.upper)).collect(),
    )])];
    for _ in 0..request.horizon {
        let mut next: States = BTreeMap::new();
        for (mode, states) in steps.last().expect("step zero exists") {
            for &x in states {
                for delta in
                    i128::from(request.flow_delta_lower)..=i128::from(request.flow_delta_upper)
                {
                    let flowed = x + delta;
                    next.entry(mode.clone()).or_default().insert(flowed);
                    for t in request
                        .transitions
                        .iter()
                        .filter(|t| t.from_mode.0 == *mode)
                    {
                        if (i128::from(t.guard.lower)..=i128::from(t.guard.upper)).contains(&flowed)
                        {
                            next.entry(t.to_mode.0.clone())
                                .or_default()
                                .insert(flowed + i128::from(t.reset_offset));
                        }
                    }
                }
            }
        }
        steps.push(next);
    }
    steps
}

/// Runs a request twice and requires equal outcomes (NFR-040-AC-3).
fn run(request: &HybridRequest) -> HybridOutcome {
    let first = reach(request);
    assert_eq!(
        first,
        reach(request),
        "two evaluations of one request differ"
    );
    first
}

fn enclosures(outcome: &HybridOutcome) -> &[ModeEnclosure] {
    match outcome {
        HybridOutcome::Enclosure { enclosures, .. } => enclosures,
        other => panic!("expected an enclosure, got {other:?}"),
    }
}

fn mode_interval(outcome: &HybridOutcome, mode: &str) -> Option<Interval> {
    enclosures(outcome)
        .iter()
        .find(|e| e.mode.0 == mode)
        .map(|e| e.interval)
}

/// True when an exact reachable state or the exact widened hull leaves `i64`.
fn exceeds_i64(request: &HybridRequest, steps: &[States], claim: Claim) -> bool {
    let judged = match claim {
        Claim::Horizon => &steps[steps.len() - 1..],
        Claim::EveryStep => steps,
    };
    let error = i128::from(request.error_bound);
    let range = i128::from(i64::MIN)..=i128::from(i64::MAX);
    judged.iter().flat_map(|step| step.values()).any(|states| {
        let (Some(low), Some(high)) = (states.first(), states.last()) else {
            return false;
        };
        !range.contains(&(low - error)) || !range.contains(&(high + error))
    })
}

/// Soundness check: an enclosure must contain every enumerated state in the
/// judged modes; a state beyond `i64` forbids an enclosure; `Incomplete` and
/// `Refused` are always sound.
fn assert_sound(request: &HybridRequest, claim: Claim, only: Option<&[&str]>) {
    let steps = oracle(request);
    let outcome = run(request);
    let HybridOutcome::Enclosure { enclosures, .. } = &outcome else {
        return;
    };
    assert!(
        !exceeds_i64(request, &steps, claim),
        "enclosure returned for a bound outside the 64-bit range: {enclosures:?}"
    );
    let judged = match claim {
        Claim::Horizon => &steps[steps.len() - 1..],
        Claim::EveryStep => &steps[..],
    };
    let error = i128::from(request.error_bound);
    for step in judged {
        for (mode, states) in step {
            if only.is_some_and(|modes| !modes.contains(&mode.as_str())) {
                continue;
            }
            let Some(found) = enclosures.iter().find(|e| e.mode.0 == *mode) else {
                panic!("reachable mode {mode} has no enclosure: {enclosures:?}");
            };
            for &state in states {
                assert!(
                    i128::from(found.interval.lower) <= state
                        && state <= i128::from(found.interval.upper),
                    "mode {mode} omits reachable state {state}; enclosure {:?}, error {error}",
                    found.interval
                );
            }
        }
    }
}

/// Hull of the enumerated horizon states of one mode, widened by the error bound.
fn exact_hull(request: &HybridRequest, mode: &str) -> Interval {
    let steps = oracle(request);
    let states = &steps[steps.len() - 1][mode];
    let error = i128::from(request.error_bound);
    Interval {
        lower: i64::try_from(states.first().expect("reached") - error).expect("in range"),
        upper: i64::try_from(states.last().expect("reached") + error).expect("in range"),
    }
}

fn request(mode: &str, lower: i64, upper: i64, horizon: u64, delta: (i64, i64)) -> HybridRequest {
    HybridRequest {
        initial_mode: mode.into(),
        transitions: vec![],
        initial_set: Interval { lower, upper },
        horizon,
        flow_delta_lower: delta.0,
        flow_delta_upper: delta.1,
        error_bound: 0,
        step_bound: horizon,
        convergence_steps: 0,
    }
}

fn transition(from: &str, to: &str, guard: (i64, i64), reset_offset: i64) -> HybridTransition {
    HybridTransition {
        from_mode: from.into(),
        to_mode: to.into(),
        guard: Interval {
            lower: guard.0,
            upper: guard.1,
        },
        reset_offset,
    }
}

fn with_transitions(mut base: HybridRequest, transitions: Vec<HybridTransition>) -> HybridRequest {
    base.transitions = transitions;
    base
}

fn assert_not_enclosure(request: &HybridRequest) {
    let outcome = run(request);
    assert!(
        !matches!(outcome, HybridOutcome::Enclosure { .. }),
        "expected Incomplete or Refused, got {outcome:?}"
    );
}

// ---------------------------------------------------------------- FR-040-AC-1

#[trace("TC-040", "FR-040-AC-1")]
#[test]
fn fr040_ac1_in_range_bounds_are_exact_including_one_unit_from_limit() {
    let full = request("m", i64::MIN + 1, i64::MAX - 1, 1, (-1, 1));
    assert_eq!(
        mode_interval(&run(&full), "m"),
        Some(Interval {
            lower: i64::MIN,
            upper: i64::MAX
        })
    );
    let top = request("m", i64::MAX - 2, i64::MAX - 1, 1, (1, 1));
    assert_eq!(
        mode_interval(&run(&top), "m"),
        Some(Interval {
            lower: i64::MAX - 1,
            upper: i64::MAX
        })
    );
    let mut widened = request("m", 0, 0, 1, (0, 0));
    widened.error_bound = 3;
    assert_eq!(
        mode_interval(&run(&widened), "m"),
        Some(Interval {
            lower: -3,
            upper: 3
        })
    );
    let reset = with_transitions(
        request("m", 0, 2, 1, (0, 0)),
        vec![transition("m", "t", (0, 2), 5)],
    );
    assert_eq!(
        mode_interval(&run(&reset), "t"),
        Some(Interval { lower: 5, upper: 7 })
    );
}

proptest! {
    #[trace("TC-040", "FR-040-AC-1")]
    #[test]
    fn fr040_ac1_small_single_mode_models_match_enumeration_exactly(
        lower in -20i64..20,
        width in 0i64..6,
        dl in -3i64..3,
        dw in 0i64..3,
        horizon in 0u64..5,
        error in 0u64..3,
    ) {
        let mut req = request("m", lower, lower + width, horizon, (dl, dl + dw));
        req.error_bound = error;
        let outcome = run(&req);
        prop_assert_eq!(mode_interval(&outcome, "m"), Some(exact_hull(&req, "m")));
    }
}

// ---------------------------------------------------------------- FR-040-AC-2

#[trace("TC-040", "FR-040-AC-2")]
#[test]
#[ignore = "VER-94 finding F1: flow step saturates at i64::MIN instead of ending Incomplete"]
fn fr040_ac2_flow_underflow_one_unit_past_limit_is_not_clamped() {
    assert_not_enclosure(&request("m", i64::MIN, i64::MIN, 1, (-1, 0)));
}

#[trace("TC-040", "FR-040-AC-2")]
#[test]
#[ignore = "VER-94 finding F1: flow step saturates at i64::MAX instead of ending Incomplete"]
fn fr040_ac2_flow_overflow_one_unit_past_limit_is_not_clamped() {
    assert_not_enclosure(&request("m", i64::MAX - 1, i64::MAX - 1, 1, (0, 2)));
}

#[trace("TC-040", "FR-040-AC-2")]
#[test]
#[ignore = "VER-94 finding F2: reset saturates at i64::MAX instead of ending Incomplete"]
fn fr040_ac2_reset_overflow_is_not_clamped() {
    let req = with_transitions(
        request("m", i64::MAX - 1, i64::MAX, 1, (0, 0)),
        vec![transition("m", "t", (i64::MAX - 1, i64::MAX), 1)],
    );
    assert_not_enclosure(&req);
}

#[trace("TC-040", "FR-040-AC-2")]
#[test]
#[ignore = "VER-94 finding F3: final widening saturates instead of ending Incomplete"]
fn fr040_ac2_widening_overflow_is_not_clamped() {
    let mut req = request("m", i64::MAX, i64::MAX, 0, (0, 0));
    req.error_bound = 1;
    assert_not_enclosure(&req);
    let mut low = request("m", i64::MIN, i64::MIN, 0, (0, 0));
    low.error_bound = 1;
    assert_not_enclosure(&low);
}

proptest! {
    #[trace("TC-040", "FR-040-AC-2")]
    #[test]
    #[ignore = "VER-94 findings F1, F2, F3: saturating arithmetic near the 64-bit limits"]
    fn fr040_ac2_random_near_limit_models_never_clamp(
        top in any::<bool>(),
        offset in 0i64..3,
        width in 0i64..2,
        dl in -2i64..2,
        dw in 0i64..3,
        reset in -2i64..3,
        error in 0u64..3,
        horizon in 0u64..3,
    ) {
        let base = if top { i64::MAX - offset - width } else { i64::MIN + offset };
        let mut req = with_transitions(
            request("m", base, base + width, horizon, (dl, dl + dw)),
            vec![transition("m", "t", (i64::MIN, i64::MAX), reset)],
        );
        req.error_bound = error;
        assert_sound(&req, Claim::Horizon, Some(&["t"]));
    }
}

// ---------------------------------------------------------------- FR-040-AC-3

#[trace("TC-040", "FR-040-AC-3")]
#[test]
fn fr040_ac3_guard_touching_flowed_interval_at_one_point_is_enabled() {
    // Flowed interval is [1, 3]; the guard touches it at 3 only.
    let upper_touch = with_transitions(
        request("m", 0, 2, 1, (1, 1)),
        vec![transition("m", "t", (3, 5), 0)],
    );
    assert_sound(&upper_touch, Claim::Horizon, Some(&["t"]));
    let found = mode_interval(&run(&upper_touch), "t").expect("touching guard enables transition");
    assert!(found.lower <= 3 && 3 <= found.upper);
    // The guard touches [1, 3] at 1 only.
    let lower_touch = with_transitions(
        request("m", 0, 2, 1, (1, 1)),
        vec![transition("m", "t", (-5, 1), 0)],
    );
    assert_sound(&lower_touch, Claim::Horizon, Some(&["t"]));
    let found = mode_interval(&run(&lower_touch), "t").expect("touching guard enables transition");
    assert!(found.lower <= 1 && 1 <= found.upper);
}

// ---------------------------------------------------------------- FR-040-AC-4

#[trace("TC-040", "FR-040-AC-4")]
#[test]
fn fr040_ac4_target_receives_reset_image_of_guard_part() {
    let req = with_transitions(
        request("m", 0, 10, 1, (0, 0)),
        vec![transition("m", "t", (8, 20), 100)],
    );
    assert_sound(&req, Claim::Horizon, Some(&["t"]));
}

#[trace("TC-040", "FR-040-AC-4")]
#[test]
#[ignore = "VER-94 finding F4: source mode is dropped once any transition is enabled"]
fn fr040_ac4_source_keeps_whole_flowed_interval_when_guard_overlaps() {
    let partial = with_transitions(
        request("m", 0, 10, 1, (0, 0)),
        vec![transition("m", "t", (8, 20), 100)],
    );
    assert_sound(&partial, Claim::Horizon, Some(&["m"]));
    let covering = with_transitions(
        request("m", 0, 10, 1, (0, 0)),
        vec![transition("m", "t", (-5, 20), 100)],
    );
    assert_sound(&covering, Claim::Horizon, Some(&["m"]));
}

// ---------------------------------------------------------------- FR-040-AC-5

#[trace("TC-040", "FR-040-AC-5")]
#[test]
fn fr040_ac5_two_transitions_into_one_target_join_every_contribution() {
    let req = with_transitions(
        request("m", 0, 5, 1, (0, 0)),
        vec![
            transition("m", "t", (0, 20), 0),
            transition("m", "t", (0, 20), 100),
            transition("m", "u", (0, 20), -50),
        ],
    );
    assert_sound(&req, Claim::Horizon, Some(&["t", "u"]));
    let outcome = run(&req);
    assert_eq!(mode_interval(&outcome, "t"), Some(exact_hull(&req, "t")));
    assert_eq!(mode_interval(&outcome, "u"), Some(exact_hull(&req, "u")));
}

#[trace("TC-040", "FR-040-AC-5")]
#[test]
fn fr040_ac5_repeated_and_reordered_transitions_give_the_same_enclosure() {
    let a = transition("m", "t", (0, 20), 7);
    let b = transition("m", "t", (0, 20), -3);
    let once = with_transitions(request("m", 0, 5, 2, (0, 1)), vec![a.clone(), b.clone()]);
    let repeated = with_transitions(
        request("m", 0, 5, 2, (0, 1)),
        vec![a.clone(), a.clone(), b.clone(), b.clone(), a.clone()],
    );
    let reordered = with_transitions(request("m", 0, 5, 2, (0, 1)), vec![b, a]);
    assert_eq!(enclosures(&run(&once)), enclosures(&run(&repeated)));
    assert_eq!(enclosures(&run(&once)), enclosures(&run(&reordered)));
}

// ---------------------------------------------------------------- FR-040-AC-6

#[trace("TC-040", "FR-040-AC-6")]
#[test]
fn fr040_ac6_reset_image_never_omits_a_reachable_state() {
    let req = with_transitions(
        request("m", 0, 10, 1, (0, 1)),
        vec![transition("m", "t", (8, 20), 100)],
    );
    assert_sound(&req, Claim::Horizon, Some(&["t"]));
}

#[trace("TC-040", "FR-040-AC-6")]
#[test]
#[ignore = "VER-94 finding F5 (precision): reset is applied to the whole flowed interval, not the guard part"]
fn fr040_ac6_reset_maps_only_the_guard_satisfying_part() {
    let req = with_transitions(
        request("m", 0, 10, 1, (0, 0)),
        vec![transition("m", "t", (8, 20), 100)],
    );
    // Exact image of the guard part [8, 10] is [108, 110].
    assert_eq!(mode_interval(&run(&req), "t"), Some(exact_hull(&req, "t")));
}

// ---------------------------------------------------------------- FR-040-AC-7

#[trace("TC-040", "FR-040-AC-7")]
#[test]
fn fr040_ac7_request_type_carries_no_invariant() {
    // Exhaustive struct literals (no `..`): the request has no invariant field, so
    // an invariant cannot reach the provider. Adding a field breaks this test and
    // forces the FR-040-AC-7 invariant behaviour to be tested at that point.
    let _ = HybridRequest {
        initial_mode: "m".into(),
        transitions: vec![HybridTransition {
            from_mode: "m".into(),
            to_mode: "t".into(),
            guard: Interval { lower: 0, upper: 0 },
            reset_offset: 0,
        }],
        initial_set: Interval { lower: 0, upper: 0 },
        horizon: 0,
        flow_delta_lower: 0,
        flow_delta_upper: 0,
        error_bound: 0,
        step_bound: 0,
        convergence_steps: 0,
    };
}

// ---------------------------------------------------------------- FR-040-AC-8

#[trace("TC-040", "FR-040-AC-8")]
#[test]
#[ignore = "VER-94 finding F7: result names no claim; enclosure covers only the horizon step"]
fn fr040_ac8_unnamed_claim_must_cover_every_step() {
    // Flow is +1 per step; steps 0, 1, 2 reach 0, 1, 2 but the horizon alone reaches 2.
    assert_sound(&request("m", 0, 0, 2, (1, 1)), Claim::EveryStep, None);
}

// ---------------------------------------------------------------- FR-040-AC-9

#[trace("TC-040", "FR-040-AC-9")]
#[test]
fn fr040_ac9_horizon_above_step_bound_is_incomplete_and_equal_is_admitted() {
    let mut req = request("m", 0, 0, 3, (1, 1));
    req.step_bound = 2;
    assert!(matches!(
        run(&req),
        HybridOutcome::Incomplete {
            code: HybridIncompleteCode::StepBoundExhausted,
            ..
        }
    ));
    req.step_bound = 3;
    assert!(matches!(run(&req), HybridOutcome::Enclosure { .. }));
}

#[trace("TC-040", "FR-040-AC-9")]
#[test]
fn fr040_ac9_caller_reported_count_above_bound_is_incomplete() {
    let mut req = request("m", 0, 0, 1, (1, 1));
    req.step_bound = 2;
    req.convergence_steps = 3;
    assert!(matches!(
        run(&req),
        HybridOutcome::Incomplete {
            code: HybridIncompleteCode::StepBoundExhausted,
            ..
        }
    ));
}

#[trace("TC-040", "FR-040-AC-9")]
#[test]
fn fr040_ac9_exhausted_horizon_never_returns_an_enclosure() {
    for step_bound in 0..4 {
        let mut req = with_transitions(
            request("m", 0, 1, 4, (0, 1)),
            vec![transition("m", "t", (0, 9), 1)],
        );
        req.step_bound = step_bound;
        assert!(matches!(run(&req), HybridOutcome::Incomplete { .. }));
    }
}

// --------------------------------------------------------------- FR-040-AC-10

#[trace("TC-040", "FR-040-AC-10")]
#[test]
fn fr040_ac10_error_bound_widens_both_ends_and_is_reported_as_declared() {
    let mut req = request("m", 2, 4, 1, (0, 0));
    req.error_bound = 5;
    let outcome = run(&req);
    assert_eq!(
        mode_interval(&outcome, "m"),
        Some(Interval {
            lower: -3,
            upper: 9
        })
    );
    let HybridOutcome::Enclosure {
        error_bound,
        witness,
        ..
    } = outcome
    else {
        unreachable!()
    };
    assert_eq!(error_bound, 5);
    assert_eq!(
        witness.enclosures[0].interval,
        Interval {
            lower: -3,
            upper: 9
        }
    );
}

#[trace("TC-040", "FR-040-AC-10")]
#[test]
#[ignore = "VER-94 finding F8: error_bound above i64::MAX is silently truncated to i64::MAX"]
fn fr040_ac10_error_bound_above_i64_max_is_not_truncated() {
    let mut req = request("m", 0, 0, 0, (0, 0));
    req.error_bound = u64::MAX;
    // Exact outward widening is [-(2^64 - 1), 2^64 - 1], not representable.
    assert_not_enclosure(&req);
}

// --------------------------------------------------------------- FR-040-AC-11

#[trace("TC-040", "FR-040-AC-11")]
#[test]
fn fr040_ac11_malformed_inputs_are_refused() {
    let good = || {
        with_transitions(
            request("m", 0, 1, 1, (0, 1)),
            vec![transition("m", "t", (0, 1), 0)],
        )
    };
    let mut inverted_initial = good();
    inverted_initial.initial_set = Interval { lower: 2, upper: 1 };
    let mut empty_initial = good();
    empty_initial.initial_mode = "".into();
    let mut empty_from = good();
    empty_from.transitions[0].from_mode = "".into();
    let mut empty_to = good();
    empty_to.transitions[0].to_mode = "".into();
    let mut inverted_flow = good();
    inverted_flow.flow_delta_lower = 2;
    inverted_flow.flow_delta_upper = 1;
    let mut inverted_guard = good();
    inverted_guard.transitions[0].guard = Interval { lower: 2, upper: 1 };
    for malformed in [
        inverted_initial,
        empty_initial,
        empty_from,
        empty_to,
        inverted_flow,
        inverted_guard,
    ] {
        let outcome = run(&malformed);
        assert!(
            matches!(
                outcome,
                HybridOutcome::Refused {
                    code: HybridRefusalCode::MalformedModel,
                    ..
                }
            ),
            "{malformed:?} gave {outcome:?}"
        );
    }
}

#[trace("TC-040", "FR-040-AC-11")]
#[test]
fn fr040_ac11_unreached_mode_endpoint_is_well_formed() {
    let req = with_transitions(
        request("m", 0, 1, 1, (0, 0)),
        vec![transition("ghost", "elsewhere", (0, 1), 0)],
    );
    assert_eq!(
        mode_interval(&run(&req), "m"),
        Some(Interval { lower: 0, upper: 1 })
    );
}

// --------------------------------------------------------------- FR-040-AC-12

#[trace("TC-040", "FR-040-AC-12")]
#[test]
fn fr040_ac12_extreme_requests_end_in_exactly_one_typed_outcome_without_panic() {
    let extremes = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
    for &bound in &extremes {
        for &delta in &extremes {
            for error in [0, 1, u64::MAX] {
                let mut req = with_transitions(
                    request("m", bound, bound, 2, (delta.min(0), delta.max(0))),
                    vec![transition("m", "t", (i64::MIN, i64::MAX), delta)],
                );
                req.error_bound = error;
                let outcome = run(&req);
                let (method, terminal) = hybrid_method_outcome(&outcome);
                assert_eq!(method, AnalyzeMethod::HybridReachability);
                let expected = match outcome {
                    HybridOutcome::Enclosure { .. } => AnalyzeOutcome::Enclosure,
                    HybridOutcome::Incomplete { .. } => AnalyzeOutcome::Incomplete,
                    HybridOutcome::Refused { .. } => AnalyzeOutcome::Refused,
                };
                assert_eq!(terminal, expected);
            }
        }
    }
}

proptest! {
    #[trace("TC-040", "FR-040-AC-12")]
    #[test]
    #[ignore = "VER-94 finding F4: transitions drop the source mode (unsound under non-forced semantics)"]
    fn fr040_ac12_random_small_models_are_sound_against_enumeration(
        lower in -10i64..10,
        width in 0i64..5,
        dl in -2i64..2,
        dw in 0i64..2,
        horizon in 0u64..4,
        guards in prop::collection::vec((0usize..3, 0usize..3, -10i64..10, 0i64..8, -5i64..5), 0..4),
    ) {
        let modes = ["a", "b", "c"];
        let transitions = guards
            .into_iter()
            .map(|(from, to, glow, gw, offset)| {
                transition(modes[from], modes[to], (glow, glow + gw), offset)
            })
            .collect();
        let req = with_transitions(request("a", lower, lower + width, horizon, (dl, dl + dw)), transitions);
        assert_sound(&req, Claim::Horizon, None);
    }
}

// ------------------------------------------------------------------ NFR-040

#[trace("TC-040", "NFR-040-AC-1")]
#[test]
#[ignore = "VER-94 finding F10: no crate limit on step bound, transitions or distinct modes"]
fn nfr040_ac1_no_crate_limits_on_steps_transitions_modes() {
    // A step bound far above any plausible crate limit ends Incomplete.
    let mut steps = request("m", 0, 0, 0, (0, 0));
    steps.step_bound = 1_000_000;
    assert!(matches!(
        run(&steps),
        HybridOutcome::Incomplete {
            code: HybridIncompleteCode::StepBoundExhausted,
            ..
        }
    ));
    // So many transitions, and so many distinct modes, end Refused.
    let many = |distinct: bool| {
        let transitions = (0..100_000)
            .map(|n| {
                let to = if distinct {
                    format!("t{n}")
                } else {
                    "t".into()
                };
                transition("m", &to, (0, 0), 0)
            })
            .collect();
        with_transitions(request("m", 0, 0, 0, (0, 0)), transitions)
    };
    for req in [many(false), many(true)] {
        assert!(matches!(run(&req), HybridOutcome::Refused { .. }));
    }
}

#[trace("TC-040", "NFR-040-AC-2")]
#[test]
#[ignore = "VER-94 finding F9: no crate step limit; an admitted horizon is iterated one step at a time"]
fn nfr040_ac2_large_horizon_is_cut_by_a_crate_limit_not_iterated() {
    // Bounded stand-in for u64::MAX: a crate limit ends this Incomplete before any
    // per-step work; today all 1,000,000 steps run and an enclosure is returned.
    let mut req = request("m", 0, 0, 1_000_000, (0, 0));
    req.step_bound = 1_000_000;
    assert!(matches!(run(&req), HybridOutcome::Incomplete { .. }));
}
