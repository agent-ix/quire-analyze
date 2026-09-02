use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::{
    smt::{lower_analysis_statements, AnalysisStatement, ANALYSIS_MODEL_PROFILE},
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedBooleanModel {
    purpose: ModelPurpose,
    values: Vec<MappedBooleanValue>,
    replayed_assertions: Vec<String>,
}

impl VerifiedBooleanModel {
    pub const fn purpose(&self) -> ModelPurpose {
        self.purpose
    }

    pub fn values(&self) -> &[MappedBooleanValue] {
        &self.values
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
                conclusion.diagnostic = Some(error);
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
) -> Result<VerifiedBooleanModel, String> {
    if bytes.is_empty() {
        return Err("solver returned no model".to_owned());
    }
    let expected = query
        .variables()
        .iter()
        .map(|variable| variable.symbol.as_str())
        .collect::<BTreeSet<_>>();
    let assertion_aliases = query
        .assertions()
        .iter()
        .map(|assertion| assertion.name.as_str())
        .collect::<BTreeSet<_>>();
    let assignments = parse_boolean_model(bytes, &expected, &assertion_aliases)?;
    let observed = assignments
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if observed != expected {
        return Err("model symbol set differs from the declared query symbols".to_owned());
    }
    let mut replayed_assertions = Vec::with_capacity(query.replay_assertions().len());
    for assertion in query.replay_assertions() {
        if assertion.expression.evaluate(&assignments) != Some(true) {
            return Err(format!(
                "model does not satisfy replay assertion {}",
                assertion.name
            ));
        }
        replayed_assertions.push(assertion.name.clone());
    }
    let values = query
        .variables()
        .iter()
        .map(|variable| MappedBooleanValue {
            symbol: variable.symbol.clone(),
            value: assignments[&variable.symbol],
            origins: variable.origins.clone(),
            binding_group: variable.binding_group.clone(),
        })
        .collect();
    Ok(VerifiedBooleanModel {
        purpose,
        values,
        replayed_assertions,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ModelToken {
    Open,
    Close,
    Atom(String),
}

fn parse_boolean_model(
    bytes: &[u8],
    expected: &BTreeSet<&str>,
    assertion_aliases: &BTreeSet<&str>,
) -> Result<BTreeMap<String, bool>, String> {
    let tokens = tokenize_model(bytes)?;
    let mut cursor = 0;
    expect_token(&tokens, &mut cursor, &ModelToken::Open)?;
    if matches!(tokens.get(cursor), Some(ModelToken::Atom(atom)) if atom == "model") {
        cursor += 1;
    }
    let mut assignments = BTreeMap::new();
    while matches!(tokens.get(cursor), Some(ModelToken::Open)) {
        expect_token(&tokens, &mut cursor, &ModelToken::Open)?;
        expect_atom(&tokens, &mut cursor, "define-fun")?;
        let symbol = take_atom(&tokens, &mut cursor)?;
        expect_token(&tokens, &mut cursor, &ModelToken::Open)?;
        expect_token(&tokens, &mut cursor, &ModelToken::Close)?;
        expect_atom(&tokens, &mut cursor, "Bool")?;
        let value = if expected.contains(symbol.as_str()) {
            Some(match take_atom(&tokens, &mut cursor)?.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err("model contains a non-Boolean value".to_owned()),
            })
        } else if assertion_aliases.contains(symbol.as_str()) {
            skip_model_term(&tokens, &mut cursor)?;
            None
        } else {
            return Err("model defines an unexpected symbol".to_owned());
        };
        expect_token(&tokens, &mut cursor, &ModelToken::Close)?;
        if let Some(value) = value {
            if assignments.insert(symbol, value).is_some() {
                return Err("model defines one symbol more than once".to_owned());
            }
        }
    }
    expect_token(&tokens, &mut cursor, &ModelToken::Close)?;
    if cursor != tokens.len() {
        return Err("model contains trailing tokens".to_owned());
    }
    Ok(assignments)
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
    use super::*;

    #[test]
    fn boolean_model_parser_accepts_only_the_closed_shape() {
        let expected = BTreeSet::from(["x"]);
        let aliases = BTreeSet::from(["assertion"]);
        assert!(
            parse_boolean_model(b"(model (define-fun x () Bool true))", &expected, &aliases)
                .expect("z3-shaped model")["x"]
        );
        assert!(
            !parse_boolean_model(b"((define-fun x () Bool false))", &expected, &aliases)
                .expect("cvc5-shaped model")["x"]
        );
        assert!(
            parse_boolean_model(
                b"(model (define-fun x () Bool true) (define-fun assertion () Bool x))",
                &expected,
                &aliases,
            )
            .expect("Z3 named-assertion alias")["x"]
        );
        for malformed in [
            b"".as_slice(),
            b"(model (define-fun x () Int 1))".as_slice(),
            b"(model (define-fun x () Bool true) (define-fun x () Bool false))".as_slice(),
            b"(model (define-fun |x| () Bool true))".as_slice(),
            b"(model) trailing".as_slice(),
        ] {
            assert!(parse_boolean_model(malformed, &expected, &aliases).is_err());
        }
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
