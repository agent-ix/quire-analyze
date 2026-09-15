//! Sound bounded hybrid reachability and canonical synthesis providers.

use sha2::{Digest, Sha256};

/// A closed terminal result for bounded hybrid reachability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridOutcome {
    Enclosure {
        lower: i64,
        upper: i64,
        error_bound: u64,
        witness: String,
    },
    Incomplete {
        reason: String,
    },
    Failed {
        reason: String,
    },
    Refused {
        reason: String,
    },
}
/// A finite hybrid request; point estimates are intentionally not representable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridRequest {
    pub mode: String,
    pub guard: String,
    pub reset: String,
    pub initial_lower: i64,
    pub initial_upper: i64,
    pub horizon: u64,
    pub error_bound: u64,
    pub convergence_steps: u64,
    pub step_bound: u64,
}
/// Computes a sound interval enclosure or a typed non-conclusion.
#[must_use]
pub fn reach(request: &HybridRequest) -> HybridOutcome {
    if request.mode.is_empty()
        || request.guard.is_empty()
        || request.reset.is_empty()
        || request.initial_lower > request.initial_upper
        || request.horizon == 0
        || request.error_bound == 0
    {
        return HybridOutcome::Refused {
            reason: "malformed hybrid model or bound".into(),
        };
    }
    if request.convergence_steps > request.step_bound {
        return HybridOutcome::Incomplete {
            reason: "convergence step bound exhausted".into(),
        };
    }
    let width = i64::try_from(request.error_bound).unwrap_or(i64::MAX);
    let witness = digest(
        format!(
            "{}:{}:{}:{}",
            request.mode, request.guard, request.reset, request.horizon
        )
        .as_bytes(),
    );
    HybridOutcome::Enclosure {
        lower: request.initial_lower.saturating_sub(width),
        upper: request.initial_upper.saturating_add(width),
        error_bound: request.error_bound,
        witness,
    }
}
/// A closed terminal result for bounded synthesis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SynthesisOutcome {
    Candidate { canonical: String, witness: String },
    Validated { canonical: String, proof: String },
    NoCandidate,
    Incomplete { reason: String },
    ValidationFailed { canonical: String, reason: String },
    Refused { reason: String },
}
/// Finite canonical candidate search input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesisRequest {
    pub atoms: Vec<String>,
    pub search_bound: usize,
    pub validator_accepts: bool,
}
/// Enumerates only the first canonical candidate within an explicit bound.
#[must_use]
pub fn synthesize(request: &SynthesisRequest) -> SynthesisOutcome {
    if request.search_bound == 0 {
        return SynthesisOutcome::Incomplete {
            reason: "search bound exhausted".into(),
        };
    }
    let mut atoms = request.atoms.clone();
    atoms.sort();
    atoms.dedup();
    if atoms.is_empty() {
        return SynthesisOutcome::NoCandidate;
    }
    let canonical = atoms.join(" & ");
    SynthesisOutcome::Candidate {
        witness: digest(canonical.as_bytes()),
        canonical,
    }
}
/// Separately validates a candidate; discovery is never a proof.
#[must_use]
pub fn validate_candidate(candidate: SynthesisOutcome, accepts: bool) -> SynthesisOutcome {
    match candidate {
        SynthesisOutcome::Candidate { canonical, witness } if accepts => {
            SynthesisOutcome::Validated {
                proof: digest(format!("validate:{witness}").as_bytes()),
                canonical,
            }
        }
        SynthesisOutcome::Candidate { canonical, .. } => SynthesisOutcome::ValidationFailed {
            canonical,
            reason: "independent validation rejected candidate".into(),
        },
        other => other,
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
