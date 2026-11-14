// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use ix_trace_rs::trace;
use proptest::prelude::*;
use quire_analyze::{
    synthesis_method_outcome, synthesize, validate_candidate, CandidateValidator, SynthesisAtom,
    SynthesisIncompleteCode, SynthesisOutcome, SynthesisRefusalCode, SynthesisRequest,
    ValidationAttestation, ValidationDecision, ValidationFailureCode, ValidationRequest,
};
use quire_analyze_method_contract::{AnalyzeMethod, AnalyzeOutcome};

struct FixtureValidator(bool);

impl CandidateValidator for FixtureValidator {
    fn validate(&self, request: &ValidationRequest) -> ValidationDecision {
        let attestation = ValidationAttestation {
            candidate_identity: request.candidate.identity.clone(),
            problem_identity: request.problem_identity.clone(),
            evidence_identity: "validator:run".into(),
        };
        if self.0 {
            ValidationDecision::Accepted(attestation)
        } else {
            ValidationDecision::Rejected(attestation)
        }
    }
}

struct MismatchedValidator;

impl CandidateValidator for MismatchedValidator {
    fn validate(&self, request: &ValidationRequest) -> ValidationDecision {
        ValidationDecision::Accepted(ValidationAttestation {
            candidate_identity: "sha256:other".into(),
            problem_identity: request.problem_identity.clone(),
            evidence_identity: "validator:mismatch".into(),
        })
    }
}

proptest! {
    #[trace("TC-014", "FR-008-AC-1")]
    #[test]
    fn tc_014_canonicalizes_arbitrary_finite_set_like_inputs(
        atoms in prop::collection::vec("[a-z]{1,4}", 1..32)
    ) {
        let request = SynthesisRequest {
            atoms: atoms.iter().cloned().map(SynthesisAtom).collect(),
            required_atoms: vec![],
            max_terms: 1,
            search_bound: 1,
        };
        let mut reordered = atoms;
        reordered.reverse();
        let reordered = SynthesisRequest {
            atoms: reordered.into_iter().map(SynthesisAtom).collect(),
            ..request.clone()
        };
        prop_assert_eq!(synthesize(&request), synthesize(&reordered));
    }
}

#[trace("TC-014", "FR-008-AC-1", "FR-008-AC-2")]
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
    assert_eq!(
        synthesis_method_outcome(&SynthesisOutcome::Candidate(candidate.clone())),
        (AnalyzeMethod::CanonicalSynthesis, AnalyzeOutcome::Candidate)
    );
    assert_eq!(candidate, other);
    assert!(matches!(
        validate_candidate(
            ValidationRequest {
                candidate: candidate.clone(),
                problem_identity: candidate.problem_identity.clone(),
            },
            &FixtureValidator(false)
        ),
        SynthesisOutcome::ValidationFailed {
            code: ValidationFailureCode::Rejected,
            ..
        }
    ));
    assert!(matches!(
        validate_candidate(
            ValidationRequest {
                candidate,
                problem_identity: "different-problem".into(),
            },
            &FixtureValidator(true)
        ),
        SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InvalidValidationBinding,
            ..
        }
    ));
    let SynthesisOutcome::Candidate(mut tampered) = synthesize(&request) else {
        panic!("expected candidate");
    };
    tampered.identity = "sha256:tampered".into();
    assert!(matches!(
        validate_candidate(
            ValidationRequest {
                problem_identity: tampered.problem_identity.clone(),
                candidate: tampered,
            },
            &FixtureValidator(true)
        ),
        SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InvalidValidationBinding,
            ..
        }
    ));
    let SynthesisOutcome::Candidate(candidate) = synthesize(&request) else {
        panic!("expected candidate");
    };
    assert!(matches!(
        validate_candidate(
            ValidationRequest {
                problem_identity: candidate.problem_identity.clone(),
                candidate,
            },
            &MismatchedValidator,
        ),
        SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InvalidValidationBinding,
            ..
        }
    ));
}

#[trace("TC-014", "FR-008-AC-3")]
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
        SynthesisOutcome::Incomplete {
            code: SynthesisIncompleteCode::SearchBoundExhausted,
            ..
        }
    ));
}

#[trace("TC-014", "FR-008-AC-1", "FR-008-AC-3")]
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
        SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InvalidRequest,
            ..
        }
    ));
}

#[trace("TC-014", "FR-008-AC-1")]
#[test]
fn tc_014_enforces_aggregate_atom_byte_boundary() {
    let at_limit = SynthesisRequest {
        atoms: vec![SynthesisAtom("a".repeat(1_048_576))],
        required_atoms: vec![],
        max_terms: 1,
        search_bound: 1,
    };
    assert!(matches!(
        synthesize(&at_limit),
        SynthesisOutcome::Candidate(_)
    ));
    let over_limit = SynthesisRequest {
        atoms: vec![SynthesisAtom("a".repeat(1_048_577))],
        ..at_limit
    };
    assert!(matches!(
        synthesize(&over_limit),
        SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InputTooLarge,
            ..
        }
    ));
}
