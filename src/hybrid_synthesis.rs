// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded canonical synthesis for FR-008.

use quire_analyze_method_contract::{AnalyzeMethod, AnalyzeOutcome, AnalyzeResult};
use sha2::{Digest, Sha256};

/// Finite ceiling on caller-controlled combination depth.
///
/// This preserves the recursive search implementation's bounded stack use.
const MAX_SYNTHESIS_TERMS: usize = 64;
/// Maximum atoms admitted before canonicalization allocates owned input.
const MAX_SYNTHESIS_ATOMS: usize = 4_096;
/// Maximum UTF-8 bytes admitted across both atom collections.
const MAX_SYNTHESIS_ATOM_BYTES: usize = 1_048_576;

/// Canonical synthesis atom.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SynthesisAtom(pub String);

/// Exact canonical synthesis search identity.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SynthesisProblemIdentity(pub String);

/// Exact canonical synthesis candidate identity.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SynthesisCandidateIdentity(pub String);

/// Immutable independent validation evidence identity.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ValidationEvidenceIdentity(pub String);

macro_rules! string_identity {
    ($($identity:ty),+ $(,)?) => {$(
        impl From<&str> for $identity {
            fn from(value: &str) -> Self { Self(value.into()) }
        }
    )+};
}

string_identity!(
    SynthesisAtom,
    SynthesisProblemIdentity,
    SynthesisCandidateIdentity,
    ValidationEvidenceIdentity
);

/// A finite canonical synthesis request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesisRequest {
    /// Set-like candidate atoms.
    pub atoms: Vec<SynthesisAtom>,
    /// Atoms every satisfying candidate must contain.
    pub required_atoms: Vec<SynthesisAtom>,
    /// Maximum terms in an admissible candidate.
    pub max_terms: usize,
    /// Maximum candidates that may be inspected.
    pub search_bound: usize,
}

/// A canonical candidate, never a proof of satisfaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesisCandidate {
    /// Sorted, duplicate-free candidate terms.
    pub terms: Vec<SynthesisAtom>,
    /// Exact search-request identity.
    pub problem_identity: SynthesisProblemIdentity,
    /// Canonical candidate identity.
    pub identity: SynthesisCandidateIdentity,
}

/// A separate independent validation input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationRequest {
    /// Candidate being checked.
    pub candidate: SynthesisCandidate,
    /// Search problem that the independent validator received.
    pub problem_identity: SynthesisProblemIdentity,
}

/// Externally supplied decision from an independent candidate validator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationDecision {
    /// The validator accepted this exact candidate with retained evidence.
    Accepted(ValidationAttestation),
    /// The validator rejected this exact candidate with retained evidence.
    Rejected(ValidationAttestation),
}

/// Immutable validator attestation bound to one exact candidate and problem.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationAttestation {
    /// Exact candidate identity reviewed by the validator.
    pub candidate_identity: SynthesisCandidateIdentity,
    /// Exact search problem reviewed by the validator.
    pub problem_identity: SynthesisProblemIdentity,
    /// Immutable validation evidence identity.
    pub evidence_identity: ValidationEvidenceIdentity,
}

/// Trust boundary for independently validating a synthesized candidate.
pub trait CandidateValidator {
    /// Validates the exact candidate and search problem without delegating to discovery.
    fn validate(&self, request: &ValidationRequest) -> ValidationDecision;
}

/// A closed terminal result for bounded synthesis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SynthesisOutcome {
    /// An unvalidated canonical candidate.
    Candidate(SynthesisCandidate),
    /// A candidate validated with independent evidence.
    Validated {
        /// Candidate preserved exactly.
        candidate: SynthesisCandidate,
        /// Independent proof/evidence identity.
        proof: ValidationEvidenceIdentity,
    },
    /// The finite candidate space was exhausted without a candidate.
    NoCandidate,
    /// The search bound ended before finite-space exhaustion.
    Incomplete {
        /// Stable non-conclusion category.
        code: SynthesisIncompleteCode,
        /// Non-conclusion explanation.
        reason: String,
    },
    /// Independent validation rejected this exact candidate.
    ValidationFailed {
        /// Candidate preserved for diagnosis.
        candidate: SynthesisCandidate,
        /// Stable validation rejection category.
        code: ValidationFailureCode,
        /// Rejection explanation.
        reason: String,
    },
    /// Input or candidate identity was malformed.
    Refused {
        /// Stable refusal category.
        code: SynthesisRefusalCode,
        /// Refusal explanation.
        reason: String,
    },
}

/// Stable non-conclusion categories for canonical synthesis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynthesisIncompleteCode {
    /// The finite candidate-inspection budget was exhausted.
    SearchBoundExhausted,
}

/// Stable validation-rejection categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationFailureCode {
    /// The independent validator rejected the candidate.
    Rejected,
}

/// Stable refusal categories for canonical synthesis and validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynthesisRefusalCode {
    /// Bounds, atoms, or request shape are invalid.
    InvalidRequest,
    /// Atom count or bytes exceed the admitted resource bound.
    InputTooLarge,
    /// Validation does not bind the exact candidate and problem.
    InvalidValidationBinding,
}

/// Maps a native synthesis terminal outcome to the shared provider contract.
#[must_use]
pub const fn synthesis_method_outcome(
    outcome: &SynthesisOutcome,
) -> (AnalyzeMethod, AnalyzeOutcome) {
    let terminal = match outcome {
        SynthesisOutcome::Candidate(_) => AnalyzeOutcome::Candidate,
        SynthesisOutcome::Validated { .. } => AnalyzeOutcome::ValidatedCandidate,
        SynthesisOutcome::NoCandidate => AnalyzeOutcome::NoCandidate,
        SynthesisOutcome::Incomplete { .. } => AnalyzeOutcome::Incomplete,
        SynthesisOutcome::ValidationFailed { .. } => AnalyzeOutcome::ValidationRejected,
        SynthesisOutcome::Refused { .. } => AnalyzeOutcome::Refused,
    };
    (AnalyzeMethod::CanonicalSynthesis, terminal)
}

/// Binds a native synthesis outcome into the exact shared result envelope.
#[must_use]
pub fn bind_synthesis_result(
    mut result: AnalyzeResult,
    outcome: &SynthesisOutcome,
) -> Option<AnalyzeResult> {
    let (method, terminal) = synthesis_method_outcome(outcome);
    (result.method == method).then(|| {
        result.outcome = terminal;
        result
    })
}

/// Enumerates finite canonical candidates in length then lexical order.
#[must_use]
pub fn synthesize(request: &SynthesisRequest) -> SynthesisOutcome {
    if request.atoms.len() > MAX_SYNTHESIS_ATOMS
        || request.required_atoms.len() > MAX_SYNTHESIS_ATOMS
        || atom_bytes(&request.atoms).saturating_add(atom_bytes(&request.required_atoms))
            > MAX_SYNTHESIS_ATOM_BYTES
    {
        return SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InputTooLarge,
            reason: "synthesis atoms exceed the admitted resource bound".into(),
        };
    }
    let atoms = canonical_atoms(&request.atoms);
    let required = canonical_atoms(&request.required_atoms);
    if request.search_bound == 0
        || request.max_terms == 0
        || request.max_terms > MAX_SYNTHESIS_TERMS
        || atoms.iter().any(|atom| atom.0.is_empty())
        || required.iter().any(|atom| atom.0.is_empty())
    {
        return SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InvalidRequest,
            reason: "invalid finite synthesis bound or atom".into(),
        };
    }
    let identity = synthesis_identity(&atoms, &required, request.max_terms, request.search_bound);
    let mut inspected = 0usize;
    let max_terms = request.max_terms.min(atoms.len());
    for size in 1..=max_terms {
        match inspect_combinations(
            &atoms,
            &required,
            size,
            &mut inspected,
            request.search_bound,
        ) {
            CombinationSearch::Candidate(terms) => {
                let candidate = SynthesisCandidate {
                    identity: candidate_identity(&identity, &terms),
                    problem_identity: identity,
                    terms,
                };
                return SynthesisOutcome::Candidate(candidate);
            }
            CombinationSearch::Incomplete => {
                return SynthesisOutcome::Incomplete {
                    code: SynthesisIncompleteCode::SearchBoundExhausted,
                    reason: "canonical candidate search bound exhausted".into(),
                };
            }
            CombinationSearch::Exhausted => {}
        }
    }
    SynthesisOutcome::NoCandidate
}

/// Binds an independently supplied validation result to its exact candidate.
#[must_use]
pub fn validate_candidate(
    validation: ValidationRequest,
    validator: &dyn CandidateValidator,
) -> SynthesisOutcome {
    if validation.problem_identity != validation.candidate.problem_identity
        || validation.candidate.terms.is_empty()
        || validation.candidate.identity
            != candidate_identity(
                &validation.candidate.problem_identity,
                &validation.candidate.terms,
            )
    {
        return SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InvalidValidationBinding,
            reason: "validation does not bind the exact candidate and problem".into(),
        };
    }
    let decision = validator.validate(&validation);
    let attestation = match &decision {
        ValidationDecision::Accepted(attestation) | ValidationDecision::Rejected(attestation) => {
            attestation
        }
    };
    if attestation.candidate_identity != validation.candidate.identity
        || attestation.problem_identity != validation.problem_identity
        || attestation.evidence_identity.0.is_empty()
    {
        return SynthesisOutcome::Refused {
            code: SynthesisRefusalCode::InvalidValidationBinding,
            reason: "validation attestation does not bind the exact candidate and problem".into(),
        };
    }
    match decision {
        ValidationDecision::Accepted(attestation) => SynthesisOutcome::Validated {
            candidate: validation.candidate,
            proof: attestation.evidence_identity,
        },
        ValidationDecision::Rejected(_) => SynthesisOutcome::ValidationFailed {
            candidate: validation.candidate,
            code: ValidationFailureCode::Rejected,
            reason: "independent validator rejected candidate".into(),
        },
    }
}

fn atom_bytes(atoms: &[SynthesisAtom]) -> usize {
    atoms
        .iter()
        .fold(0usize, |total, atom| total.saturating_add(atom.0.len()))
}

fn canonical_atoms(atoms: &[SynthesisAtom]) -> Vec<SynthesisAtom> {
    let mut canonical = atoms.to_vec();
    canonical.sort();
    canonical.dedup();
    canonical
}
enum CombinationSearch {
    Candidate(Vec<SynthesisAtom>),
    Exhausted,
    Incomplete,
}

fn inspect_combinations(
    atoms: &[SynthesisAtom],
    required: &[SynthesisAtom],
    size: usize,
    inspected: &mut usize,
    search_bound: usize,
) -> CombinationSearch {
    let mut partial = Vec::with_capacity(size);
    inspect_combination_prefix(
        atoms,
        required,
        size,
        0,
        &mut partial,
        inspected,
        search_bound,
    )
}

fn inspect_combination_prefix(
    atoms: &[SynthesisAtom],
    required: &[SynthesisAtom],
    remaining: usize,
    start: usize,
    partial: &mut Vec<SynthesisAtom>,
    inspected: &mut usize,
    search_bound: usize,
) -> CombinationSearch {
    if remaining == 0 {
        if *inspected == search_bound {
            return CombinationSearch::Incomplete;
        }
        *inspected = inspected.saturating_add(1);
        return if required
            .iter()
            .all(|required_atom| partial.binary_search(required_atom).is_ok())
        {
            CombinationSearch::Candidate(partial.clone())
        } else {
            CombinationSearch::Exhausted
        };
    }
    for index in start..=atoms.len().saturating_sub(remaining) {
        partial.push(atoms[index].clone());
        match inspect_combination_prefix(
            atoms,
            required,
            remaining - 1,
            index + 1,
            partial,
            inspected,
            search_bound,
        ) {
            CombinationSearch::Exhausted => {}
            result => {
                partial.pop();
                return result;
            }
        }
        partial.pop();
    }
    CombinationSearch::Exhausted
}
fn synthesis_identity(
    atoms: &[SynthesisAtom],
    required: &[SynthesisAtom],
    max_terms: usize,
    search_bound: usize,
) -> SynthesisProblemIdentity {
    let mut bytes = b"quire-analyze/synthesis-problem/v1".to_vec();
    encode_atoms(&mut bytes, atoms);
    encode_atoms(&mut bytes, required);
    encode_text(&mut bytes, &max_terms.to_string());
    encode_text(&mut bytes, &search_bound.to_string());
    SynthesisProblemIdentity(digest(&bytes))
}

fn candidate_identity(
    problem_identity: &SynthesisProblemIdentity,
    terms: &[SynthesisAtom],
) -> SynthesisCandidateIdentity {
    let mut bytes = b"quire-analyze/synthesis-candidate/v1".to_vec();
    encode_text(&mut bytes, &problem_identity.0);
    encode_atoms(&mut bytes, terms);
    SynthesisCandidateIdentity(digest(&bytes))
}

fn encode_atoms(bytes: &mut Vec<u8>, atoms: &[SynthesisAtom]) {
    encode_text(bytes, &atoms.len().to_string());
    for atom in atoms {
        encode_text(bytes, &atom.0);
    }
}

fn encode_text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(value.len().to_string().as_bytes());
    bytes.push(b':');
    bytes.extend_from_slice(value.as_bytes());
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use ix_trace_rs::trace;
    use quire_analyze_method_contract::{
        AnalyzeAssumptionsIdentity, AnalyzeBoundsIdentity, AnalyzeContentIdentity,
        AnalyzeImplementationIdentity, AnalyzeOptionsIdentity, AnalyzeRequestIdentity,
        AnalyzeResultIdentity, AnalyzeRunIdentity, AnalyzeSubjectIdentity,
        AnalyzeToolchainIdentity, ProviderRevision, METHOD_RESULT_VERSION,
    };

    fn envelope(method: AnalyzeMethod) -> AnalyzeResult {
        AnalyzeResult {
            contract_version: METHOD_RESULT_VERSION.into(),
            provider_revision: ProviderRevision("rev".into()),
            method,
            outcome: AnalyzeOutcome::Failed,
            subject_identity: AnalyzeSubjectIdentity("subject".into()),
            run_identity: AnalyzeRunIdentity("run".into()),
            request_identity: AnalyzeRequestIdentity("request".into()),
            implementation_identity: AnalyzeImplementationIdentity("implementation".into()),
            toolchain_identity: AnalyzeToolchainIdentity("toolchain".into()),
            options_identity: AnalyzeOptionsIdentity("options".into()),
            assumptions_identity: AnalyzeAssumptionsIdentity("assumptions".into()),
            bounds_identity: AnalyzeBoundsIdentity("bounds".into()),
            content_identity: AnalyzeContentIdentity("content".into()),
            result_identity: AnalyzeResultIdentity("result".into()),
        }
    }

    #[trace("TC-014")]
    #[test]
    fn binders_preserve_exact_envelope_and_refuse_wrong_method() {
        for (outcome, expected_outcome) in [
            (
                synthesize(&SynthesisRequest {
                    atoms: vec!["a".into()],
                    required_atoms: vec!["a".into()],
                    max_terms: 1,
                    search_bound: 1,
                }),
                AnalyzeOutcome::Candidate,
            ),
            (
                synthesize(&SynthesisRequest {
                    atoms: vec!["a".into()],
                    required_atoms: vec!["missing".into()],
                    max_terms: 1,
                    search_bound: 1,
                }),
                AnalyzeOutcome::NoCandidate,
            ),
            (
                synthesize(&SynthesisRequest {
                    atoms: vec!["a".into(), "b".into()],
                    required_atoms: vec!["b".into()],
                    max_terms: 2,
                    search_bound: 1,
                }),
                AnalyzeOutcome::Incomplete,
            ),
        ] {
            let synthesis_envelope = envelope(AnalyzeMethod::CanonicalSynthesis);
            let mut expected = synthesis_envelope.clone();
            expected.outcome = expected_outcome;
            assert_eq!(
                bind_synthesis_result(synthesis_envelope, &outcome),
                Some(expected)
            );
        }
        let SynthesisOutcome::Candidate(candidate) = synthesize(&SynthesisRequest {
            atoms: vec!["a".into()],
            required_atoms: vec!["a".into()],
            max_terms: 1,
            search_bound: 1,
        }) else {
            panic!("candidate fixture")
        };
        for (outcome, expected_outcome) in [
            (
                SynthesisOutcome::Validated {
                    candidate: candidate.clone(),
                    proof: ValidationEvidenceIdentity("proof".into()),
                },
                AnalyzeOutcome::ValidatedCandidate,
            ),
            (
                SynthesisOutcome::ValidationFailed {
                    candidate: candidate.clone(),
                    code: ValidationFailureCode::Rejected,
                    reason: "rejected".into(),
                },
                AnalyzeOutcome::ValidationRejected,
            ),
            (
                SynthesisOutcome::Refused {
                    code: SynthesisRefusalCode::InvalidRequest,
                    reason: "refused".into(),
                },
                AnalyzeOutcome::Refused,
            ),
        ] {
            let source = envelope(AnalyzeMethod::CanonicalSynthesis);
            let mut expected = source.clone();
            expected.outcome = expected_outcome;
            assert_eq!(bind_synthesis_result(source, &outcome), Some(expected));
        }
    }
}
