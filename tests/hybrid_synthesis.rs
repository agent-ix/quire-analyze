use quire_analyze::{
    reach, synthesize, validate_candidate, HybridOutcome, HybridRequest, SynthesisOutcome,
    SynthesisRequest,
};

#[test]
fn tc_013_hybrid_enclosures_never_turn_failed_convergence_into_proof() {
    let failed = reach(&HybridRequest {
        mode: "m".into(),
        guard: "g".into(),
        reset: "r".into(),
        initial_lower: 0,
        initial_upper: 1,
        horizon: 1,
        error_bound: 1,
        convergence_steps: 2,
        step_bound: 1,
    });
    assert!(matches!(failed, HybridOutcome::Incomplete { .. }));
    let enclosure = reach(&HybridRequest {
        mode: "m".into(),
        guard: "g".into(),
        reset: "r".into(),
        initial_lower: 0,
        initial_upper: 1,
        horizon: 1,
        error_bound: 2,
        convergence_steps: 1,
        step_bound: 1,
    });
    assert!(matches!(
        enclosure,
        HybridOutcome::Enclosure {
            lower: -2,
            upper: 3,
            error_bound: 2,
            ..
        }
    ));
}
#[test]
fn tc_014_candidate_and_independent_validation_are_distinct() {
    let candidate = synthesize(&SynthesisRequest {
        atoms: vec!["b".into(), "a".into(), "a".into()],
        search_bound: 1,
        validator_accepts: false,
    });
    assert!(
        matches!(&candidate,SynthesisOutcome::Candidate { canonical,.. } if canonical=="a & b")
    );
    assert!(matches!(
        validate_candidate(candidate, false),
        SynthesisOutcome::ValidationFailed { .. }
    ));
    assert!(matches!(
        synthesize(&SynthesisRequest {
            atoms: vec![],
            search_bound: 1,
            validator_accepts: true
        }),
        SynthesisOutcome::NoCandidate
    ));
    assert!(matches!(
        synthesize(&SynthesisRequest {
            atoms: vec!["a".into()],
            search_bound: 0,
            validator_accepts: true
        }),
        SynthesisOutcome::Incomplete { .. }
    ));
}
