use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::{
    smt::{
        lower_analysis_statements, AnalysisStatement, ReplayAssertion, ReplayAssignment,
        ReplayFault, ANALYSIS_MODEL_PROFILE,
    },
    AnalysisDigest, AnalysisKind, AssertionMap, AssertionPolarity, BindingGroup, LoweringError,
    QueryBundle, SolverOutcome, SolverRecord, StatementInput, StatementRole,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalysisRequestErrorCode {
    EmptyGroup,
    DuplicateStatement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisRequestError {
    code: AnalysisRequestErrorCode,
    path: String,
    message: String,
}

impl AnalysisRequestError {
    fn new(
        code: AnalysisRequestErrorCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }

    pub const fn code(&self) -> AnalysisRequestErrorCode {
        self.code
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AnalysisRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid analysis request at {}: {}",
            self.path, self.message
        )
    }
}

impl std::error::Error for AnalysisRequestError {}

#[derive(Clone, Debug)]
struct RequestStatement {
    statement: StatementInput,
    role: StatementRole,
    polarity: AssertionPolarity,
}

#[derive(Clone, Debug)]
pub struct AnalysisRequest {
    kind: AnalysisKind,
    statements: Vec<RequestStatement>,
    bindings: Vec<BindingGroup>,
}

impl AnalysisRequest {
    pub fn consistency(
        assumptions: Vec<StatementInput>,
        selected: Vec<StatementInput>,
        bindings: Vec<BindingGroup>,
    ) -> Result<Self, Vec<AnalysisRequestError>> {
        if selected.is_empty() {
            return Err(vec![AnalysisRequestError::new(
                AnalysisRequestErrorCode::EmptyGroup,
                "selected",
                "consistency requires at least one selected statement",
            )]);
        }
        Self::from_groups(
            AnalysisKind::Consistency,
            [
                (
                    assumptions,
                    StatementRole::Assumption,
                    AssertionPolarity::Positive,
                ),
                (
                    selected,
                    StatementRole::Selected,
                    AssertionPolarity::Positive,
                ),
            ],
            bindings,
        )
    }

    pub fn contradiction(
        assumptions: Vec<StatementInput>,
        left: Vec<StatementInput>,
        right: Vec<StatementInput>,
        bindings: Vec<BindingGroup>,
    ) -> Result<Self, Vec<AnalysisRequestError>> {
        let mut errors = Vec::new();
        for (path, statements) in [("left", &left), ("right", &right)] {
            if statements.is_empty() {
                errors.push(AnalysisRequestError::new(
                    AnalysisRequestErrorCode::EmptyGroup,
                    path,
                    format!("contradiction requires at least one {path} statement"),
                ));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Self::from_groups(
            AnalysisKind::Contradiction,
            [
                (
                    assumptions,
                    StatementRole::Assumption,
                    AssertionPolarity::Positive,
                ),
                (left, StatementRole::Left, AssertionPolarity::Positive),
                (right, StatementRole::Right, AssertionPolarity::Positive),
            ],
            bindings,
        )
    }

    pub fn implication(
        assumptions: Vec<StatementInput>,
        antecedents: Vec<StatementInput>,
        consequent: StatementInput,
        bindings: Vec<BindingGroup>,
    ) -> Result<Self, Vec<AnalysisRequestError>> {
        require_nonempty("antecedents", &antecedents, "implication")?;
        Self::from_groups(
            AnalysisKind::Implication,
            [
                (
                    assumptions,
                    StatementRole::Assumption,
                    AssertionPolarity::Positive,
                ),
                (
                    antecedents,
                    StatementRole::Antecedent,
                    AssertionPolarity::Positive,
                ),
                (
                    vec![consequent],
                    StatementRole::Consequent,
                    AssertionPolarity::Negated,
                ),
            ],
            bindings,
        )
    }

    pub fn redundancy(
        assumptions: Vec<StatementInput>,
        peers: Vec<StatementInput>,
        candidate: StatementInput,
        bindings: Vec<BindingGroup>,
    ) -> Result<Self, Vec<AnalysisRequestError>> {
        require_nonempty("peers", &peers, "redundancy")?;
        Self::from_groups(
            AnalysisKind::Redundancy,
            [
                (
                    assumptions,
                    StatementRole::Assumption,
                    AssertionPolarity::Positive,
                ),
                (peers, StatementRole::Peer, AssertionPolarity::Positive),
                (
                    vec![candidate],
                    StatementRole::Candidate,
                    AssertionPolarity::Negated,
                ),
            ],
            bindings,
        )
    }

    pub fn dead_antecedent(
        assumptions: Vec<StatementInput>,
        antecedent: StatementInput,
        bindings: Vec<BindingGroup>,
    ) -> Result<Self, Vec<AnalysisRequestError>> {
        Self::from_groups(
            AnalysisKind::DeadAntecedent,
            [
                (
                    assumptions,
                    StatementRole::Assumption,
                    AssertionPolarity::Positive,
                ),
                (
                    vec![antecedent],
                    StatementRole::Antecedent,
                    AssertionPolarity::Positive,
                ),
            ],
            bindings,
        )
    }

    fn from_groups<const N: usize>(
        kind: AnalysisKind,
        groups: [(Vec<StatementInput>, StatementRole, AssertionPolarity); N],
        bindings: Vec<BindingGroup>,
    ) -> Result<Self, Vec<AnalysisRequestError>> {
        let mut statements = Vec::new();
        let mut identities = BTreeSet::new();
        let mut errors = Vec::new();
        for (group, role, polarity) in groups {
            for statement in group {
                let identity = statement_identity(&statement);
                if !identities.insert(identity.clone()) {
                    errors.push(AnalysisRequestError::new(
                        AnalysisRequestErrorCode::DuplicateStatement,
                        role.as_str(),
                        format!("statement {identity} occurs in more than one request role"),
                    ));
                }
                statements.push(RequestStatement {
                    statement,
                    role,
                    polarity,
                });
            }
        }
        if errors.is_empty() {
            Ok(Self {
                kind,
                statements,
                bindings,
            })
        } else {
            errors.sort_by(|left, right| {
                (&left.path, &left.message).cmp(&(&right.path, &right.message))
            });
            Err(errors)
        }
    }

    pub const fn kind(&self) -> AnalysisKind {
        self.kind
    }

    pub fn bindings(&self) -> &[BindingGroup] {
        &self.bindings
    }

    pub fn statement_count(&self) -> usize {
        self.statements.len()
    }
}

fn require_nonempty(
    path: &str,
    statements: &[StatementInput],
    kind: &str,
) -> Result<(), Vec<AnalysisRequestError>> {
    if statements.is_empty() {
        Err(vec![AnalysisRequestError::new(
            AnalysisRequestErrorCode::EmptyGroup,
            path,
            format!("{kind} requires at least one {path} statement"),
        )])
    } else {
        Ok(())
    }
}

fn statement_identity(statement: &StatementInput) -> String {
    let clause = statement.clause();
    format!(
        "{}|{}|{}|{}|{}",
        clause.requirement().package(),
        clause.requirement().requirement(),
        clause.requirement().revision().get(),
        clause.clause(),
        statement.clause_digest()
    )
}

pub fn lower_analysis_request(
    request: &AnalysisRequest,
) -> Result<QueryBundle, Vec<LoweringError>> {
    let statements = request
        .statements
        .iter()
        .map(|statement| AnalysisStatement {
            statement: &statement.statement,
            role: statement.role,
            polarity: statement.polarity,
        })
        .collect::<Vec<_>>();
    lower_analysis_statements(request.kind, &statements, &request.bindings)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalysisStatus {
    Satisfied,
    Refuted,
    Unknown,
    Unsupported,
    Timeout,
    ToolError,
}

impl AnalysisStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Refuted => "refuted",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
            Self::Timeout => "timeout",
            Self::ToolError => "tool-error",
        }
    }

    pub const fn is_conclusive(self) -> bool {
        matches!(self, Self::Satisfied | Self::Refuted)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplanationState {
    NotApplicable,
    Incomplete,
    Verified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelPurpose {
    Shared,
    Common,
    Counterexample,
    Distinguishing,
    Activation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappedBooleanValue {
    symbol: String,
    value: bool,
    origins: Vec<String>,
    binding_group: Option<String>,
}

impl MappedBooleanValue {
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub const fn value(&self) -> bool {
        self.value
    }

    pub fn origins(&self) -> &[String] {
        &self.origins
    }

    pub fn binding_group(&self) -> Option<&str> {
        self.binding_group.as_deref()
    }
}

/// An integer value of a verified model, mapped to the requirement values it stands for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappedIntegerValue {
    symbol: String,
    value: i128,
    origins: Vec<String>,
    binding_group: Option<String>,
}

impl MappedIntegerValue {
    /// The query symbol the solver assigned the value to.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// The replay-verified integer value.
    pub const fn value(&self) -> i128 {
        self.value
    }

    /// Every complete origin identity of the symbol from the variable map.
    pub fn origins(&self) -> &[String] {
        &self.origins
    }

    /// The explicit binding group of the symbol, if it has one.
    pub fn binding_group(&self) -> Option<&str> {
        self.binding_group.as_deref()
    }
}

/// The closed set of reasons a returned model fails replay, in precedence order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReplayFailureReason {
    ValueMalformed,
    SymbolRepeated,
    SymbolUnknown,
    SortMismatch,
    SymbolMissing,
    ValueOutOfDomain,
    EvaluationOverflow,
    AssertionFalse,
}

impl ReplayFailureReason {
    /// The reason's stable snake_case code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValueMalformed => "value_malformed",
            Self::SymbolRepeated => "symbol_repeated",
            Self::SymbolUnknown => "symbol_unknown",
            Self::SortMismatch => "sort_mismatch",
            Self::SymbolMissing => "symbol_missing",
            Self::ValueOutOfDomain => "value_out_of_domain",
            Self::EvaluationOverflow => "evaluation_overflow",
            Self::AssertionFalse => "assertion_false",
        }
    }
}

/// A model that did not verify: one reason and the symbol or assertion it names, if any.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayFailure {
    reason: ReplayFailureReason,
    subject: Option<String>,
}

impl ReplayFailure {
    fn new(reason: ReplayFailureReason, subject: Option<String>) -> Self {
        Self { reason, subject }
    }

    pub const fn reason(&self) -> ReplayFailureReason {
        self.reason
    }

    /// The first symbol, or the first assertion, that exhibits the reason.
    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }
}

impl fmt::Display for ReplayFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.subject {
            Some(subject) => write!(formatter, "{}: {subject}", self.reason.as_str()),
            None => formatter.write_str(self.reason.as_str()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedBooleanModel {
    purpose: ModelPurpose,
    values: Vec<MappedBooleanValue>,
    integer_values: Vec<MappedIntegerValue>,
    replayed_assertions: Vec<String>,
}

impl VerifiedBooleanModel {
    pub const fn purpose(&self) -> ModelPurpose {
        self.purpose
    }

    pub fn values(&self) -> &[MappedBooleanValue] {
        &self.values
    }

    /// The verified integer values, in query symbol order.
    pub fn integer_values(&self) -> &[MappedIntegerValue] {
        &self.integer_values
    }

    pub fn replayed_assertions(&self) -> &[String] {
        &self.replayed_assertions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisConclusion {
    kind: Option<AnalysisKind>,
    status: AnalysisStatus,
    analysis_model_profile: &'static str,
    encoding_profile: &'static str,
    logic: &'static str,
    request_digest: AnalysisDigest,
    query_digest: AnalysisDigest,
    binding_set_digest: AnalysisDigest,
    assertions: Vec<AssertionMap>,
    solver: SolverRecord,
    explanation: ExplanationState,
    verified_model: Option<VerifiedBooleanModel>,
    replay_failure: Option<ReplayFailure>,
    diagnostic: Option<String>,
}

impl AnalysisConclusion {
    pub const fn kind(&self) -> Option<AnalysisKind> {
        self.kind
    }

    pub const fn status(&self) -> AnalysisStatus {
        self.status
    }

    pub const fn analysis_model_profile(&self) -> &'static str {
        self.analysis_model_profile
    }

    pub const fn encoding_profile(&self) -> &'static str {
        self.encoding_profile
    }

    pub const fn logic(&self) -> &'static str {
        self.logic
    }

    pub const fn request_digest(&self) -> AnalysisDigest {
        self.request_digest
    }

    pub const fn query_digest(&self) -> AnalysisDigest {
        self.query_digest
    }

    pub const fn binding_set_digest(&self) -> AnalysisDigest {
        self.binding_set_digest
    }

    pub fn assertions(&self) -> &[AssertionMap] {
        &self.assertions
    }

    pub const fn solver(&self) -> &SolverRecord {
        &self.solver
    }

    pub const fn explanation(&self) -> ExplanationState {
        self.explanation
    }

    pub fn verified_model(&self) -> Option<&VerifiedBooleanModel> {
        self.verified_model.as_ref()
    }

    /// Why the returned model failed replay; absent when the model verified or none was returned.
    pub fn replay_failure(&self) -> Option<&ReplayFailure> {
        self.replay_failure.as_ref()
    }

    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    pub const fn is_conclusive(&self) -> bool {
        self.status.is_conclusive()
    }
}

pub fn classify_analysis(query: &QueryBundle, solver: &SolverRecord) -> AnalysisConclusion {
    let kind = query.analysis_kind();
    let mut conclusion = AnalysisConclusion {
        kind,
        status: AnalysisStatus::ToolError,
        analysis_model_profile: ANALYSIS_MODEL_PROFILE,
        encoding_profile: query.profile(),
        logic: query.logic(),
        request_digest: query.analysis_request_digest(),
        query_digest: query.query_digest(),
        binding_set_digest: query.binding_set_digest(),
        assertions: query.assertions().to_vec(),
        solver: solver.clone(),
        explanation: ExplanationState::NotApplicable,
        verified_model: None,
        replay_failure: None,
        diagnostic: None,
    };
    let Some(kind) = kind else {
        conclusion.diagnostic = Some("query is not an analysis request".to_owned());
        return conclusion;
    };
    if solver.query_digest() != query.query_digest().to_string() {
        conclusion.diagnostic =
            Some("solver record query identity differs from request".to_owned());
        return conclusion;
    }

    conclusion.status = status_for(kind, solver.outcome());

    if solver.outcome() == SolverOutcome::Sat {
        let purpose = match kind {
            AnalysisKind::Consistency => ModelPurpose::Shared,
            AnalysisKind::Contradiction => ModelPurpose::Common,
            AnalysisKind::Implication => ModelPurpose::Counterexample,
            AnalysisKind::Redundancy => ModelPurpose::Distinguishing,
            AnalysisKind::DeadAntecedent => ModelPurpose::Activation,
        };
        match verify_model(query, solver.model(), purpose) {
            Ok(model) => {
                conclusion.explanation = ExplanationState::Verified;
                conclusion.verified_model = Some(model);
            }
            Err(error) => {
                conclusion.explanation = ExplanationState::Incomplete;
                conclusion.diagnostic = Some(error.to_string());
                conclusion.replay_failure = Some(error);
            }
        }
    }
    conclusion
}

fn status_for(kind: AnalysisKind, outcome: SolverOutcome) -> AnalysisStatus {
    match outcome {
        SolverOutcome::Sat if kind == AnalysisKind::Consistency => AnalysisStatus::Satisfied,
        SolverOutcome::Sat => AnalysisStatus::Refuted,
        SolverOutcome::Unsat if kind == AnalysisKind::Consistency => AnalysisStatus::Refuted,
        SolverOutcome::Unsat => AnalysisStatus::Satisfied,
        SolverOutcome::Unknown => AnalysisStatus::Unknown,
        SolverOutcome::TimedOut | SolverOutcome::Cancelled => AnalysisStatus::Timeout,
        SolverOutcome::UnsupportedPlatform => AnalysisStatus::Unsupported,
        _ => AnalysisStatus::ToolError,
    }
}

fn verify_model(
    query: &QueryBundle,
    bytes: &[u8],
    purpose: ModelPurpose,
) -> Result<VerifiedBooleanModel, ReplayFailure> {
    if bytes.is_empty() {
        return Err(malformed_model());
    }
    let assertion_aliases = query
        .assertions()
        .iter()
        .map(|assertion| assertion.name.as_str())
        .collect::<BTreeSet<_>>();
    let ranges = query.integer_ranges();
    // A symbol with no declared range is read as Bool. Enum-sorted symbols (VER-87 owns their
    // replay) have no range either, so they must not reach this point as Bool.
    let declared = query
        .variables()
        .iter()
        .map(|variable| {
            let sort = if ranges.contains_key(&variable.symbol) {
                Sort::Int
            } else {
                Sort::Bool
            };
            (variable.symbol.as_str(), sort)
        })
        .collect::<BTreeMap<_, _>>();
    let parsed = parse_model(bytes, &declared, &assertion_aliases)?;
    if let Some(failure) = parsed.first_defect() {
        return Err(failure);
    }
    if let Some(missing) = declared
        .keys()
        .find(|symbol| !parsed.defined.contains(**symbol))
    {
        return Err(ReplayFailure::new(
            ReplayFailureReason::SymbolMissing,
            Some((*missing).to_owned()),
        ));
    }
    let out_of_range = parsed
        .assignment
        .integers
        .iter()
        .find_map(|(symbol, value)| {
            let (minimum, maximum) = ranges.get(symbol)?;
            (value < minimum || value > maximum).then_some(symbol)
        });
    if let Some(symbol) = [parsed.out_of_domain.as_ref(), out_of_range]
        .into_iter()
        .flatten()
        .min()
    {
        return Err(ReplayFailure::new(
            ReplayFailureReason::ValueOutOfDomain,
            Some(symbol.clone()),
        ));
    }
    let replayed_assertions = replay_assertions(query.replay_assertions(), &parsed.assignment)?;
    let values = query
        .variables()
        .iter()
        .filter_map(|variable| {
            Some(MappedBooleanValue {
                symbol: variable.symbol.clone(),
                value: *parsed.assignment.booleans.get(&variable.symbol)?,
                origins: variable.origins.clone(),
                binding_group: variable.binding_group.clone(),
            })
        })
        .collect();
    let integer_values = query
        .variables()
        .iter()
        .filter_map(|variable| {
            Some(MappedIntegerValue {
                symbol: variable.symbol.clone(),
                value: *parsed.assignment.integers.get(&variable.symbol)?,
                origins: variable.origins.clone(),
                binding_group: variable.binding_group.clone(),
            })
        })
        .collect();
    Ok(VerifiedBooleanModel {
        purpose,
        values,
        integer_values,
        replayed_assertions,
    })
}

/// Evaluates every assertion in order on the typed expressions. The first assertion that does
/// not evaluate true fails the replay, with the reason that assertion has.
fn replay_assertions(
    assertions: &[ReplayAssertion],
    assignment: &ReplayAssignment,
) -> Result<Vec<String>, ReplayFailure> {
    let mut replayed = Vec::with_capacity(assertions.len());
    for assertion in assertions {
        let reason = match assertion.expression.evaluate(assignment) {
            Ok(true) => {
                replayed.push(assertion.name.clone());
                continue;
            }
            Ok(false) | Err(ReplayFault::Unassigned) => ReplayFailureReason::AssertionFalse,
            Err(ReplayFault::Overflow) => ReplayFailureReason::EvaluationOverflow,
        };
        return Err(ReplayFailure::new(reason, Some(assertion.name.clone())));
    }
    Ok(replayed)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Sort {
    Bool,
    Int,
}

/// Longest accepted integer numeral, in characters.
const MAX_NUMERAL_LENGTH: usize = 1024;

/// The values read from a model and the first offending symbol per defect kind.
#[derive(Default)]
struct ParsedModel {
    assignment: ReplayAssignment,
    /// Declared symbols the model defined, whatever their value.
    defined: BTreeSet<String>,
    malformed: Option<String>,
    repeated: Option<String>,
    unknown: Option<String>,
    sort_mismatch: Option<String>,
    out_of_domain: Option<String>,
}

impl ParsedModel {
    /// The highest-precedence parse defect, naming the first symbol in ascending order.
    fn first_defect(&self) -> Option<ReplayFailure> {
        [
            (ReplayFailureReason::ValueMalformed, &self.malformed),
            (ReplayFailureReason::SymbolRepeated, &self.repeated),
            (ReplayFailureReason::SymbolUnknown, &self.unknown),
            (ReplayFailureReason::SortMismatch, &self.sort_mismatch),
        ]
        .into_iter()
        .find_map(|(reason, symbol)| {
            symbol
                .as_ref()
                .map(|symbol| ReplayFailure::new(reason, Some(symbol.clone())))
        })
    }
}

/// Keeps the lowest symbol seen for a defect kind.
fn note(slot: &mut Option<String>, symbol: &str) {
    if slot.as_deref().is_none_or(|held| symbol < held) {
        *slot = Some(symbol.to_owned());
    }
}

enum ModelValue {
    Boolean(bool),
    Integer(i128),
    /// A well-formed numeral beyond the 128-bit signed range.
    OutOfDomain,
    Malformed,
}

fn malformed_model() -> ReplayFailure {
    ReplayFailure::new(ReplayFailureReason::ValueMalformed, None)
}

/// Parses a Z3 (`(model (define-fun ...) ...)`) or cvc5 (`((define-fun ...) ...)`) model.
/// A model whose shape cannot be read at all is `value_malformed`; every other defect is
/// recorded so the caller can apply the reason precedence.
fn parse_model(
    bytes: &[u8],
    declared: &BTreeMap<&str, Sort>,
    assertion_aliases: &BTreeSet<&str>,
) -> Result<ParsedModel, ReplayFailure> {
    let tokens = tokenize_model(bytes).map_err(|_| malformed_model())?;
    let shape = |result: Result<(), String>| result.map_err(|_| malformed_model());
    let mut cursor = 0;
    shape(expect_token(&tokens, &mut cursor, &ModelToken::Open))?;
    if matches!(tokens.get(cursor), Some(ModelToken::Atom(atom)) if atom == "model") {
        cursor += 1;
    }
    let mut parsed = ParsedModel::default();
    while matches!(tokens.get(cursor), Some(ModelToken::Open)) {
        shape(expect_token(&tokens, &mut cursor, &ModelToken::Open))?;
        shape(expect_atom(&tokens, &mut cursor, "define-fun"))?;
        let symbol = take_atom(&tokens, &mut cursor).map_err(|_| malformed_model())?;
        shape(expect_token(&tokens, &mut cursor, &ModelToken::Open))?;
        shape(expect_token(&tokens, &mut cursor, &ModelToken::Close))?;
        let sort = take_atom(&tokens, &mut cursor).map_err(|_| malformed_model())?;
        if !declared.contains_key(symbol.as_str()) && assertion_aliases.contains(symbol.as_str()) {
            shape(skip_model_term(&tokens, &mut cursor))?;
        } else {
            let value = take_model_value(&tokens, &mut cursor)?;
            record_value(&mut parsed, declared, &symbol, &sort, value);
        }
        shape(expect_token(&tokens, &mut cursor, &ModelToken::Close))?;
    }
    shape(expect_token(&tokens, &mut cursor, &ModelToken::Close))?;
    if cursor == tokens.len() {
        Ok(parsed)
    } else {
        Err(malformed_model())
    }
}

fn record_value(
    parsed: &mut ParsedModel,
    declared: &BTreeMap<&str, Sort>,
    symbol: &str,
    sort: &str,
    value: ModelValue,
) {
    let model_sort = match sort {
        "Bool" => Sort::Bool,
        "Int" => Sort::Int,
        _ => return note(&mut parsed.malformed, symbol),
    };
    // The value is read first: a malformed numeral outranks every symbol-level defect.
    if matches!(value, ModelValue::Malformed) {
        return note(&mut parsed.malformed, symbol);
    }
    let Some(&declared_sort) = declared.get(symbol) else {
        return note(&mut parsed.unknown, symbol);
    };
    if !parsed.defined.insert(symbol.to_owned()) {
        return note(&mut parsed.repeated, symbol);
    }
    if declared_sort != model_sort {
        return note(&mut parsed.sort_mismatch, symbol);
    }
    match (model_sort, value) {
        (_, ModelValue::Malformed) => unreachable!("malformed values return above"),
        (Sort::Bool, ModelValue::Boolean(value)) => {
            parsed.assignment.booleans.insert(symbol.to_owned(), value);
        }
        (Sort::Int, ModelValue::Integer(value)) => {
            parsed.assignment.integers.insert(symbol.to_owned(), value);
        }
        (Sort::Int, ModelValue::OutOfDomain) => note(&mut parsed.out_of_domain, symbol),
        _ => note(&mut parsed.sort_mismatch, symbol),
    }
}

/// Reads one model value: `true`, `false`, a numeral, or the negation `(- numeral)` of one.
/// Any other term is consumed and reported as malformed.
fn take_model_value(
    tokens: &[ModelToken],
    cursor: &mut usize,
) -> Result<ModelValue, ReplayFailure> {
    let negation = [ModelToken::Open, ModelToken::Atom("-".to_owned())];
    if tokens.get(*cursor..*cursor + 2) == Some(negation.as_slice()) {
        if let (Some(ModelToken::Atom(digits)), Some(ModelToken::Close)) =
            (tokens.get(*cursor + 2), tokens.get(*cursor + 3))
        {
            *cursor += 4;
            return Ok(parse_numeral(digits, true));
        }
    }
    let value = match tokens.get(*cursor) {
        Some(ModelToken::Atom(atom)) => match atom.as_str() {
            "true" => ModelValue::Boolean(true),
            "false" => ModelValue::Boolean(false),
            digits => parse_numeral(digits, false),
        },
        _ => ModelValue::Malformed,
    };
    skip_model_term(tokens, cursor).map_err(|_| malformed_model())?;
    Ok(value)
}

/// A numeral is ASCII digits with no leading zero. The length bound is checked before any
/// conversion; a value outside the 128-bit signed range is out of domain, not malformed.
fn parse_numeral(text: &str, negative: bool) -> ModelValue {
    if text.is_empty()
        || text.len() > MAX_NUMERAL_LENGTH
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return ModelValue::Malformed;
    }
    let Ok(magnitude) = text.parse::<u128>() else {
        return ModelValue::OutOfDomain;
    };
    let value = if negative {
        0i128.checked_sub_unsigned(magnitude)
    } else {
        i128::try_from(magnitude).ok()
    };
    value.map_or(ModelValue::OutOfDomain, ModelValue::Integer)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ModelToken {
    Open,
    Close,
    Atom(String),
}

fn skip_model_term(tokens: &[ModelToken], cursor: &mut usize) -> Result<(), String> {
    match tokens.get(*cursor) {
        Some(ModelToken::Atom(_)) => {
            *cursor += 1;
            Ok(())
        }
        Some(ModelToken::Open) => {
            *cursor += 1;
            while !matches!(tokens.get(*cursor), Some(ModelToken::Close)) {
                skip_model_term(tokens, cursor)?;
            }
            *cursor += 1;
            Ok(())
        }
        _ => Err("model contains an invalid Boolean term".to_owned()),
    }
}

fn tokenize_model(bytes: &[u8]) -> Result<Vec<ModelToken>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "model is not UTF-8".to_owned())?;
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let bytes = text.as_bytes();
    while cursor < bytes.len() {
        match bytes[cursor] {
            byte if byte.is_ascii_whitespace() => cursor += 1,
            b';' => {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            b'(' => {
                tokens.push(ModelToken::Open);
                cursor += 1;
            }
            b')' => {
                tokens.push(ModelToken::Close);
                cursor += 1;
            }
            b'"' | b'|' => return Err("quoted model atoms are outside Boolean v1".to_owned()),
            _ => {
                let start = cursor;
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && !matches!(bytes[cursor], b'(' | b')' | b';')
                {
                    if !bytes[cursor].is_ascii() {
                        return Err("model atom is not ASCII".to_owned());
                    }
                    cursor += 1;
                }
                tokens.push(ModelToken::Atom(text[start..cursor].to_owned()));
            }
        }
    }
    Ok(tokens)
}

fn expect_token(
    tokens: &[ModelToken],
    cursor: &mut usize,
    expected: &ModelToken,
) -> Result<(), String> {
    if tokens.get(*cursor) == Some(expected) {
        *cursor += 1;
        Ok(())
    } else {
        Err("model has an invalid Boolean-v1 shape".to_owned())
    }
}

fn expect_atom(tokens: &[ModelToken], cursor: &mut usize, expected: &str) -> Result<(), String> {
    if matches!(tokens.get(*cursor), Some(ModelToken::Atom(atom)) if atom == expected) {
        *cursor += 1;
        Ok(())
    } else {
        Err(format!("model expected atom {expected}"))
    }
}

fn take_atom(tokens: &[ModelToken], cursor: &mut usize) -> Result<String, String> {
    match tokens.get(*cursor) {
        Some(ModelToken::Atom(atom)) => {
            *cursor += 1;
            Ok(atom.clone())
        }
        _ => Err("model expected an atom".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_contract_model::ComparisonOperator;

    use super::*;
    use crate::smt::{ReplayExpression, ReplayTerm};

    /// A query for `x >= 1` over `x` declared in `[0, 10]`, built from typed contract nodes.
    fn lower_positive_query() -> QueryBundle {
        use quire_contract_model::{
            AnchorName, Clause, ClauseId, ClauseKind, ContractPackage, DeclarationEnvironment,
            ExecutionPoint, Expression, ExpressionKind, IntegerDomain, IntegerType, OverflowPolicy,
            PackageId, Requirement, RequirementId, RequirementRef, RequirementRevision,
            SchemaVersion, SourceDocumentId, SourceIdentity, SourceLocation, SourceRevision,
            SourceSpan, StateObservation, SymbolName, TypedExpression, ValueDeclaration,
            ValueDeclarationKind, ValueType,
        };
        let source = SourceIdentity::new(
            SourceDocumentId::new("replay-unit").expect("source id"),
            SourceRevision::new(1).expect("source revision"),
        );
        let span = SourceSpan::new(
            SourceLocation::new(source.clone(), 1, 1, 0).expect("start"),
            SourceLocation::new(source, 1, 2, 1).expect("end"),
        )
        .expect("span");
        let integer = IntegerType::new(IntegerDomain::Signed, 0, 10, OverflowPolicy::Reject)
            .expect("integer type");
        let package_id = PackageId::new("agent-ix/replay-unit").expect("package");
        let requirement_id = RequirementId::new("REQ-replay").expect("requirement");
        let revision = RequirementRevision::new(1).expect("revision");
        let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
        let name = SymbolName::new("x").expect("symbol");
        let environment = DeclarationEnvironment::new(
            owner,
            vec![],
            vec![ValueDeclaration::new(
                name.clone(),
                ValueDeclarationKind::Input,
                ValueType::Integer {
                    value: integer.clone(),
                },
                span.clone(),
            )],
            vec![],
        )
        .expect("environment");
        let point = ExecutionPoint::Pre {
            operation: AnchorName::new("check").expect("anchor").into(),
        };
        let node = |kind| Expression::new(kind, span.clone());
        let expression = node(ExpressionKind::Compare {
            operator: ComparisonOperator::GreaterEqual,
            left: Box::new(node(ExpressionKind::ValueReference {
                name,
                observation: StateObservation::Current,
            })),
            right: Box::new(node(ExpressionKind::IntegerLiteral {
                value: 1,
                value_type: integer,
            })),
        });
        let checked = environment
            .check_expression(&expression, &ValueType::Boolean, &point, true)
            .expect("typed expression");
        let clause = Clause::new(
            ClauseId::new("C-replay").expect("clause"),
            ClauseKind::Assertion,
            Some(point),
            span.clone(),
            checked,
        )
        .expect("clause");
        let requirement = Requirement::<TypedExpression>::new(
            &package_id,
            requirement_id,
            revision,
            span.clone(),
            vec![clause],
        )
        .expect("requirement");
        let package = ContractPackage::new(
            package_id,
            SchemaVersion::V1_1,
            span.source().clone(),
            vec![requirement],
        )
        .expect("package");
        let statement = StatementInput::from_clause(
            &package,
            &package.requirements()[0],
            &package.requirements()[0].clauses()[0],
            environment,
        )
        .expect("statement");
        crate::lower_boolean_statements(&[statement], &[]).expect("query")
    }

    /// FR-011-AC-9: replay reads the typed expressions and the parsed assignment, never the
    /// query text, so replacing the text moves no verdict.
    #[trace("TC-017", "FR-011-AC-9")]
    #[test]
    fn replay_verdict_ignores_the_query_text() {
        let query = lower_positive_query();
        let symbol = query.variables()[0].symbol.clone();
        let mut mutated = query.clone();
        mutated.set_query_text("(assert false)\n(check-sat)\n");
        assert_ne!(mutated.query(), query.query());
        for (value, verifies) in [("5", true), ("0", false)] {
            let model = format!("(model (define-fun {symbol} () Int {value}))");
            for bundle in [&query, &mutated] {
                let verdict = verify_model(bundle, model.as_bytes(), ModelPurpose::Shared);
                assert_eq!(verdict.is_ok(), verifies, "{value}");
            }
        }
    }

    fn declared<'a>(entries: &[(&'a str, Sort)]) -> BTreeMap<&'a str, Sort> {
        entries.iter().copied().collect()
    }

    fn parse(text: &str, entries: &[(&str, Sort)]) -> Result<ParsedModel, ReplayFailure> {
        let aliases = BTreeSet::from(["assertion"]);
        parse_model(text.as_bytes(), &declared(entries), &aliases)
    }

    /// The first parse defect, if any. `Ok(None)` means the model parsed cleanly, with an
    /// out-of-domain numeral reported as such.
    fn defect(text: &str, entries: &[(&str, Sort)]) -> Option<ReplayFailure> {
        match parse(text, entries) {
            Ok(parsed) => parsed.first_defect().or_else(|| {
                parsed.out_of_domain.clone().map(|symbol| {
                    ReplayFailure::new(ReplayFailureReason::ValueOutOfDomain, Some(symbol))
                })
            }),
            Err(failure) => Some(failure),
        }
    }

    fn reason_of(text: &str, entries: &[(&str, Sort)]) -> ReplayFailureReason {
        defect(text, entries).expect("a defect").reason()
    }

    /// FR-011-AC-2: both solver spellings, negative values, mixed sorts and echoed aliases.
    #[trace("TC-017", "FR-011-AC-2")]
    #[test]
    fn model_parser_reads_both_solver_spellings() {
        let entries = [("b", Sort::Bool), ("x", Sort::Int), ("y", Sort::Int)];
        for text in [
            "(model (define-fun b () Bool true) (define-fun x () Int 7) \
             (define-fun y () Int (- 3)) (define-fun assertion () Bool b))",
            "((define-fun b () Bool true)\n (define-fun x () Int 7)\n (define-fun y () Int (- 3)))",
        ] {
            let parsed = parse(text, &entries).expect("model parses");
            assert!(parsed.first_defect().is_none());
            assert!(parsed.assignment.booleans["b"]);
            assert_eq!(parsed.assignment.integers["x"], 7);
            assert_eq!(parsed.assignment.integers["y"], -3);
            assert_eq!(parsed.defined.len(), 3);
        }
    }

    /// FR-011-AC-6: numeral spellings and the length and 128-bit boundaries.
    #[trace("TC-017", "FR-011-AC-6")]
    #[test]
    fn numeral_boundaries() {
        let entries = [("x", Sort::Int)];
        let model = |value: &str| format!("(model (define-fun x () Int {value}))");
        for malformed in [
            "1.5", "1e5", "-5", "007", "(- -5)", "(+ 1 2)", "abc", "(- 1 2)",
        ] {
            assert_eq!(
                reason_of(&model(malformed), &entries),
                ReplayFailureReason::ValueMalformed,
                "{malformed}"
            );
        }
        assert_eq!(
            reason_of("(model (define-fun x () Real 1.0))", &entries),
            ReplayFailureReason::ValueMalformed
        );
        let long = "1".repeat(1025);
        assert_eq!(
            reason_of(&model(&long), &entries),
            ReplayFailureReason::ValueMalformed
        );
        let at_limit = format!("1{}", "0".repeat(1023));
        assert_eq!(
            reason_of(&model(&at_limit), &entries),
            ReplayFailureReason::ValueOutOfDomain
        );
        let two_127 = (1u128 << 127).to_string();
        assert_eq!(
            reason_of(&model(&two_127), &entries),
            ReplayFailureReason::ValueOutOfDomain
        );
        // The negative bound of the 128-bit range is representable; one beyond it is not.
        let minimum = parse(&model(&format!("(- {two_127})")), &entries).expect("parses");
        assert_eq!(minimum.assignment.integers["x"], i128::MIN);
        let beyond = (1u128 << 127) + 1;
        assert_eq!(
            reason_of(&model(&format!("(- {beyond})")), &entries),
            ReplayFailureReason::ValueOutOfDomain
        );
        let maximum = parse(&model(&i128::MAX.to_string()), &entries).expect("parses");
        assert_eq!(maximum.assignment.integers["x"], i128::MAX);
    }

    /// FR-011-AC-4 and AC-10: each parse defect has its reason, and the first listed wins.
    #[trace("TC-017", "FR-011-AC-4", "FR-011-AC-10")]
    #[test]
    fn parse_defects_follow_the_reason_precedence() {
        let entries = [("b", Sort::Bool), ("x", Sort::Int)];
        let reason = |text: &str| reason_of(text, &entries);
        assert_eq!(
            reason("(model (define-fun x () Int 1) (define-fun x () Int 1))"),
            ReplayFailureReason::SymbolRepeated
        );
        assert_eq!(
            reason("(model (define-fun z () Int 1))"),
            ReplayFailureReason::SymbolUnknown
        );
        assert_eq!(
            reason("(model (define-fun x () Bool true))"),
            ReplayFailureReason::SortMismatch
        );
        assert_eq!(
            reason("(model (define-fun b () Int 1))"),
            ReplayFailureReason::SortMismatch
        );
        assert_eq!(
            reason("(model (define-fun b () Bool 1))"),
            ReplayFailureReason::SortMismatch
        );
        // repeated outranks unknown, which outranks sort mismatch
        assert_eq!(
            reason(
                "(model (define-fun b () Int 1) (define-fun z () Int 1) \
                 (define-fun x () Int 1) (define-fun x () Int 1))"
            ),
            ReplayFailureReason::SymbolRepeated
        );
        assert_eq!(
            reason("(model (define-fun b () Int 1) (define-fun z () Int 1))"),
            ReplayFailureReason::SymbolUnknown
        );
        // a malformed numeral outranks every symbol-level defect of its own entry
        for text in [
            "(model (define-fun z () Int 1.5))",
            "(model (define-fun x () Int 1) (define-fun x () Int 1.5))",
            "(model (define-fun b () Int 1.5))",
            "(model (define-fun x () Bool 1.5))",
        ] {
            assert_eq!(reason(text), ReplayFailureReason::ValueMalformed, "{text}");
        }
        // malformed outranks everything, including a structural defect after it
        assert_eq!(
            reason("(model (define-fun z () Int 1) (define-fun x () Int 1.5))"),
            ReplayFailureReason::ValueMalformed
        );
        for structural in [
            "",
            "(model",
            "(model) trailing",
            "(model (define-fun |x| () Int 1))",
            "(model (assert true))",
            "(model (define-fun x (y) Int 1))",
        ] {
            let failure = defect(structural, &entries).expect("a defect");
            assert_eq!(failure.reason(), ReplayFailureReason::ValueMalformed);
            assert_eq!(failure.subject(), None, "{structural}");
        }
    }

    /// FR-011 (model shape): a guard in the left operand protects the right operand, so an
    /// overflow-prone right side is not evaluated when the left side decides the result.
    #[trace("TC-017", "FR-011-AC-7")]
    #[test]
    fn boolean_connectives_short_circuit() {
        let overflowing = ReplayExpression::Compare {
            operator: ComparisonOperator::Greater,
            left: ReplayTerm::Add(
                Box::new(ReplayTerm::Variable("x".to_owned())),
                Box::new(ReplayTerm::Constant(1)),
            ),
            right: ReplayTerm::Constant(0),
        };
        let boxed = || Box::new(overflowing.clone());
        let literal = |value| Box::new(ReplayExpression::Literal(value));
        let max = assigned(i128::MAX);
        assert_eq!(
            ReplayExpression::And(literal(false), boxed()).evaluate(&max),
            Ok(false)
        );
        assert_eq!(
            ReplayExpression::Or(literal(true), boxed()).evaluate(&max),
            Ok(true)
        );
        assert_eq!(
            ReplayExpression::Implies(literal(false), boxed()).evaluate(&max),
            Ok(true)
        );
        for unguarded in [
            ReplayExpression::And(literal(true), boxed()),
            ReplayExpression::Or(literal(false), boxed()),
            ReplayExpression::Implies(literal(true), boxed()),
        ] {
            assert_eq!(unguarded.evaluate(&max), Err(ReplayFault::Overflow));
        }
    }

    fn compare_assertion(
        name: &str,
        operator: ComparisonOperator,
        constant: i128,
    ) -> ReplayAssertion {
        ReplayAssertion {
            name: name.to_owned(),
            expression: ReplayExpression::Compare {
                operator,
                left: ReplayTerm::Add(
                    Box::new(ReplayTerm::Variable("x".to_owned())),
                    Box::new(ReplayTerm::Constant(1)),
                ),
                right: ReplayTerm::Constant(constant),
            },
        }
    }

    fn assigned(value: i128) -> ReplayAssignment {
        let mut assignment = ReplayAssignment::default();
        assignment.integers.insert("x".to_owned(), value);
        assignment
    }

    /// FR-011-AC-7: `x + 1 > 0` over an unchecked expression with `x` at the maximum overflows.
    #[trace("TC-017", "FR-011-AC-7", "NFR-004-AC-2")]
    #[test]
    fn overflowing_evaluation_fails_instead_of_wrapping() {
        let unchecked = [compare_assertion("a", ComparisonOperator::Greater, 0)];
        let failure = replay_assertions(&unchecked, &assigned(i128::MAX)).expect_err("overflow");
        assert_eq!(failure.reason(), ReplayFailureReason::EvaluationOverflow);
        assert_eq!(failure.subject(), Some("a"));
        let verified =
            replay_assertions(&unchecked, &assigned(i128::MAX - 1)).expect("no overflow");
        assert_eq!(verified, ["a"]);
    }

    /// FR-011-AC-10: the first assertion in map order that is not true decides the reason.
    #[trace("TC-017", "FR-011-AC-10")]
    #[test]
    fn overflow_and_false_assertions_are_ordered_by_assertion_map() {
        let overflowing = compare_assertion("a", ComparisonOperator::Greater, 0);
        let false_one = ReplayAssertion {
            name: "b".to_owned(),
            expression: ReplayExpression::Compare {
                operator: ComparisonOperator::Less,
                left: ReplayTerm::Constant(1),
                right: ReplayTerm::Constant(0),
            },
        };
        let max = assigned(i128::MAX);
        let forward =
            replay_assertions(&[overflowing.clone(), false_one.clone()], &max).expect_err("fails");
        assert_eq!(forward.reason(), ReplayFailureReason::EvaluationOverflow);
        assert_eq!(forward.subject(), Some("a"));
        let reverse = replay_assertions(
            &[
                ReplayAssertion {
                    name: "a".to_owned(),
                    ..false_one
                },
                ReplayAssertion {
                    name: "b".to_owned(),
                    ..overflowing
                },
            ],
            &max,
        )
        .expect_err("fails");
        assert_eq!(reverse.reason(), ReplayFailureReason::AssertionFalse);
        assert_eq!(reverse.subject(), Some("a"));
    }

    #[test]
    fn public_status_census_is_closed() {
        let statuses = [
            AnalysisStatus::Satisfied,
            AnalysisStatus::Refuted,
            AnalysisStatus::Unknown,
            AnalysisStatus::Unsupported,
            AnalysisStatus::Timeout,
            AnalysisStatus::ToolError,
        ];
        assert_eq!(
            statuses
                .iter()
                .filter(|status| status.is_conclusive())
                .count(),
            2
        );
        assert_eq!(
            statuses
                .map(AnalysisStatus::as_str)
                .into_iter()
                .collect::<BTreeSet<_>>()
                .len(),
            statuses.len()
        );
        for kind in [
            AnalysisKind::Consistency,
            AnalysisKind::Contradiction,
            AnalysisKind::Implication,
            AnalysisKind::Redundancy,
            AnalysisKind::DeadAntecedent,
        ] {
            assert_eq!(
                status_for(kind, SolverOutcome::Sat),
                if kind == AnalysisKind::Consistency {
                    AnalysisStatus::Satisfied
                } else {
                    AnalysisStatus::Refuted
                }
            );
            assert_eq!(
                status_for(kind, SolverOutcome::Unsat),
                if kind == AnalysisKind::Consistency {
                    AnalysisStatus::Refuted
                } else {
                    AnalysisStatus::Satisfied
                }
            );
        }
        assert_eq!(
            status_for(AnalysisKind::Consistency, SolverOutcome::Unknown),
            AnalysisStatus::Unknown
        );
        for outcome in [SolverOutcome::TimedOut, SolverOutcome::Cancelled] {
            assert_eq!(
                status_for(AnalysisKind::Consistency, outcome),
                AnalysisStatus::Timeout
            );
        }
        assert_eq!(
            status_for(
                AnalysisKind::Consistency,
                SolverOutcome::UnsupportedPlatform
            ),
            AnalysisStatus::Unsupported
        );
        for outcome in [
            SolverOutcome::MalformedOutput,
            SolverOutcome::SolverError,
            SolverOutcome::CleanupFailed,
        ] {
            assert_eq!(
                status_for(AnalysisKind::Consistency, outcome),
                AnalysisStatus::ToolError
            );
        }
    }
}
