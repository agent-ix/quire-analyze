//! Shared, dependency-light outcome contract for pinned Analyze providers.

/// Version of the shared method-outcome contract.
pub const METHOD_RESULT_VERSION: &str = "quire.method-result/v1";

/// Analyze method whose terminal outcome is being retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalyzeMethod {
    /// Sound bounded hybrid reachability enclosure.
    HybridReachability,
    /// Bounded canonical synthesis search.
    CanonicalSynthesis,
}

/// Typed terminal state emitted by an Analyze method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalyzeOutcome {
    /// A bounded sound enclosure.
    Enclosure,
    /// A candidate requiring independent validation.
    Candidate,
    /// A candidate with retained independent-validation evidence.
    ValidatedCandidate,
    /// An independent validator rejected the candidate.
    ValidationRejected,
    /// The finite candidate space was exhausted.
    NoCandidate,
    /// The provider reached its declared bound without a conclusion.
    Incomplete,
    /// The provider failed without a semantic conclusion.
    Failed,
    /// The provider refused malformed input.
    Refused,
}
