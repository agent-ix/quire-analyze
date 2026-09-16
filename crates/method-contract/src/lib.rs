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

macro_rules! exact_identity {
    ($($identity:ident),+ $(,)?) => {$(
        /// Exact immutable identity retained in an Analyze result envelope.
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub struct $identity(pub String);
    )+};
}

exact_identity!(
    ProviderRevision,
    AnalyzeSubjectIdentity,
    AnalyzeRunIdentity,
    AnalyzeRequestIdentity,
    AnalyzeImplementationIdentity,
    AnalyzeToolchainIdentity,
    AnalyzeOptionsIdentity,
    AnalyzeAssumptionsIdentity,
    AnalyzeBoundsIdentity,
    AnalyzeContentIdentity,
    AnalyzeResultIdentity,
);

/// Exact producer result shared by Analyze and Verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyzeResult {
    /// Shared terminal method-result contract version.
    pub contract_version: String,
    /// Revision that produced this result.
    pub provider_revision: ProviderRevision,
    /// Exact provider method.
    pub method: AnalyzeMethod,
    /// Typed terminal outcome.
    pub outcome: AnalyzeOutcome,
    /// Exact semantic subject identity.
    pub subject_identity: AnalyzeSubjectIdentity,
    /// Exact provider run identity.
    pub run_identity: AnalyzeRunIdentity,
    /// Exact resolved method request identity.
    pub request_identity: AnalyzeRequestIdentity,
    /// Exact Analyze implementation identity.
    pub implementation_identity: AnalyzeImplementationIdentity,
    /// Exact producer toolchain identity.
    pub toolchain_identity: AnalyzeToolchainIdentity,
    /// Exact resolved method-options identity.
    pub options_identity: AnalyzeOptionsIdentity,
    /// Exact resolved assumptions identity.
    pub assumptions_identity: AnalyzeAssumptionsIdentity,
    /// Exact resolved semantic-bounds identity.
    pub bounds_identity: AnalyzeBoundsIdentity,
    /// Exact immutable outcome/witness content identity.
    pub content_identity: AnalyzeContentIdentity,
    /// Immutable result/artifact identity.
    pub result_identity: AnalyzeResultIdentity,
}
