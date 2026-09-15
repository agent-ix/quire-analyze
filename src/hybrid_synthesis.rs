// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded hybrid reachability and canonical synthesis for FR-007 and FR-008.
//!
//! Both providers are deliberately conservative: reachability emits enclosures,
//! while synthesis emits a candidate that must be independently validated.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

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
    pub from_mode: String,
    /// Target mode identity.
    pub to_mode: String,
    /// Guard interval; overlap enables a conservative transition.
    pub guard: Interval,
    /// Additive reset applied after a guard overlap.
    pub reset_offset: i64,
}

/// A finite hybrid reachability request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridRequest {
    /// Initial exact mode identity.
    pub initial_mode: String,
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
    pub model_identity: String,
    /// Analyzed horizon.
    pub horizon: u64,
    /// Final enclosures keyed by mode identity.
    pub enclosures: Vec<ModeEnclosure>,
}

/// An enclosure for one reachable hybrid mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModeEnclosure {
    /// Mode identity.
    pub mode: String,
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
    /// A numerical operation could not produce an enclosure.
    Failed {
        /// Failure explanation.
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
    pub atoms: Vec<String>,
    /// Atoms every satisfying candidate must contain.
    pub required_atoms: Vec<String>,
    /// Maximum terms in an admissible candidate.
    pub max_terms: usize,
    /// Maximum candidates that may be inspected.
    pub search_bound: usize,
}

/// A canonical candidate, never a proof of satisfaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesisCandidate {
    /// Sorted, duplicate-free candidate terms.
    pub terms: Vec<String>,
    /// Exact search-request identity.
    pub problem_identity: String,
    /// Canonical candidate identity.
    pub identity: String,
}

/// A separate independent validation input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationRequest {
    /// Candidate being checked.
    pub candidate: SynthesisCandidate,
    /// Search problem that the independent validator received.
    pub problem_identity: String,
    /// External validator result; discovery cannot set this value.
    pub accepted: bool,
    /// Immutable validation evidence identity.
    pub evidence_identity: String,
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
        proof: String,
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
        || atoms.iter().any(String::is_empty)
        || required.iter().any(String::is_empty)
    {
        return SynthesisOutcome::Refused {
            reason: "invalid finite synthesis bound or atom".into(),
        };
    }
    let identity = synthesis_identity(&atoms, &required, request.max_terms);
    let mut inspected = 0usize;
    let max_terms = request.max_terms.min(atoms.len());
    for size in 1..=max_terms {
        let mut selections = Vec::new();
        combinations(&atoms, size, 0, &mut Vec::new(), &mut selections);
        for terms in selections {
            if inspected == request.search_bound {
                return SynthesisOutcome::Incomplete {
                    reason: "canonical candidate search bound exhausted".into(),
                };
            }
            inspected = inspected.saturating_add(1);
            if required
                .iter()
                .all(|required_atom| terms.binary_search(required_atom).is_ok())
            {
                let candidate = SynthesisCandidate {
                    identity: digest(format!("{identity}:{}", terms.join(" & ")).as_bytes()),
                    problem_identity: identity,
                    terms,
                };
                return SynthesisOutcome::Candidate(candidate);
            }
        }
    }
    SynthesisOutcome::NoCandidate
}

/// Binds an independently supplied validation result to its exact candidate.
#[must_use]
pub fn validate_candidate(validation: ValidationRequest) -> SynthesisOutcome {
    if validation.problem_identity != validation.candidate.problem_identity
        || validation.evidence_identity.is_empty()
        || validation.candidate.terms.is_empty()
        || validation.candidate.identity
            != digest(
                format!(
                    "{}:{}",
                    validation.candidate.problem_identity,
                    validation.candidate.terms.join(" & ")
                )
                .as_bytes(),
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
    if request.initial_mode.is_empty()
        || !valid_interval(request.initial_set)
        || request.flow_delta_lower > request.flow_delta_upper
    {
        return Err("malformed initial mode, set, or flow enclosure".into());
    }
    if request.transitions.iter().any(|transition| {
        transition.from_mode.is_empty()
            || transition.to_mode.is_empty()
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
fn join_interval(target: &mut BTreeMap<String, Interval>, mode: String, interval: Interval) {
    target
        .entry(mode)
        .and_modify(|existing| {
            existing.lower = existing.lower.min(interval.lower);
            existing.upper = existing.upper.max(interval.upper);
        })
        .or_insert(interval);
}
fn canonical_atoms(atoms: &[String]) -> Vec<String> {
    let mut canonical = atoms.to_vec();
    canonical.sort();
    canonical.dedup();
    canonical
}
fn combinations(
    atoms: &[String],
    remaining: usize,
    start: usize,
    partial: &mut Vec<String>,
    output: &mut Vec<Vec<String>>,
) {
    if remaining == 0 {
        output.push(partial.clone());
        return;
    }
    for index in start..=atoms.len().saturating_sub(remaining) {
        partial.push(atoms[index].clone());
        combinations(atoms, remaining - 1, index + 1, partial, output);
        partial.pop();
    }
}
fn hybrid_identity(request: &HybridRequest) -> String {
    digest(format!("{:?}", request).as_bytes())
}
fn synthesis_identity(atoms: &[String], required: &[String], max_terms: usize) -> String {
    digest(
        format!(
            "{}|{}|{max_terms}",
            atoms.join("\u{1f}"),
            required.join("\u{1f}")
        )
        .as_bytes(),
    )
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
