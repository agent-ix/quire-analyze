// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use quire_analyze::{
    reach, replay_hybrid, synthesize, validate_candidate, HybridOutcome, HybridRequest,
    HybridTransition, Interval, SynthesisAtom, SynthesisOutcome, SynthesisRequest,
    ValidationRequest,
};

fn model() -> HybridRequest {
    HybridRequest {
        initial_mode: "warmup".into(),
        transitions: vec![HybridTransition {
            from_mode: "warmup".into(),
            to_mode: "run".into(),
            guard: Interval { lower: 1, upper: 3 },
            reset_offset: 10,
        }],
        initial_set: Interval { lower: 0, upper: 1 },
        horizon: 2,
        flow_delta_lower: 1,
        flow_delta_upper: 2,
        error_bound: 1,
        step_bound: 2,
        convergence_steps: 2,
    }
}

/// Trace: TC-013, FR-007-AC-1, FR-007-AC-3.
#[test]
fn tc_013_preserves_mode_guard_reset_and_exact_replay() {
    let request = model();
    let outcome = reach(&request);
    let HybridOutcome::Enclosure {
        enclosures,
        witness,
        ..
    } = outcome
    else {
        panic!("expected enclosure");
    };
    assert_eq!(enclosures.len(), 1);
    assert_eq!(enclosures[0].mode, "run".into());
    assert_eq!(
        enclosures[0].interval,
        Interval {
            lower: 11,
            upper: 16
        }
    );
    assert!(replay_hybrid(&request, &witness));
    let mut changed = request;
    changed.transitions[0].reset_offset = 11;
    assert!(!replay_hybrid(&changed, &witness));
}

/// Trace: TC-013, FR-007-AC-2.
#[test]
fn tc_013_refuses_malformed_models_and_keeps_bound_exhaustion_nonconclusive() {
    let mut bounded = model();
    bounded.step_bound = 1;
    assert!(matches!(reach(&bounded), HybridOutcome::Incomplete { .. }));
    let mut malformed = model();
    malformed.transitions[0].guard = Interval { lower: 4, upper: 3 };
    assert!(matches!(reach(&malformed), HybridOutcome::Refused { .. }));
}

/// Trace: TC-014, FR-008-AC-1, FR-008-AC-2.
#[test]
fn tc_014_canonical_candidate_requires_exact_independent_validation() {
    let request = SynthesisRequest {
        atoms: vec!["b".into(), "a".into(), "a".into()],
        required_atoms: vec!["a".into()],
        max_terms: 2,
        search_bound: 3,
    };
    let first = synthesize(&request);
    let second = synthesize(&SynthesisRequest {
        atoms: vec!["a".into(), "b".into()],
        ..request.clone()
    });
    let (SynthesisOutcome::Candidate(candidate), SynthesisOutcome::Candidate(other)) =
        (first, second)
    else {
        panic!("expected candidates");
    };
    assert_eq!(candidate, other);
    assert!(matches!(
        validate_candidate(ValidationRequest {
            candidate: candidate.clone(),
            problem_identity: candidate.problem_identity.clone(),
            accepted: false,
            evidence_identity: "validator:run:1".into()
        }),
        SynthesisOutcome::ValidationFailed { .. }
    ));
    assert!(matches!(
        validate_candidate(ValidationRequest {
            candidate,
            problem_identity: "different-problem".into(),
            accepted: true,
            evidence_identity: "validator:run:2".into()
        }),
        SynthesisOutcome::Refused { .. }
    ));
    let SynthesisOutcome::Candidate(mut tampered) = synthesize(&request) else {
        panic!("expected candidate");
    };
    tampered.identity = "sha256:tampered".into();
    assert!(matches!(
        validate_candidate(ValidationRequest {
            problem_identity: tampered.problem_identity.clone(),
            candidate: tampered,
            accepted: true,
            evidence_identity: "validator:run:3".into(),
        }),
        SynthesisOutcome::Refused { .. }
    ));
}

/// Trace: TC-014, FR-008-AC-3.
#[test]
fn tc_014_distinguishes_exhaustive_no_candidate_from_incomplete_search() {
    let exhaustive = SynthesisRequest {
        atoms: vec!["a".into()],
        required_atoms: vec!["missing".into()],
        max_terms: 1,
        search_bound: 1,
    };
    assert!(matches!(
        synthesize(&exhaustive),
        SynthesisOutcome::NoCandidate
    ));
    let incomplete = SynthesisRequest {
        atoms: vec!["a".into(), "b".into()],
        required_atoms: vec!["b".into()],
        max_terms: 2,
        search_bound: 1,
    };
    assert!(matches!(
        synthesize(&incomplete),
        SynthesisOutcome::Incomplete { .. }
    ));
}

/// Trace: TC-014, FR-008-AC-1, FR-008-AC-3.
#[test]
fn tc_014_distinguishes_delimited_atoms_and_stops_at_search_bound() {
    let single_atom = SynthesisRequest {
        atoms: vec!["a\u{1f}b".into()],
        required_atoms: vec!["a\u{1f}b".into()],
        max_terms: 1,
        search_bound: 1,
    };
    let split_atoms = SynthesisRequest {
        atoms: vec!["a".into(), "b".into()],
        required_atoms: vec!["a".into(), "b".into()],
        max_terms: 2,
        search_bound: 3,
    };
    let (SynthesisOutcome::Candidate(single), SynthesisOutcome::Candidate(split)) =
        (synthesize(&single_atom), synthesize(&split_atoms))
    else {
        panic!("expected candidates");
    };
    assert_ne!(single.problem_identity, split.problem_identity);
    let different_bound = SynthesisRequest {
        search_bound: 2,
        ..single_atom.clone()
    };
    let SynthesisOutcome::Candidate(different_bound) = synthesize(&different_bound) else {
        panic!("expected candidate");
    };
    assert_ne!(single.problem_identity, different_bound.problem_identity);
    let bounded = SynthesisRequest {
        atoms: (0..10_000)
            .map(|index| SynthesisAtom(format!("atom:{index}")))
            .collect(),
        required_atoms: vec!["missing".into()],
        max_terms: 1,
        search_bound: 1,
    };
    assert!(matches!(
        synthesize(&bounded),
        SynthesisOutcome::Refused {
            code: quire_analyze::SynthesisRefusalCode::InputTooLarge,
            ..
        }
    ));
    let deep = SynthesisRequest {
        atoms: (0..65)
            .map(|index| SynthesisAtom(format!("deep:{index}")))
            .collect(),
        required_atoms: vec!["deep:0".into()],
        max_terms: 65,
        search_bound: 1,
    };
    assert!(matches!(
        synthesize(&deep),
        SynthesisOutcome::Refused { .. }
    ));
}
