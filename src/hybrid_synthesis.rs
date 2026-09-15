// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded hybrid reachability and canonical synthesis for FR-007 and FR-008.
//!
//! Both providers are deliberately conservative: reachability emits enclosures,
//! while synthesis emits a candidate that must be independently validated.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

/// Exact hybrid mode identity.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ModeIdentity(pub String);

/// Exact hybrid-model identity for witness replay.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct HybridModelIdentity(pub String);

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
    ModeIdentity,
    SynthesisAtom,
    SynthesisProblemIdentity,
    SynthesisCandidateIdentity,
    ValidationEvidenceIdentity,
);

/// A closed integer interval used for initial sets, guards, and enclosures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interval {
    /// Inclusive lower bound.
    pub lower: i64,
    /// Inclusive upper bound.
    pub upper: i64,
}

/// A deterministic guarded/reset transition in a hybrid model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridTransition {
    /// Source mode identity.
    pub from_mode: ModeIdentity,
    /// Target mode identity.
    pub to_mode: ModeIdentity,
    /// Guard interval; overlap enables a conservative transition.
    pub guard: Interval,
    /// Additive reset applied after a guard overlap.
    pub reset_offset: i64,
}

/// A finite hybrid reachability request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridRequest {
    /// Initial exact mode identity.
    pub initial_mode: ModeIdentity,
    /// All authored guarded/reset transitions.
    pub transitions: Vec<HybridTransition>,
    /// Initial state enclosure.
    pub initial_set: Interval,
    /// Number of discrete flow/transition steps to analyze.
    pub horizon: u64,
    /// Inclusive lower change of one flow step.
    pub flow_delta_lower: i64,
    /// Inclusive upper change of one flow step.
    pub flow_delta_upper: i64,
    /// Declared widening error applied to every final enclosure.
    pub error_bound: u64,
    /// Maximum admitted reachability iterations.
    pub step_bound: u64,
    /// Iterations used by the submitted numerical enclosure method.
    pub convergence_steps: u64,
}

/// A replayable witness that binds exact model input and final enclosures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridWitness {
    /// Digest of the entire exact request.
    pub model_identity: HybridModelIdentity,
    /// Analyzed horizon.
    pub horizon: u64,
    /// Final enclosures keyed by mode identity.
    pub enclosures: Vec<ModeEnclosure>,
}

/// An enclosure for one reachable hybrid mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModeEnclosure {
    /// Mode identity.
    pub mode: ModeIdentity,
    /// Sound final reachable interval.
    pub interval: Interval,
}

/// A closed terminal result for hybrid reachability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridOutcome {
    /// Conservative enclosures and a replayable witness.
    Enclosure {
        /// Enclosures for each reachable mode.
        enclosures: Vec<ModeEnclosure>,
        /// Declared final widening bound.
        error_bound: u64,
        /// Exact replay witness.
        witness: HybridWitness,
    },
    /// The request was valid but its admitted finite analysis did not finish.
    Incomplete {
        /// Typed non-conclusion explanation.
        reason: String,
    },
    /// The model or bound was structurally invalid.
    Refused {
        /// Refusal explanation.
        reason: String,
    },
}

/// Computes mode-sensitive sound interval enclosures for a finite horizon.
#[must_use]
pub fn reach(request: &HybridRequest) -> HybridOutcome {
    if let Err(reason) = validate_hybrid(request) {
        return HybridOutcome::Refused { reason };
    }
    if request.horizon > request.step_bound || request.convergence_steps > request.step_bound {
        return HybridOutcome::Incomplete {
            reason: "declared hybrid step bound exhausted".into(),
        };
    }
    let widening = i64::try_from(request.error_bound).unwrap_or(i64::MAX);
    let mut current = BTreeMap::from([(request.initial_mode.clone(), request.initial_set)]);
    for _ in 0..request.horizon {
        let mut next = BTreeMap::new();
        for (mode, interval) in current {
            let flowed = Interval {
                lower: interval.lower.saturating_add(request.flow_delta_lower),
                upper: interval.upper.saturating_add(request.flow_delta_upper),
            };
            let enabled: Vec<_> = request
                .transitions
                .iter()
                .filter(|transition| {
                    transition.from_mode == mode && overlaps(flowed, transition.guard)
                })
                .collect();
            if enabled.is_empty() {
                join_interval(&mut next, mode, flowed);
            } else {
                for transition in enabled {
                    join_interval(
                        &mut next,
                        transition.to_mode.clone(),
                        Interval {
                            lower: flowed.lower.saturating_add(transition.reset_offset),
                            upper: flowed.upper.saturating_add(transition.reset_offset),
                        },
                    );
                }
            }
        }
        current = next;
    }
    let enclosures: Vec<_> = current
        .into_iter()
        .map(|(mode, interval)| ModeEnclosure {
            mode,
            interval: Interval {
                lower: interval.lower.saturating_sub(widening),
                upper: interval.upper.saturating_add(widening),
            },
        })
        .collect();
    let witness = HybridWitness {
        model_identity: hybrid_identity(request),
        horizon: request.horizon,
        enclosures: enclosures.clone(),
    };
    HybridOutcome::Enclosure {
        enclosures,
        error_bound: request.error_bound,
        witness,
    }
}

/// Replays a witness only against the exact request and identical enclosure.
#[must_use]
pub fn replay_hybrid(request: &HybridRequest, witness: &HybridWitness) -> bool {
    if witness.model_identity != hybrid_identity(request) || witness.horizon != request.horizon {
        return false;
    }
    matches!(reach(request), HybridOutcome::Enclosure { witness: replayed, .. } if replayed == *witness)
}

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
    /// External validator result; discovery cannot set this value.
    pub accepted: bool,
    /// Immutable validation evidence identity.
    pub evidence_identity: ValidationEvidenceIdentity,
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
        /// Non-conclusion explanation.
        reason: String,
    },
    /// Independent validation rejected this exact candidate.
    ValidationFailed {
        /// Candidate preserved for diagnosis.
        candidate: SynthesisCandidate,
        /// Rejection explanation.
        reason: String,
    },
    /// Input or candidate identity was malformed.
    Refused {
        /// Refusal explanation.
        reason: String,
    },
}

/// Enumerates finite canonical candidates in length then lexical order.
#[must_use]
pub fn synthesize(request: &SynthesisRequest) -> SynthesisOutcome {
    let atoms = canonical_atoms(&request.atoms);
    let required = canonical_atoms(&request.required_atoms);
    if request.search_bound == 0
        || request.max_terms == 0
        || atoms.iter().any(|atom| atom.0.is_empty())
        || required.iter().any(|atom| atom.0.is_empty())
    {
        return SynthesisOutcome::Refused {
            reason: "invalid finite synthesis bound or atom".into(),
        };
    }
    let identity = synthesis_identity(&atoms, &required, request.max_terms);
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
pub fn validate_candidate(validation: ValidationRequest) -> SynthesisOutcome {
    if validation.problem_identity != validation.candidate.problem_identity
        || validation.evidence_identity.0.is_empty()
        || validation.candidate.terms.is_empty()
        || validation.candidate.identity
            != candidate_identity(
                &validation.candidate.problem_identity,
                &validation.candidate.terms,
            )
    {
        return SynthesisOutcome::Refused {
            reason: "validation does not bind the exact candidate and problem".into(),
        };
    }
    if validation.accepted {
        SynthesisOutcome::Validated {
            candidate: validation.candidate,
            proof: validation.evidence_identity,
        }
    } else {
        SynthesisOutcome::ValidationFailed {
            candidate: validation.candidate,
            reason: "independent validator rejected candidate".into(),
        }
    }
}

fn validate_hybrid(request: &HybridRequest) -> Result<(), String> {
    if request.initial_mode.0.is_empty()
        || !valid_interval(request.initial_set)
        || request.flow_delta_lower > request.flow_delta_upper
    {
        return Err("malformed initial mode, set, or flow enclosure".into());
    }
    if request.transitions.iter().any(|transition| {
        transition.from_mode.0.is_empty()
            || transition.to_mode.0.is_empty()
            || !valid_interval(transition.guard)
    }) {
        return Err("malformed mode, guard, or reset transition".into());
    }
    Ok(())
}

fn valid_interval(interval: Interval) -> bool {
    interval.lower <= interval.upper
}
fn overlaps(left: Interval, right: Interval) -> bool {
    left.lower <= right.upper && right.lower <= left.upper
}
fn join_interval(
    target: &mut BTreeMap<ModeIdentity, Interval>,
    mode: ModeIdentity,
    interval: Interval,
) {
    target
        .entry(mode)
        .and_modify(|existing| {
            existing.lower = existing.lower.min(interval.lower);
            existing.upper = existing.upper.max(interval.upper);
        })
        .or_insert(interval);
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
fn hybrid_identity(request: &HybridRequest) -> HybridModelIdentity {
    HybridModelIdentity(digest(format!("{:?}", request).as_bytes()))
}
fn synthesis_identity(
    atoms: &[SynthesisAtom],
    required: &[SynthesisAtom],
    max_terms: usize,
) -> SynthesisProblemIdentity {
    let mut bytes = b"quire-analyze/synthesis-problem/v1".to_vec();
    encode_atoms(&mut bytes, atoms);
    encode_atoms(&mut bytes, required);
    encode_text(&mut bytes, &max_terms.to_string());
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
