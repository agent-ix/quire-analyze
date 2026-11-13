use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use quire_contract_model::{
    BooleanOperator, CanonicalProfile, Clause, ClauseRef, ComparisonOperator, ContractPackage,
    DeclarationEnvironment, ExecutionPoint, Expression, ExpressionKind, IntegerDomain, IntegerType,
    NumericOperator, OverflowPolicy, Requirement, RequirementRef, SourceSpan, StateObservation,
    SymbolName, TypeDeclaration, TypedExpression, ValueDeclarationKind, ValueType,
};
use sha2::{Digest as _, Sha256};

/// The accepted engine-neutral analysis model profile.
pub const ANALYSIS_MODEL_PROFILE: &str = "quire.analysis-model/v1";
/// The exact SMT-LIB2 encoding profile: Boolean and linear integer arithmetic.
pub const SMTLIB2_PROFILE: &str = "quire.smtlib2/v2";
/// The logic of every query that declares no datatypes.
pub const SMTLIB2_LOGIC: &str = "QF_LIA";
/// The logic of every query that declares at least one datatype.
pub const SMTLIB2_DATATYPE_LOGIC: &str = "QF_UFDTLIA";
/// Bound on variants of one enum.
pub const MAX_ENUM_VARIANTS: usize = 256;
/// Bound on datatypes declared in one query.
pub const MAX_QUERY_DATATYPES: usize = 256;
/// Bound on a generated query before any solver is invoked.
pub const MAX_QUERY_BYTES: usize = 16 * 1024 * 1024;
/// Bound on statements in one query.
pub const MAX_QUERY_STATEMENTS: usize = 10_000;
/// Bound on expression recursion performed by this lowering profile.
pub const MAX_EXPRESSION_DEPTH: usize = 128;
/// Bound on expression nodes inspected in one statement.
pub const MAX_EXPRESSION_NODES: usize = 100_000;
/// The only statement grouping represented by this issue's lowering API.
pub const LOWERING_REQUEST_KIND: &str = "boolean_conjunction";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AnalysisKind {
    Consistency,
    Contradiction,
    Implication,
    Redundancy,
    DeadAntecedent,
}

impl AnalysisKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Consistency => "consistency",
            Self::Contradiction => "contradiction",
            Self::Implication => "implication",
            Self::Redundancy => "redundancy",
            Self::DeadAntecedent => "dead-antecedent",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StatementRole {
    Assumption,
    Selected,
    Left,
    Right,
    Antecedent,
    Consequent,
    Peer,
    Candidate,
}

impl StatementRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Assumption => "assumption",
            Self::Selected => "selected",
            Self::Left => "left",
            Self::Right => "right",
            Self::Antecedent => "antecedent",
            Self::Consequent => "consequent",
            Self::Peer => "peer",
            Self::Candidate => "candidate",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssertionPolarity {
    Positive,
    Negated,
}

impl AssertionPolarity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negated => "negated",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LoweringErrorCode {
    InvalidStatement,
    InvalidBinding,
    DuplicateStatement,
    UnsupportedConstruct,
    UnsupportedType,
    ResourceLimit,
}

/// Why an integer term has no exact linear encoding.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UnsupportedClass {
    NonLinearArithmetic,
    IntegerDivision,
    IntegerRemainder,
    SaturatingArithmetic,
    ConstantOverflow,
}

impl UnsupportedClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NonLinearArithmetic => "non_linear_arithmetic",
            Self::IntegerDivision => "integer_division",
            Self::IntegerRemainder => "integer_remainder",
            Self::SaturatingArithmetic => "saturating_arithmetic",
            Self::ConstantOverflow => "constant_overflow",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringError {
    code: LoweringErrorCode,
    class: Option<UnsupportedClass>,
    path: String,
    message: String,
}

impl LoweringError {
    fn new(code: LoweringErrorCode, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            class: None,
            path: path.into(),
            message: message.into(),
        }
    }

    pub const fn code(&self) -> LoweringErrorCode {
        self.code
    }
    /// The integer construct class of an unsupported integer term, when it has one.
    pub const fn unsupported_class(&self) -> Option<UnsupportedClass> {
        self.class
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} at {}: {}",
            self.code, self.path, self.message
        )
    }
}

impl std::error::Error for LoweringError {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AnalysisDigest([u8; 32]);

impl AnalysisDigest {
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for AnalysisDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct StatementInput {
    clause: ClauseRef,
    clause_digest: AnalysisDigest,
    execution_point: ExecutionPoint,
    environment: DeclarationEnvironment,
    expression: Expression,
    source: SourceSpan,
}

impl StatementInput {
    /// Creates a statement only from a clause whose canonical digest can be recomputed.
    pub fn from_clause(
        package: &ContractPackage<TypedExpression>,
        requirement: &Requirement<TypedExpression>,
        clause: &Clause<TypedExpression>,
        environment: DeclarationEnvironment,
    ) -> Result<Self, Vec<LoweringError>> {
        let mut errors = Vec::new();
        let canonical = package
            .canonical_clause(requirement, clause, CanonicalProfile::V1)
            .map_err(|diagnostic| {
                errors.push(LoweringError::new(
                    LoweringErrorCode::InvalidStatement,
                    "statement.clause",
                    diagnostic.to_string(),
                ));
            })
            .ok();
        let requirement_ref = package.requirement_ref(requirement);
        if environment.owner() != &requirement_ref {
            errors.push(LoweringError::new(
                LoweringErrorCode::InvalidStatement,
                "statement.environment.owner",
                "declaration owner differs from the clause requirement",
            ));
        }
        let execution_point = clause.anchor().cloned();
        if execution_point.is_none() {
            errors.push(LoweringError::new(
                LoweringErrorCode::InvalidStatement,
                "statement.clause.anchor",
                "analysis requires an executable anchored clause",
            ));
        }
        if let Some(execution_point) = &execution_point {
            match environment.check_expression(
                clause.body().expression(),
                &ValueType::Boolean,
                execution_point,
                true,
            ) {
                Ok(checked) if &checked != clause.body() => errors.push(LoweringError::new(
                    LoweringErrorCode::InvalidStatement,
                    "statement.expression",
                    "clause typed expression differs from validation under the supplied declarations",
                )),
                Ok(_) => {}
                Err(diagnostics) => errors.extend(diagnostics.into_iter().map(|diagnostic| {
                LoweringError::new(
                    LoweringErrorCode::InvalidStatement,
                    "statement.expression",
                    diagnostic.to_string(),
                )
                })),
            }
        }
        if errors.is_empty() {
            let canonical = canonical.expect("canonical output exists when there are no errors");
            let execution_point =
                execution_point.expect("execution point exists when there are no errors");
            Ok(Self {
                clause: ClauseRef::new(requirement_ref, clause.id().clone()),
                clause_digest: hash_fields("clause", &[canonical.bytes().as_slice()]),
                execution_point,
                environment,
                expression: clause.body().expression().clone(),
                source: clause.source().clone(),
            })
        } else {
            sort_errors(&mut errors);
            Err(errors)
        }
    }

    pub fn clause(&self) -> &ClauseRef {
        &self.clause
    }
    pub const fn clause_digest(&self) -> AnalysisDigest {
        self.clause_digest
    }
    pub fn execution_point(&self) -> &ExecutionPoint {
        &self.execution_point
    }
    pub fn expression(&self) -> &Expression {
        &self.expression
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BindingMember {
    key: String,
    package: String,
    kind: &'static str,
    observation: &'static str,
    execution_point: String,
    type_shape_digest: AnalysisDigest,
}

impl BindingMember {
    pub fn from_declaration(
        environment: &DeclarationEnvironment,
        name: &SymbolName,
        observation: StateObservation,
        execution_point: &ExecutionPoint,
    ) -> Result<Self, LoweringError> {
        let declaration = environment
            .values()
            .iter()
            .find(|value| value.name() == name)
            .ok_or_else(|| {
                LoweringError::new(
                    LoweringErrorCode::InvalidBinding,
                    "binding.member",
                    "value declaration does not resolve",
                )
            })?;
        let kind = declaration_kind(declaration.kind());
        if declaration.kind() == ValueDeclarationKind::Input
            && observation != StateObservation::Current
        {
            return Err(LoweringError::new(
                LoweringErrorCode::InvalidBinding,
                "binding.member.observation",
                "input bindings require current observation",
            ));
        }
        let type_shape_digest = type_shape_digest(environment, declaration.value_type())?;
        let package = environment.owner().package().as_str().to_owned();
        let observation = observation_name(observation);
        let execution_point = execution_point_key(execution_point);
        let key = variable_key(
            environment.owner(),
            kind,
            observation,
            name.as_str(),
            &execution_point,
            type_shape_digest,
        );
        Ok(Self {
            key,
            package,
            kind,
            observation,
            execution_point,
            type_shape_digest,
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }
    pub const fn type_shape_digest(&self) -> AnalysisDigest {
        self.type_shape_digest
    }

    fn compatible_with(&self, other: &Self) -> bool {
        self.package == other.package
            && self.kind == other.kind
            && self.observation == other.observation
            && self.execution_point == other.execution_point
            && self.type_shape_digest == other.type_shape_digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingGroup {
    id: String,
    members: Vec<BindingMember>,
}

impl BindingGroup {
    pub fn new(
        id: impl Into<String>,
        mut members: Vec<BindingMember>,
    ) -> Result<Self, LoweringError> {
        let id = id.into();
        if !valid_binding_id(&id) {
            return Err(LoweringError::new(
                LoweringErrorCode::InvalidBinding,
                "binding.id",
                "binding id must use the contract identifier grammar",
            ));
        }
        if members.len() < 2 {
            return Err(LoweringError::new(
                LoweringErrorCode::InvalidBinding,
                "binding.members",
                "binding group requires at least two members",
            ));
        }
        members.sort();
        if members.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(LoweringError::new(
                LoweringErrorCode::InvalidBinding,
                "binding.members",
                "binding group contains a duplicate member",
            ));
        }
        if members
            .iter()
            .skip(1)
            .any(|member| !members[0].compatible_with(member))
        {
            return Err(LoweringError::new(LoweringErrorCode::InvalidBinding, "binding.members", "binding members differ in package, kind, observation, execution point, or type shape"));
        }
        Ok(Self { id, members })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn members(&self) -> &[BindingMember] {
        &self.members
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityContract {
    pub profile: &'static str,
    pub smtlib_version: &'static str,
    pub logic: &'static str,
    pub exact_constructs: &'static [&'static str],
    pub unsupported_constructs: &'static [&'static str],
    pub features: &'static [&'static str],
}

pub const CAPABILITY_CONTRACT: CapabilityContract = CapabilityContract {
    profile: SMTLIB2_PROFILE,
    smtlib_version: "2.6",
    logic: SMTLIB2_LOGIC,
    exact_constructs: &[
        "boolean_literal",
        "boolean_value_reference",
        "boolean_not",
        "short_circuit_and",
        "short_circuit_or",
        "total_and",
        "total_or",
        "implication",
        "boolean_equal",
        "boolean_not_equal",
        "integer_literal",
        "integer_value_reference",
        "integer_negation",
        "integer_addition",
        "integer_subtraction",
        "integer_constant_multiplication",
        "integer_equal",
        "integer_not_equal",
        "integer_less",
        "integer_less_equal",
        "integer_greater",
        "integer_greater_equal",
        "enum_literal",
        "enum_equality",
        "enum_inequality",
    ],
    unsupported_constructs: &[
        "non_linear_arithmetic",
        "integer_division",
        "integer_remainder",
        "saturating_arithmetic",
        "constant_overflow",
        "rational",
        "text",
        "record",
        "option",
        "collection",
        "field_access",
        "option_access",
        "collection_access",
        "pure_function_call",
        "rational_arithmetic",
        "ordering",
        "quantification",
        "local_reference",
    ],
    features: &[
        "named_assertions",
        "models",
        "complete_source_map",
        "explicit_bindings",
    ],
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssertionMap {
    pub name: String,
    pub clause: ClauseRef,
    pub clause_digest: AnalysisDigest,
    pub source: SourceSpan,
    pub role: StatementRole,
    pub polarity: AssertionPolarity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableMap {
    pub symbol: String,
    pub origins: Vec<String>,
    pub binding_group: Option<String>,
}

/// The encoded symbols of one enum datatype, derived from the declaration environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumSortMap {
    /// The SMT-LIB sort symbol.
    pub sort: String,
    /// The declaring package.
    pub package: String,
    /// The declared enum name.
    pub name: String,
    /// The constructors, in ascending variant-name order.
    pub constructors: Vec<EnumConstructor>,
}

/// One enum variant and its encoded constructor symbol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumConstructor {
    /// The declared variant name.
    pub variant: String,
    /// The SMT-LIB constructor symbol.
    pub symbol: String,
}

/// The datatypes and datatype-sorted variables a query uses.
#[derive(Default)]
struct DatatypeSet {
    enums: BTreeMap<String, EnumSortMap>,
    variable_sorts: BTreeMap<String, String>,
}

/// A concrete model assignment: Boolean and integer values by query symbol.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ReplayAssignment {
    pub(crate) booleans: BTreeMap<String, bool>,
    pub(crate) integers: BTreeMap<String, i128>,
}

/// Why a replay expression produced no verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReplayFault {
    /// An intermediate result left the 128-bit signed range.
    Overflow,
    /// A referenced symbol has no value in the assignment.
    Unassigned,
    /// The expression involves an enum term, which is not replayed yet (VER-87).
    EnumUnreplayed,
}

/// An integer-valued replay term, evaluated with checked 128-bit signed arithmetic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReplayTerm {
    Constant(i128),
    Variable(String),
    Negate(Box<Self>),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Scale { factor: i128, term: Box<Self> },
}

impl ReplayTerm {
    pub(crate) fn evaluate(&self, values: &ReplayAssignment) -> Result<i128, ReplayFault> {
        match self {
            Self::Constant(value) => Ok(*value),
            Self::Variable(symbol) => values
                .integers
                .get(symbol)
                .copied()
                .ok_or(ReplayFault::Unassigned),
            Self::Negate(operand) => operand
                .evaluate(values)?
                .checked_neg()
                .ok_or(ReplayFault::Overflow),
            Self::Add(left, right) => left
                .evaluate(values)?
                .checked_add(right.evaluate(values)?)
                .ok_or(ReplayFault::Overflow),
            Self::Subtract(left, right) => left
                .evaluate(values)?
                .checked_sub(right.evaluate(values)?)
                .ok_or(ReplayFault::Overflow),
            Self::Scale { factor, term } => term
                .evaluate(values)?
                .checked_mul(*factor)
                .ok_or(ReplayFault::Overflow),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReplayExpression {
    Literal(bool),
    Variable(String),
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Implies(Box<Self>, Box<Self>),
    Equal(Box<Self>, Box<Self>),
    NotEqual(Box<Self>, Box<Self>),
    /// A term over an enum sort. Enum witnesses are not replayed yet (VER-87), so it never
    /// evaluates to a verdict.
    EnumUnreplayed,
    /// A comparison of two integer terms.
    Compare {
        operator: ComparisonOperator,
        left: ReplayTerm,
        right: ReplayTerm,
    },
}

impl ReplayExpression {
    pub(crate) fn evaluate(&self, values: &ReplayAssignment) -> Result<bool, ReplayFault> {
        match self {
            Self::Literal(value) => Ok(*value),
            Self::Variable(symbol) => values
                .booleans
                .get(symbol)
                .copied()
                .ok_or(ReplayFault::Unassigned),
            Self::Not(operand) => Ok(!operand.evaluate(values)?),
            // A guard in the left operand may protect arithmetic in the right one, so the right
            // operand is evaluated only when the left does not decide the result.
            Self::And(left, right) => Ok(left.evaluate(values)? && right.evaluate(values)?),
            Self::Or(left, right) => Ok(left.evaluate(values)? || right.evaluate(values)?),
            Self::Implies(left, right) => Ok(!left.evaluate(values)? || right.evaluate(values)?),
            Self::Equal(left, right) => Ok(left.evaluate(values)? == right.evaluate(values)?),
            Self::NotEqual(left, right) => Ok(left.evaluate(values)? != right.evaluate(values)?),
            Self::EnumUnreplayed => Err(ReplayFault::EnumUnreplayed),
            Self::Compare {
                operator,
                left,
                right,
            } => {
                let (left, right) = (left.evaluate(values)?, right.evaluate(values)?);
                Ok(match operator {
                    ComparisonOperator::Equal => left == right,
                    ComparisonOperator::NotEqual => left != right,
                    ComparisonOperator::Less => left < right,
                    ComparisonOperator::LessEqual => left <= right,
                    ComparisonOperator::Greater => left > right,
                    ComparisonOperator::GreaterEqual => left >= right,
                })
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReplayAssertion {
    pub(crate) name: String,
    pub(crate) expression: ReplayExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryBundle {
    profile: &'static str,
    logic: &'static str,
    query: String,
    analysis_request_digest: AnalysisDigest,
    query_digest: AnalysisDigest,
    binding_set_digest: AnalysisDigest,
    analysis_kind: Option<AnalysisKind>,
    requests_model: bool,
    assertions: Vec<AssertionMap>,
    variables: Vec<VariableMap>,
    enum_sorts: Vec<EnumSortMap>,
    enum_variables: Vec<String>,
    replay_assertions: Vec<ReplayAssertion>,
    integer_ranges: BTreeMap<String, (i128, i128)>,
}

impl QueryBundle {
    pub const fn profile(&self) -> &'static str {
        self.profile
    }

    pub const fn logic(&self) -> &'static str {
        self.logic
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Replaces the query text, to show that replay never reads it.
    #[cfg(test)]
    pub(crate) fn set_query_text(&mut self, text: &str) {
        self.query = text.to_owned();
    }

    pub const fn analysis_request_digest(&self) -> AnalysisDigest {
        self.analysis_request_digest
    }

    pub const fn query_digest(&self) -> AnalysisDigest {
        self.query_digest
    }

    pub const fn binding_set_digest(&self) -> AnalysisDigest {
        self.binding_set_digest
    }

    pub const fn analysis_kind(&self) -> Option<AnalysisKind> {
        self.analysis_kind
    }

    pub const fn requests_model(&self) -> bool {
        self.requests_model
    }

    pub fn assertions(&self) -> &[AssertionMap] {
        &self.assertions
    }

    pub fn variables(&self) -> &[VariableMap] {
        &self.variables
    }

    /// The symbols of enum-sorted variables, in ascending order.
    pub(crate) fn enum_variables(&self) -> &[String] {
        &self.enum_variables
    }

    /// The enum datatypes the query declares, in declaration order.
    pub fn enum_sorts(&self) -> &[EnumSortMap] {
        &self.enum_sorts
    }

    pub(crate) fn replay_assertions(&self) -> &[ReplayAssertion] {
        &self.replay_assertions
    }

    /// The declared inclusive range of each integer symbol; Boolean symbols are absent.
    pub(crate) const fn integer_ranges(&self) -> &BTreeMap<String, (i128, i128)> {
        &self.integer_ranges
    }
}

pub fn lower_boolean_statements(
    statements: &[StatementInput],
    bindings: &[BindingGroup],
) -> Result<QueryBundle, Vec<LoweringError>> {
    let statements = statements
        .iter()
        .map(|statement| AnalysisStatement {
            statement,
            role: StatementRole::Selected,
            polarity: AssertionPolarity::Positive,
        })
        .collect::<Vec<_>>();
    lower_statement_specs(None, LOWERING_REQUEST_KIND, &statements, bindings)
}

pub(crate) struct AnalysisStatement<'a> {
    pub(crate) statement: &'a StatementInput,
    pub(crate) role: StatementRole,
    pub(crate) polarity: AssertionPolarity,
}

pub(crate) fn lower_analysis_statements(
    kind: AnalysisKind,
    statements: &[AnalysisStatement<'_>],
    bindings: &[BindingGroup],
) -> Result<QueryBundle, Vec<LoweringError>> {
    lower_statement_specs(Some(kind), kind.as_str(), statements, bindings)
}

fn lower_statement_specs(
    analysis_kind: Option<AnalysisKind>,
    request_kind: &str,
    statements: &[AnalysisStatement<'_>],
    bindings: &[BindingGroup],
) -> Result<QueryBundle, Vec<LoweringError>> {
    if statements.is_empty() {
        return Err(vec![LoweringError::new(
            LoweringErrorCode::InvalidStatement,
            "statements",
            "at least one statement is required",
        )]);
    }
    if statements.len() > MAX_QUERY_STATEMENTS {
        return Err(vec![LoweringError::new(
            LoweringErrorCode::ResourceLimit,
            "statements",
            "statement count exceeds the public bound",
        )]);
    }

    let (binding_lookup, binding_set_digest) = validate_bindings(bindings)?;
    let mut encountered = BTreeSet::new();
    let mut variables: BTreeMap<String, VariableMap> = BTreeMap::new();
    let mut ranges: BTreeMap<String, (i128, i128)> = BTreeMap::new();
    let mut datatypes = DatatypeSet::default();
    let mut lowered = Vec::with_capacity(statements.len());
    let mut errors = Vec::new();

    for specification in statements {
        let source_digest = statement_digest(specification.statement, binding_set_digest);
        let mapped_digest = analysis_kind.map_or(source_digest, |kind| {
            hash_fields(
                "analysis-statement",
                &[
                    ANALYSIS_MODEL_PROFILE.as_bytes(),
                    kind.as_str().as_bytes(),
                    specification.role.as_str().as_bytes(),
                    specification.polarity.as_str().as_bytes(),
                    source_digest.as_bytes(),
                ],
            )
        });
        let assertion_name = assertion_symbol(specification.statement, mapped_digest);
        match lower_expression(
            specification.statement,
            &binding_lookup,
            &mut encountered,
            &mut variables,
            &mut ranges,
            &mut datatypes,
        ) {
            Ok(mut expression) => {
                if specification.polarity == AssertionPolarity::Negated {
                    expression = LoweredExpression {
                        smt: format!("(not {})", expression.smt),
                        replay: ReplayExpression::Not(Box::new(expression.replay)),
                    };
                }
                lowered.push((assertion_name, mapped_digest, specification, expression));
            }
            Err(error) => errors.push(error),
        }
    }
    if !errors.is_empty() {
        sort_errors(&mut errors);
        return Err(errors);
    }

    for member in binding_lookup.keys() {
        if !encountered.contains(member) {
            errors.push(LoweringError::new(
                LoweringErrorCode::InvalidBinding,
                "binding.members",
                format!("binding member {member} is not referenced by a selected statement"),
            ));
        }
    }
    if !errors.is_empty() {
        sort_errors(&mut errors);
        return Err(errors);
    }

    lowered.sort_by(|left, right| left.0.cmp(&right.0));
    if lowered.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(vec![LoweringError::new(
            LoweringErrorCode::DuplicateStatement,
            "statements",
            "duplicate stable assertion identity",
        )]);
    }

    let mut canonical_statements = Vec::new();
    for (_, digest, _, _) in &lowered {
        push_field(&mut canonical_statements, digest.as_bytes());
    }
    let analysis_request_digest = hash_fields(
        "request",
        &[
            ANALYSIS_MODEL_PROFILE.as_bytes(),
            request_kind.as_bytes(),
            &canonical_statements,
            MAX_QUERY_STATEMENTS.to_string().as_bytes(),
            MAX_QUERY_BYTES.to_string().as_bytes(),
            MAX_EXPRESSION_DEPTH.to_string().as_bytes(),
            MAX_EXPRESSION_NODES.to_string().as_bytes(),
            binding_set_digest.as_bytes(),
        ],
    );

    let mut query = String::new();
    query.push_str("(set-info :smt-lib-version 2.6)\n");
    query.push_str("(set-info :source |");
    query.push_str(SMTLIB2_PROFILE);
    query.push_str("|)\n(set-option :produce-models true)\n(set-logic ");
    let logic = if datatypes.enums.is_empty() {
        SMTLIB2_LOGIC
    } else {
        SMTLIB2_DATATYPE_LOGIC
    };
    query.push_str(logic);
    query.push_str(")\n");
    for sort in datatypes.enums.values() {
        query.push_str("(declare-datatypes ((");
        query.push_str(&sort.sort);
        query.push_str(" 0)) ((");
        for (index, constructor) in sort.constructors.iter().enumerate() {
            if index > 0 {
                query.push(' ');
            }
            query.push('(');
            query.push_str(&constructor.symbol);
            query.push(')');
        }
        query.push_str(")))\n");
    }
    for symbol in variables.keys() {
        query.push_str("(declare-fun ");
        query.push_str(symbol);
        query.push_str(" () ");
        if let Some(sort) = datatypes.variable_sorts.get(symbol) {
            query.push_str(sort);
        } else if ranges.contains_key(symbol) {
            query.push_str("Int");
        } else {
            query.push_str("Bool");
        }
        query.push_str(")\n");
    }
    for (symbol, (minimum, maximum)) in &ranges {
        query.push_str(&format!(
            "(assert (and (<= {} {symbol}) (<= {symbol} {})))\n",
            numeral(*minimum),
            numeral(*maximum)
        ));
    }
    let mut assertions = Vec::with_capacity(lowered.len());
    let mut replay_assertions = Vec::with_capacity(lowered.len());
    for (name, _, specification, expression) in lowered {
        query.push_str("(assert (! ");
        query.push_str(&expression.smt);
        query.push_str(" :named ");
        query.push_str(&name);
        query.push_str("))\n");
        assertions.push(AssertionMap {
            name: name.clone(),
            clause: specification.statement.clause.clone(),
            clause_digest: specification.statement.clause_digest,
            source: specification.statement.source.clone(),
            role: specification.role,
            polarity: specification.polarity,
        });
        replay_assertions.push(ReplayAssertion {
            name,
            expression: expression.replay,
        });
    }
    query.push_str("(check-sat)\n");
    let requests_model = analysis_kind.is_some();
    if requests_model {
        query.push_str("(get-model)\n");
    }
    if query.len() > MAX_QUERY_BYTES {
        return Err(vec![LoweringError::new(
            LoweringErrorCode::ResourceLimit,
            "query",
            "generated query exceeds the public byte bound",
        )]);
    }
    let query_digest = hash_fields(
        "query",
        &[
            SMTLIB2_PROFILE.as_bytes(),
            analysis_request_digest.as_bytes(),
            query.as_bytes(),
        ],
    );
    Ok(QueryBundle {
        profile: SMTLIB2_PROFILE,
        logic,
        query,
        analysis_request_digest,
        query_digest,
        binding_set_digest,
        analysis_kind,
        requests_model,
        assertions,
        variables: variables.into_values().collect(),
        enum_sorts: datatypes.enums.into_values().collect(),
        enum_variables: datatypes.variable_sorts.into_keys().collect(),
        replay_assertions,
        integer_ranges: ranges,
    })
}

type BindingLookup = BTreeMap<String, String>;

struct LoweredExpression {
    smt: String,
    replay: ReplayExpression,
}

fn validate_bindings(
    bindings: &[BindingGroup],
) -> Result<(BindingLookup, AnalysisDigest), Vec<LoweringError>> {
    let mut groups: Vec<_> = bindings.iter().collect();
    groups.sort_by(|left, right| left.id.cmp(&right.id));
    let mut lookup = BTreeMap::new();
    let mut canonical = Vec::new();
    let mut errors = Vec::new();
    let mut group_ids = BTreeSet::new();
    for group in groups {
        if !group_ids.insert(group.id.clone()) {
            errors.push(LoweringError::new(
                LoweringErrorCode::InvalidBinding,
                "binding.id",
                format!("binding id {} occurs more than once", group.id),
            ));
        }
        push_field(&mut canonical, group.id.as_bytes());
        for member in &group.members {
            push_field(&mut canonical, member.key.as_bytes());
            if lookup
                .insert(member.key.clone(), group.id.clone())
                .is_some()
            {
                errors.push(LoweringError::new(
                    LoweringErrorCode::InvalidBinding,
                    "binding.members",
                    "one member occurs in multiple binding groups",
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok((
            lookup,
            hash_fields("bindings", &[ANALYSIS_MODEL_PROFILE.as_bytes(), &canonical]),
        ))
    } else {
        sort_errors(&mut errors);
        Err(errors)
    }
}

fn lower_expression(
    statement: &StatementInput,
    bindings: &BindingLookup,
    encountered: &mut BTreeSet<String>,
    variables: &mut BTreeMap<String, VariableMap>,
    ranges: &mut BTreeMap<String, (i128, i128)>,
    datatypes: &mut DatatypeSet,
) -> Result<LoweredExpression, LoweringError> {
    let mut state = LowerState {
        statement,
        bindings,
        encountered,
        variables,
        ranges,
        datatypes,
        nodes: 0,
    };
    state.lower(&statement.expression, 1, &ExprPath::ROOT)
}

struct LowerState<'a> {
    statement: &'a StatementInput,
    bindings: &'a BindingLookup,
    encountered: &'a mut BTreeSet<String>,
    variables: &'a mut BTreeMap<String, VariableMap>,
    ranges: &'a mut BTreeMap<String, (i128, i128)>,
    datatypes: &'a mut DatatypeSet,
    nodes: usize,
}

impl LowerState<'_> {
    fn lower(
        &mut self,
        expression: &Expression,
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
        count_node(depth, &mut self.nodes)?;
        match expression.kind() {
            ExpressionKind::BooleanLiteral { value } => Ok(LoweredExpression {
                smt: value.to_string(),
                replay: ReplayExpression::Literal(*value),
            }),
            ExpressionKind::ValueReference { name, observation } => {
                let symbol = self.reference(name, *observation)?;
                Ok(LoweredExpression {
                    smt: symbol.clone(),
                    replay: ReplayExpression::Variable(symbol),
                })
            }
            ExpressionKind::EnumLiteral {
                enumeration,
                variant,
            } => {
                let sort = self.enum_sort(enumeration)?;
                let constructor = sort
                    .constructors
                    .iter()
                    .find(|constructor| constructor.variant == variant.as_str())
                    .map(|constructor| constructor.symbol.clone())
                    .ok_or_else(|| {
                        LoweringError::new(
                            LoweringErrorCode::InvalidStatement,
                            "expression.enum_literal",
                            "validated enum variant no longer resolves",
                        )
                    })?;
                Ok(LoweredExpression {
                    smt: constructor,
                    replay: ReplayExpression::EnumUnreplayed,
                })
            }
            ExpressionKind::BooleanNot { operand } => {
                let operand = self.lower(operand, depth + 1, &path.child("operand"))?;
                Ok(LoweredExpression {
                    smt: format!("(not {})", operand.smt),
                    replay: ReplayExpression::Not(Box::new(operand.replay)),
                })
            }
            ExpressionKind::Boolean {
                operator,
                left,
                right,
            } => {
                let left = self.lower(left, depth + 1, &path.child("left"))?;
                let right = self.lower(right, depth + 1, &path.child("right"))?;
                let (operator_name, replay) = match operator {
                    BooleanOperator::ShortCircuitAnd | BooleanOperator::TotalAnd => (
                        "and",
                        ReplayExpression::And(Box::new(left.replay), Box::new(right.replay)),
                    ),
                    BooleanOperator::ShortCircuitOr | BooleanOperator::TotalOr => (
                        "or",
                        ReplayExpression::Or(Box::new(left.replay), Box::new(right.replay)),
                    ),
                    BooleanOperator::Implication => (
                        "=>",
                        ReplayExpression::Implies(Box::new(left.replay), Box::new(right.replay)),
                    ),
                };
                Ok(LoweredExpression {
                    smt: format!("({operator_name} {} {})", left.smt, right.smt),
                    replay,
                })
            }
            ExpressionKind::Compare {
                operator,
                left,
                right,
            } => {
                if self.is_integer_term(left) {
                    return self.lower_integer_comparison(*operator, left, right, depth, path);
                }
                if !matches!(
                    operator,
                    ComparisonOperator::Equal | ComparisonOperator::NotEqual
                ) {
                    return Err(unsupported(
                        &path.render(),
                        "ordering comparison of non-integer terms is unsupported",
                    ));
                }
                let left_sort = self.term_sort(left);
                let right_sort = self.term_sort(right);
                let left = self.lower(left, depth + 1, &path.child("left"))?;
                let right = self.lower(right, depth + 1, &path.child("right"))?;
                if left_sort.is_none() || left_sort != right_sort {
                    return Err(unsupported(
                        &path.render(),
                        "comparison operands must both be Boolean or both of one enum",
                    ));
                }
                if matches!(left_sort, Some(TermSort::Enum(_))) {
                    let name = if *operator == ComparisonOperator::Equal {
                        "="
                    } else {
                        "distinct"
                    };
                    return Ok(LoweredExpression {
                        smt: format!("({name} {} {})", left.smt, right.smt),
                        replay: ReplayExpression::EnumUnreplayed,
                    });
                }
                Ok(if *operator == ComparisonOperator::Equal {
                    LoweredExpression {
                        smt: format!("(= {} {})", left.smt, right.smt),
                        replay: ReplayExpression::Equal(
                            Box::new(left.replay),
                            Box::new(right.replay),
                        ),
                    }
                } else {
                    LoweredExpression {
                        smt: format!("(not (= {} {}))", left.smt, right.smt),
                        replay: ReplayExpression::NotEqual(
                            Box::new(left.replay),
                            Box::new(right.replay),
                        ),
                    }
                })
            }
            other => Err(unsupported(&path.render(), construct_name(other))),
        }
    }

    /// Whether the term is integer-typed. Validation makes both comparison operands agree, so
    /// the left operand decides.
    fn is_integer_term(&self, expression: &Expression) -> bool {
        match expression.kind() {
            ExpressionKind::IntegerLiteral { .. } => true,
            ExpressionKind::Numeric { left, .. } => self.is_integer_term(left),
            ExpressionKind::NumericNegate { operand } => self.is_integer_term(operand),
            ExpressionKind::ValueReference { name, .. } => self.integer_type(name).is_some(),
            _ => false,
        }
    }

    /// The sort of a lowered non-integer term, when it is Boolean or an enum.
    fn term_sort(&self, expression: &Expression) -> Option<TermSort> {
        match expression.kind() {
            ExpressionKind::BooleanLiteral { .. }
            | ExpressionKind::BooleanNot { .. }
            | ExpressionKind::Boolean { .. }
            | ExpressionKind::Compare { .. } => Some(TermSort::Bool),
            ExpressionKind::EnumLiteral { enumeration, .. } => {
                Some(TermSort::Enum(enumeration.clone()))
            }
            ExpressionKind::ValueReference { name, .. } => self
                .statement
                .environment
                .values()
                .iter()
                .find(|value| value.name() == name)
                .and_then(|declaration| match declaration.value_type() {
                    ValueType::Boolean => Some(TermSort::Bool),
                    ValueType::Enum { name } => Some(TermSort::Enum(name.clone())),
                    _ => None,
                }),
            _ => None,
        }
    }

    /// Declares the enum datatype of the statement environment on first use and returns its map.
    fn enum_sort(&mut self, name: &SymbolName) -> Result<EnumSortMap, LoweringError> {
        let environment = &self.statement.environment;
        let declaration = environment
            .types()
            .iter()
            .find_map(|candidate| match candidate {
                TypeDeclaration::Enum { declaration } if declaration.name() == name => {
                    Some(declaration)
                }
                _ => None,
            })
            .ok_or_else(|| {
                LoweringError::new(
                    LoweringErrorCode::InvalidBinding,
                    "binding.type",
                    "enum declaration does not resolve",
                )
            })?;
        if declaration.variants().len() > MAX_ENUM_VARIANTS {
            return Err(LoweringError::new(
                LoweringErrorCode::ResourceLimit,
                "type.enum.variants",
                "enum variant count exceeds the public bound",
            ));
        }
        let mut variants: Vec<&str> = declaration
            .variants()
            .iter()
            .map(|variant| variant.name().as_str())
            .collect();
        variants.sort_unstable();
        let package = environment.owner().package().as_str();
        let mut identity = vec![package.as_bytes(), name.as_str().as_bytes()];
        identity.extend(variants.iter().map(|variant| variant.as_bytes()));
        let digest = hex(hash_fields("enum-sort", &identity).as_bytes());
        let sort = format!("s_{digest}");
        if let Some(existing) = self.datatypes.enums.get(&sort) {
            return Ok(existing.clone());
        }
        if self.datatypes.enums.len() >= MAX_QUERY_DATATYPES {
            return Err(LoweringError::new(
                LoweringErrorCode::ResourceLimit,
                "type.datatypes",
                "datatype count exceeds the public bound",
            ));
        }
        let map = EnumSortMap {
            sort: sort.clone(),
            package: package.to_owned(),
            name: name.as_str().to_owned(),
            constructors: variants
                .iter()
                .map(|variant| EnumConstructor {
                    variant: (*variant).to_owned(),
                    symbol: format!("c_{digest}_{}", hex(variant.as_bytes())),
                })
                .collect(),
        };
        self.datatypes.enums.insert(sort, map.clone());
        Ok(map)
    }

    fn integer_type(&self, name: &SymbolName) -> Option<IntegerType> {
        integer_declaration_type(&self.statement.environment, name)
    }

    fn lower_integer_comparison(
        &mut self,
        operator: ComparisonOperator,
        left: &Expression,
        right: &Expression,
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
        let environment = &self.statement.environment;
        let types = |name: &SymbolName| integer_declaration_type(environment, name);
        let left = fold_integer(
            left,
            &path.child("left"),
            &types,
            depth + 1,
            &mut self.nodes,
        )?;
        let right = fold_integer(
            right,
            &path.child("right"),
            &types,
            depth + 1,
            &mut self.nodes,
        )?;
        let (left, left_term) = self.emit(&left.tree)?;
        let (right, right_term) = self.emit(&right.tree)?;
        let name = match operator {
            ComparisonOperator::Equal => "=",
            ComparisonOperator::NotEqual => "distinct",
            ComparisonOperator::Less => "<",
            ComparisonOperator::LessEqual => "<=",
            ComparisonOperator::Greater => ">",
            ComparisonOperator::GreaterEqual => ">=",
        };
        Ok(LoweredExpression {
            smt: format!("({name} {left} {right})"),
            replay: ReplayExpression::Compare {
                operator,
                left: left_term,
                right: right_term,
            },
        })
    }

    fn emit(&mut self, tree: &IntegerTree) -> Result<(String, ReplayTerm), LoweringError> {
        Ok(match tree {
            IntegerTree::Constant(value) => (numeral(*value), ReplayTerm::Constant(*value)),
            IntegerTree::Reference { name, observation } => {
                let symbol = self.reference(name, *observation)?;
                (symbol.clone(), ReplayTerm::Variable(symbol))
            }
            IntegerTree::Negate(operand) => {
                let (smt, term) = self.emit(operand)?;
                (format!("(- {smt})"), ReplayTerm::Negate(Box::new(term)))
            }
            IntegerTree::Add(left, right) => {
                let (left_smt, left) = self.emit(left)?;
                let (right_smt, right) = self.emit(right)?;
                (
                    format!("(+ {left_smt} {right_smt})"),
                    ReplayTerm::Add(Box::new(left), Box::new(right)),
                )
            }
            IntegerTree::Subtract(left, right) => {
                let (left_smt, left) = self.emit(left)?;
                let (right_smt, right) = self.emit(right)?;
                (
                    format!("(- {left_smt} {right_smt})"),
                    ReplayTerm::Subtract(Box::new(left), Box::new(right)),
                )
            }
            IntegerTree::Scale { factor, term } => {
                let (smt, term) = self.emit(term)?;
                (
                    format!("(* {} {smt})", numeral(*factor)),
                    ReplayTerm::Scale {
                        factor: *factor,
                        term: Box::new(term),
                    },
                )
            }
        })
    }

    /// Resolves a value reference to its query symbol and records the variable. An integer
    /// reference also records its declared range.
    fn reference(
        &mut self,
        name: &SymbolName,
        observation: StateObservation,
    ) -> Result<String, LoweringError> {
        let statement = self.statement;
        let declaration = statement
            .environment
            .values()
            .iter()
            .find(|value| value.name() == name)
            .ok_or_else(|| {
                LoweringError::new(
                    LoweringErrorCode::InvalidStatement,
                    "expression.value_reference",
                    "validated declaration no longer resolves",
                )
            })?;
        let mut enum_sort = None;
        let range =
            match declaration.value_type() {
                ValueType::Boolean => None,
                ValueType::Integer { value } => Some((value.minimum(), value.maximum())),
                ValueType::Enum { name } => {
                    enum_sort = Some(self.enum_sort(name)?.sort);
                    None
                }
                _ => return Err(unsupported(
                    "expression.value_reference",
                    "only Boolean, integer and enum value references are exact in quire.smtlib2/v2",
                )),
            };
        let kind = declaration_kind(declaration.kind());
        let point = execution_point_key(&statement.execution_point);
        let digest = type_shape_digest(&statement.environment, declaration.value_type())?;
        let key = variable_key(
            statement.environment.owner(),
            kind,
            observation_name(observation),
            name.as_str(),
            &point,
            digest,
        );
        let (symbol, binding_group) = match self.bindings.get(&key) {
            Some(group) => {
                self.encountered.insert(key.clone());
                (
                    format!("v_b_{}", hex(group.as_bytes())),
                    Some(group.clone()),
                )
            }
            None => (
                format!("v_s_{}", hash_fields("variable", &[key.as_bytes()])),
                None,
            ),
        };
        let entry = self
            .variables
            .entry(symbol.clone())
            .or_insert_with(|| VariableMap {
                symbol: symbol.clone(),
                origins: Vec::new(),
                binding_group,
            });
        if entry.binding_group.is_none()
            && !entry.origins.is_empty()
            && !entry.origins.contains(&key)
        {
            return Err(LoweringError::new(
                LoweringErrorCode::InvalidStatement,
                "expression.value_reference",
                "distinct variable identities produced one symbol",
            ));
        }
        if !entry.origins.contains(&key) {
            entry.origins.push(key);
            entry.origins.sort();
        }
        if let Some(range) = range {
            self.ranges.insert(symbol.clone(), range);
        }
        if let Some(sort) = enum_sort {
            self.datatypes.variable_sorts.insert(symbol.clone(), sort);
        }
        Ok(symbol)
    }
}

/// The sort of a Boolean or enum term.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TermSort {
    Bool,
    Enum(SymbolName),
}

/// The position of an expression within the statement, rendered only when an error needs it.
struct ExprPath<'a> {
    parent: Option<&'a ExprPath<'a>>,
    segment: &'static str,
}

impl<'a> ExprPath<'a> {
    const ROOT: ExprPath<'static> = ExprPath {
        parent: None,
        segment: "expression",
    };

    fn child(&'a self, segment: &'static str) -> ExprPath<'a> {
        ExprPath {
            parent: Some(self),
            segment,
        }
    }

    fn render(&self) -> String {
        match self.parent {
            Some(parent) => format!("{}.{}", parent.render(), self.segment),
            None => self.segment.to_owned(),
        }
    }
}

fn count_node(depth: usize, nodes: &mut usize) -> Result<(), LoweringError> {
    if depth > MAX_EXPRESSION_DEPTH {
        return Err(LoweringError::new(
            LoweringErrorCode::ResourceLimit,
            "expression.depth",
            "expression depth exceeds the public bound",
        ));
    }
    *nodes = nodes.saturating_add(1);
    if *nodes > MAX_EXPRESSION_NODES {
        return Err(LoweringError::new(
            LoweringErrorCode::ResourceLimit,
            "expression.nodes",
            "expression node count exceeds the public bound",
        ));
    }
    Ok(())
}

fn integer_declaration_type(
    environment: &DeclarationEnvironment,
    name: &SymbolName,
) -> Option<IntegerType> {
    environment
        .values()
        .iter()
        .find(|value| value.name() == name)
        .and_then(|declaration| match declaration.value_type() {
            ValueType::Integer { value } => Some(value.clone()),
            _ => None,
        })
}

/// Spells an integer exactly: a negative value is the negation of its magnitude.
fn numeral(value: i128) -> String {
    if value < 0 {
        format!("(- {})", value.unsigned_abs())
    } else {
        value.to_string()
    }
}

/// A linear integer term after constant folding.
#[derive(Clone, Debug, Eq, PartialEq)]
enum IntegerTree {
    Constant(i128),
    Reference {
        name: SymbolName,
        observation: StateObservation,
    },
    Negate(Box<IntegerTree>),
    Add(Box<IntegerTree>, Box<IntegerTree>),
    Subtract(Box<IntegerTree>, Box<IntegerTree>),
    /// A constant factor times a non-constant term.
    Scale {
        factor: i128,
        term: Box<IntegerTree>,
    },
}

struct FoldedInteger {
    tree: IntegerTree,
    value_type: IntegerType,
}

fn integer_unsupported(
    class: UnsupportedClass,
    path: &ExprPath<'_>,
    message: &str,
) -> LoweringError {
    let mut error = unsupported(&path.render(), &format!("{}: {message}", class.as_str()));
    error.class = Some(class);
    error
}

/// Folds every maximal constant subtree to one literal and rejects any term outside linear
/// integer arithmetic. `types` resolves a value reference to its declared integer type.
fn fold_integer(
    expression: &Expression,
    path: &ExprPath<'_>,
    types: &dyn Fn(&SymbolName) -> Option<IntegerType>,
    depth: usize,
    nodes: &mut usize,
) -> Result<FoldedInteger, LoweringError> {
    count_node(depth, nodes)?;
    let overflow = || {
        integer_unsupported(
            UnsupportedClass::ConstantOverflow,
            path,
            "constant fold leaves the 128-bit signed range",
        )
    };
    match expression.kind() {
        ExpressionKind::IntegerLiteral { value, value_type } => Ok(FoldedInteger {
            tree: IntegerTree::Constant(*value),
            value_type: value_type.clone(),
        }),
        ExpressionKind::ValueReference { name, observation } => {
            let value_type = types(name).ok_or_else(|| {
                unsupported(
                    &path.render(),
                    "only integer value references are exact in an integer term",
                )
            })?;
            Ok(FoldedInteger {
                tree: IntegerTree::Reference {
                    name: name.clone(),
                    observation: *observation,
                },
                value_type,
            })
        }
        ExpressionKind::NumericNegate { operand } => {
            let operand = fold_integer(operand, &path.child("operand"), types, depth + 1, nodes)?;
            reject_saturating(&operand.value_type, path)?;
            let tree = match operand.tree {
                IntegerTree::Constant(value) => {
                    IntegerTree::Constant(value.checked_neg().ok_or_else(overflow)?)
                }
                other => IntegerTree::Negate(Box::new(other)),
            };
            Ok(FoldedInteger {
                tree,
                value_type: operand.value_type,
            })
        }
        ExpressionKind::Numeric {
            operator,
            left,
            right,
        } => {
            match operator {
                NumericOperator::Divide => {
                    return Err(integer_unsupported(
                        UnsupportedClass::IntegerDivision,
                        path,
                        "integer division is outside linear integer arithmetic",
                    ))
                }
                NumericOperator::Remainder => {
                    return Err(integer_unsupported(
                        UnsupportedClass::IntegerRemainder,
                        path,
                        "integer remainder is outside linear integer arithmetic",
                    ))
                }
                NumericOperator::Add | NumericOperator::Subtract | NumericOperator::Multiply => {}
            }
            let left = fold_integer(left, &path.child("left"), types, depth + 1, nodes)?;
            let right = fold_integer(right, &path.child("right"), types, depth + 1, nodes)?;
            reject_saturating(&left.value_type, path)?;
            let tree = match (operator, left.tree, right.tree) {
                (NumericOperator::Add, IntegerTree::Constant(a), IntegerTree::Constant(b)) => {
                    IntegerTree::Constant(a.checked_add(b).ok_or_else(overflow)?)
                }
                (NumericOperator::Add, a, b) => IntegerTree::Add(Box::new(a), Box::new(b)),
                (NumericOperator::Subtract, IntegerTree::Constant(a), IntegerTree::Constant(b)) => {
                    IntegerTree::Constant(a.checked_sub(b).ok_or_else(overflow)?)
                }
                (NumericOperator::Subtract, a, b) => {
                    IntegerTree::Subtract(Box::new(a), Box::new(b))
                }
                (NumericOperator::Multiply, IntegerTree::Constant(a), IntegerTree::Constant(b)) => {
                    IntegerTree::Constant(a.checked_mul(b).ok_or_else(overflow)?)
                }
                (NumericOperator::Multiply, IntegerTree::Constant(factor), term)
                | (NumericOperator::Multiply, term, IntegerTree::Constant(factor)) => {
                    IntegerTree::Scale {
                        factor,
                        term: Box::new(term),
                    }
                }
                _ => {
                    return Err(integer_unsupported(
                        UnsupportedClass::NonLinearArithmetic,
                        path,
                        "product of two non-constant terms",
                    ))
                }
            };
            Ok(FoldedInteger {
                tree,
                value_type: left.value_type,
            })
        }
        _ => Err(unsupported(
            &path.render(),
            "construct is outside linear integer arithmetic",
        )),
    }
}

fn reject_saturating(value_type: &IntegerType, path: &ExprPath<'_>) -> Result<(), LoweringError> {
    if value_type.overflow() == OverflowPolicy::Saturate {
        Err(integer_unsupported(
            UnsupportedClass::SaturatingArithmetic,
            path,
            "a saturating result has no exact encoding",
        ))
    } else {
        Ok(())
    }
}

fn unsupported(path: &str, message: &str) -> LoweringError {
    LoweringError::new(LoweringErrorCode::UnsupportedConstruct, path, message)
}

fn construct_name(kind: &ExpressionKind) -> &'static str {
    match kind {
        ExpressionKind::IntegerLiteral { .. } => {
            "integer literal outside a comparison is unsupported"
        }
        ExpressionKind::RationalLiteral { .. } => "rational literals are unsupported",
        ExpressionKind::TextLiteral { .. } => "text literals are unsupported",
        ExpressionKind::EnumLiteral { .. } => "construct is supported",
        ExpressionKind::OptionNone { .. }
        | ExpressionKind::OptionSome { .. }
        | ExpressionKind::IsPresent { .. }
        | ExpressionKind::Unwrap { .. } => "option expressions are unsupported",
        ExpressionKind::RecordLiteral { .. } | ExpressionKind::FieldAccess { .. } => {
            "record expressions are unsupported"
        }
        ExpressionKind::CollectionLiteral { .. }
        | ExpressionKind::Length { .. }
        | ExpressionKind::Index { .. } => "collection expressions are unsupported",
        ExpressionKind::LocalReference { .. } | ExpressionKind::Quantifier { .. } => {
            "quantification is unsupported"
        }
        ExpressionKind::Call { .. } => "pure function calls are unsupported",
        ExpressionKind::Numeric { .. } | ExpressionKind::NumericNegate { .. } => {
            "rational arithmetic is unsupported"
        }
        ExpressionKind::Compare { .. } => "non-Boolean comparison is unsupported",
        ExpressionKind::BooleanLiteral { .. }
        | ExpressionKind::ValueReference { .. }
        | ExpressionKind::BooleanNot { .. }
        | ExpressionKind::Boolean { .. } => "construct is supported",
    }
}

pub fn type_shape_digest(
    environment: &DeclarationEnvironment,
    value_type: &ValueType,
) -> Result<AnalysisDigest, LoweringError> {
    let mut projection = Vec::new();
    project_type(environment, value_type, &mut projection)?;
    Ok(hash_fields(
        "type-shape",
        &[ANALYSIS_MODEL_PROFILE.as_bytes(), &projection],
    ))
}

fn project_type(
    environment: &DeclarationEnvironment,
    value_type: &ValueType,
    output: &mut Vec<u8>,
) -> Result<(), LoweringError> {
    match value_type {
        ValueType::Boolean => push_field(output, b"boolean"),
        ValueType::Integer { value } => {
            push_field(output, b"integer");
            push_field(
                output,
                match value.domain() {
                    IntegerDomain::Signed => b"signed",
                    IntegerDomain::Unsigned => b"unsigned",
                },
            );
            push_field(output, value.minimum().to_string().as_bytes());
            push_field(output, value.maximum().to_string().as_bytes());
            push_field(
                output,
                match value.overflow() {
                    OverflowPolicy::Reject => b"reject",
                    OverflowPolicy::Saturate => b"saturate",
                },
            );
        }
        ValueType::Rational { value } => {
            push_field(output, b"rational");
            push_field(output, value.numerator_minimum().to_string().as_bytes());
            push_field(output, value.numerator_maximum().to_string().as_bytes());
            push_field(output, value.maximum_denominator().to_string().as_bytes());
        }
        ValueType::Text => push_field(output, b"text"),
        ValueType::Option { value } => {
            push_field(output, b"option");
            project_type(environment, value, output)?;
        }
        ValueType::Collection { value } => {
            push_field(output, b"collection");
            push_field(output, value.maximum_items().to_string().as_bytes());
            project_type(environment, value.element(), output)?;
        }
        ValueType::Enum { name } => {
            push_field(output, b"enum");
            push_field(output, name.as_str().as_bytes());
            let declaration = environment
                .types()
                .iter()
                .find_map(|candidate| match candidate {
                    TypeDeclaration::Enum { declaration } if declaration.name() == name => {
                        Some(declaration)
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    LoweringError::new(
                        LoweringErrorCode::InvalidBinding,
                        "binding.type",
                        "enum declaration does not resolve",
                    )
                })?;
            let mut variants: Vec<_> = declaration.variants().iter().collect();
            variants.sort_by(|left, right| left.name().cmp(right.name()));
            for variant in variants {
                push_field(output, variant.name().as_str().as_bytes());
            }
        }
        ValueType::Record { name } => {
            push_field(output, b"record");
            push_field(output, name.as_str().as_bytes());
            let declaration = environment
                .types()
                .iter()
                .find_map(|candidate| match candidate {
                    TypeDeclaration::Record { declaration } if declaration.name() == name => {
                        Some(declaration)
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    LoweringError::new(
                        LoweringErrorCode::InvalidBinding,
                        "binding.type",
                        "record declaration does not resolve",
                    )
                })?;
            let mut fields: Vec<_> = declaration.fields().iter().collect();
            fields.sort_by(|left, right| left.name().cmp(right.name()));
            for field in fields {
                push_field(output, field.name().as_str().as_bytes());
                project_type(environment, field.value_type(), output)?;
            }
        }
    }
    Ok(())
}

fn statement_digest(statement: &StatementInput, binding_digest: AnalysisDigest) -> AnalysisDigest {
    hash_fields(
        "statement",
        &[
            ANALYSIS_MODEL_PROFILE.as_bytes(),
            statement.clause.requirement().package().as_str().as_bytes(),
            statement
                .clause
                .requirement()
                .requirement()
                .as_str()
                .as_bytes(),
            statement
                .clause
                .requirement()
                .revision()
                .get()
                .to_string()
                .as_bytes(),
            statement.clause.clause().as_str().as_bytes(),
            statement.clause_digest.as_bytes(),
            execution_point_key(&statement.execution_point).as_bytes(),
            binding_digest.as_bytes(),
        ],
    )
}

fn assertion_symbol(statement: &StatementInput, digest: AnalysisDigest) -> String {
    let reference = &statement.clause;
    format!(
        "a_p{}_r{}_v{}_c{}_d{}",
        hex(reference.requirement().package().as_str().as_bytes()),
        hex(reference.requirement().requirement().as_str().as_bytes()),
        reference.requirement().revision().get(),
        hex(reference.clause().as_str().as_bytes()),
        digest
    )
}

fn variable_key(
    requirement: &RequirementRef,
    kind: &str,
    observation: &str,
    name: &str,
    point: &str,
    digest: AnalysisDigest,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        requirement.package(),
        requirement.requirement(),
        requirement.revision().get(),
        kind,
        observation,
        name,
        point,
        digest
    )
}

fn declaration_kind(kind: ValueDeclarationKind) -> &'static str {
    match kind {
        ValueDeclarationKind::Input => "input",
        ValueDeclarationKind::State => "state",
    }
}
fn observation_name(value: StateObservation) -> &'static str {
    match value {
        StateObservation::Current => "current",
        StateObservation::Pre => "pre",
        StateObservation::Post => "post",
    }
}
fn execution_point_key(value: &ExecutionPoint) -> String {
    match value {
        ExecutionPoint::Initialization { name } => format!("initialization:{}", name.as_str()),
        ExecutionPoint::Handler { name } => format!("handler:{}", name.as_str()),
        ExecutionPoint::Pre { operation } => format!("pre:{}", operation.as_str()),
        ExecutionPoint::Post { operation } => format!("post:{}", operation.as_str()),
    }
}

fn valid_binding_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn hash_fields(domain: &str, fields: &[&[u8]]) -> AnalysisDigest {
    let mut hasher = Sha256::new();
    hasher.update(b"quire-analyze\0");
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    AnalysisDigest(hasher.finalize().into())
}

fn sort_errors(errors: &mut [LoweringError]) {
    errors.sort_by(|left, right| {
        (&left.path, &left.message, left.code).cmp(&(&right.path, &right.message, right.code))
    });
}

fn push_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u64).to_be_bytes());
    output.extend_from_slice(field);
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0xf) as usize] as char);
    }
    out
}

/// The logic a query's feature set requires and the one it declares, read by scanning commands.
pub(crate) struct QueryLogic {
    pub(crate) required: &'static str,
    pub(crate) declared: String,
}

/// Scans the top-level commands of a query, skipping comments, quoted symbols and string
/// literals. Returns `None` unless there is exactly one `set-logic` that precedes every
/// declaration. Datatypes need `QF_UFDTLIA`, otherwise `QF_LIA`.
pub(crate) fn query_logic(query: &[u8]) -> Option<QueryLogic> {
    let mut depth = 0_usize;
    let mut cursor = 0;
    let mut expect_head = false;
    let mut declared: Option<String> = None;
    let mut datatypes = false;
    let mut seen_declaration = false;
    let mut set_logics = 0_usize;
    let mut logic_argument = false;
    while cursor < query.len() {
        match query[cursor] {
            b';' => {
                while cursor < query.len() && query[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            b'|' => {
                cursor += 1;
                while cursor < query.len() && query[cursor] != b'|' {
                    cursor += 1;
                }
                if cursor >= query.len() {
                    return None;
                }
                cursor += 1;
                expect_head = false;
            }
            b'"' => {
                cursor += 1;
                loop {
                    match query.get(cursor) {
                        None => return None,
                        Some(b'"') if query.get(cursor + 1) == Some(&b'"') => cursor += 2,
                        Some(b'"') => break,
                        Some(_) => cursor += 1,
                    }
                }
                cursor += 1;
                expect_head = false;
            }
            b'(' => {
                depth += 1;
                expect_head = depth == 1;
                cursor += 1;
            }
            b')' => {
                depth = depth.checked_sub(1)?;
                expect_head = false;
                logic_argument = false;
                cursor += 1;
            }
            byte if byte.is_ascii_whitespace() => cursor += 1,
            _ => {
                let start = cursor;
                while cursor < query.len()
                    && !query[cursor].is_ascii_whitespace()
                    && !matches!(query[cursor], b'(' | b')' | b';' | b'|' | b'"')
                {
                    cursor += 1;
                }
                let atom = &query[start..cursor];
                if expect_head {
                    match atom {
                        b"set-logic" => {
                            if set_logics > 0 || seen_declaration {
                                return None;
                            }
                            set_logics += 1;
                            logic_argument = true;
                        }
                        b"declare-datatypes"
                        | b"declare-datatype"
                        | b"declare-codatatypes"
                        | b"declare-codatatype" => {
                            datatypes = true;
                            seen_declaration = true;
                        }
                        b"declare-fun" | b"declare-const" | b"declare-sort" | b"define-fun" => {
                            seen_declaration = true;
                        }
                        _ => {}
                    }
                } else if logic_argument && depth == 1 && declared.is_none() {
                    declared = Some(String::from_utf8_lossy(atom).into_owned());
                }
                expect_head = false;
            }
        }
    }
    if depth != 0 {
        return None;
    }
    Some(QueryLogic {
        required: if datatypes {
            SMTLIB2_DATATYPE_LOGIC
        } else {
            SMTLIB2_LOGIC
        },
        declared: declared?,
    })
}

/// Re-derive a query digest from the two identities a report retains.
///
/// `queryDigest` is `hash_fields("query", [profile, requestDigest, query])`, and
/// a differential report carries both `requestDigest` and the query bytes. That
/// makes the binding checkable from the document alone, so report validation
/// checks it rather than trusting a field that names itself.
pub(crate) fn derive_query_digest(
    request_digest_hex: &str,
    query: &[u8],
) -> Result<String, String> {
    if request_digest_hex.len() != 64 || !request_digest_hex.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("request digest is not a SHA-256 digest".to_owned());
    }
    let mut request = [0_u8; 32];
    for (index, slot) in request.iter_mut().enumerate() {
        let pair = &request_digest_hex[index * 2..index * 2 + 2];
        *slot = u8::from_str_radix(pair, 16).map_err(|_| "request digest is not hex".to_owned())?;
    }
    Ok(hash_fields("query", &[SMTLIB2_PROFILE.as_bytes(), &request, query]).to_string())
}

#[cfg(test)]
mod tests {
    use quire_contract_model::{SourceDocumentId, SourceIdentity, SourceLocation, SourceRevision};

    use super::*;

    fn span() -> SourceSpan {
        let source = SourceIdentity::new(
            SourceDocumentId::new("unit").expect("source id"),
            SourceRevision::new(1).expect("revision"),
        );
        SourceSpan::new(
            SourceLocation::new(source.clone(), 1, 1, 0).expect("start"),
            SourceLocation::new(source, 1, 2, 1).expect("end"),
        )
        .expect("span")
    }

    fn literal(value: i128, value_type: &IntegerType) -> Expression {
        Expression::new(
            ExpressionKind::IntegerLiteral {
                value,
                value_type: value_type.clone(),
            },
            span(),
        )
    }

    fn binary(operator: NumericOperator, left: Expression, right: Expression) -> Expression {
        Expression::new(
            ExpressionKind::Numeric {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            },
            span(),
        )
    }

    fn fold(expression: &Expression) -> Result<FoldedInteger, LoweringError> {
        fold_integer(expression, &ExprPath::ROOT, &|_| None, 1, &mut 0)
    }

    /// FR-010-AC-11: a constant fold outside the 128-bit signed range is refused, not wrapped.
    #[ix_trace_rs::trace("TC-016", "FR-010-AC-11")]
    #[test]
    fn constant_fold_overflow_is_unsupported() {
        let ty = IntegerType::new(
            IntegerDomain::Signed,
            i128::MIN,
            i128::MAX,
            OverflowPolicy::Reject,
        )
        .expect("type");
        let cases = [
            binary(
                NumericOperator::Add,
                literal(i128::MAX, &ty),
                literal(1, &ty),
            ),
            binary(
                NumericOperator::Subtract,
                literal(i128::MIN, &ty),
                literal(1, &ty),
            ),
            binary(
                NumericOperator::Multiply,
                literal(i128::MAX, &ty),
                literal(2, &ty),
            ),
            Expression::new(
                ExpressionKind::NumericNegate {
                    operand: Box::new(literal(i128::MIN, &ty)),
                },
                span(),
            ),
        ];
        for expression in &cases {
            let error = fold(expression).err().expect("overflow");
            assert_eq!(error.code(), LoweringErrorCode::UnsupportedConstruct);
            assert_eq!(
                error.unsupported_class(),
                Some(UnsupportedClass::ConstantOverflow)
            );
        }
        let exact = binary(
            NumericOperator::Add,
            literal(i128::MAX - 1, &ty),
            literal(1, &ty),
        );
        assert_eq!(
            fold(&exact).expect("in range").tree,
            IntegerTree::Constant(i128::MAX)
        );
    }

    fn logic_of(query: &str) -> Option<(&'static str, String)> {
        query_logic(query.as_bytes()).map(|logic| (logic.required, logic.declared))
    }

    /// FR-020-AC-9: the logic derivation reads commands, so comments, quoted symbols and strings
    /// cannot change it, singular datatype declarations count, and a repeated or late
    /// `set-logic` is refused.
    #[ix_trace_rs::trace("TC-020", "FR-020-AC-9")]
    #[test]
    fn logic_derivation_scans_commands_not_text() {
        let plain = "(set-logic QF_LIA)\n(declare-fun x () Bool)\n";
        assert_eq!(logic_of(plain), Some(("QF_LIA", "QF_LIA".to_owned())));
        for hidden in [
            "; (declare-datatypes ((S 0)) (((a))))\n",
            "(declare-fun |(declare-datatypes | () Bool)\n",
            "(set-info :note \"(declare-datatype S ((a)))\")\n",
        ] {
            let query = format!("(set-logic QF_LIA)\n{hidden}");
            assert_eq!(
                logic_of(&query),
                Some(("QF_LIA", "QF_LIA".to_owned())),
                "{hidden}"
            );
        }
        for command in [
            "(declare-datatype S ((a)))",
            "(declare-datatypes ((S 0)) (((a))))",
        ] {
            let query = format!("(set-logic QF_UFDTLIA)\n{command}\n");
            assert_eq!(
                logic_of(&query),
                Some(("QF_UFDTLIA", "QF_UFDTLIA".to_owned())),
                "{command}"
            );
        }
        let comment_logic = "; (set-logic QF_UF)\n(set-logic QF_LIA)\n";
        assert_eq!(
            logic_of(comment_logic),
            Some(("QF_LIA", "QF_LIA".to_owned()))
        );
        assert_eq!(logic_of("(set-logic QF_LIA)\n(set-logic QF_UF)\n"), None);
        assert_eq!(
            logic_of("(declare-fun x () Bool)\n(set-logic QF_LIA)\n"),
            None
        );
        assert_eq!(logic_of("(declare-fun x () Bool)\n"), None);
        assert_eq!(logic_of("(set-logic)\n(set-logic QF_LIA)\n"), None);
        assert_eq!(
            logic_of("(set-logic QF_LIA)\n(declare-fun |x () Bool)\n"),
            None
        );
    }

    fn unchecked_statement(
        types: Vec<TypeDeclaration>,
        values: Vec<quire_contract_model::ValueDeclaration>,
        expression: Expression,
    ) -> StatementInput {
        let package = quire_contract_model::PackageId::new("agent-ix/unit").expect("package");
        let requirement = quire_contract_model::RequirementId::new("REQ-U").expect("requirement");
        let revision = quire_contract_model::RequirementRevision::new(1).expect("revision");
        let owner = RequirementRef::new(package, requirement, revision);
        StatementInput {
            clause: ClauseRef::new(
                owner.clone(),
                quire_contract_model::ClauseId::new("C-U").expect("clause"),
            ),
            clause_digest: hash_fields("clause", &[b"unit"]),
            execution_point: ExecutionPoint::Pre {
                operation: quire_contract_model::AnchorName::new("check")
                    .expect("anchor")
                    .into(),
            },
            environment: DeclarationEnvironment::new(owner, types, values, vec![]).expect("env"),
            expression,
            source: span(),
        }
    }

    fn enum_declaration() -> TypeDeclaration {
        let variants = ["A", "B"]
            .iter()
            .map(|name| {
                quire_contract_model::EnumVariantDeclaration::new(
                    SymbolName::new(*name).expect("symbol"),
                    span(),
                )
            })
            .collect();
        TypeDeclaration::Enum {
            declaration: quire_contract_model::EnumDeclaration::new(
                SymbolName::new("E").expect("symbol"),
                span(),
                variants,
            )
            .expect("enum"),
        }
    }

    fn enum_literal(name: &str) -> Expression {
        Expression::new(
            ExpressionKind::EnumLiteral {
                enumeration: SymbolName::new("E").expect("symbol"),
                variant: SymbolName::new(name).expect("symbol"),
            },
            span(),
        )
    }

    /// FR-020-AC-5: the lowering refuses an enum ordering or mixed-sort comparison itself, over an
    /// expression that skipped statement-build validation.
    #[ix_trace_rs::trace("TC-020", "FR-020-AC-5")]
    #[test]
    fn lowering_refuses_unchecked_enum_ordering_and_mixed_sorts() {
        let compare = |operator, left, right| {
            Expression::new(
                ExpressionKind::Compare {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span(),
            )
        };
        let boolean = Expression::new(ExpressionKind::BooleanLiteral { value: true }, span());
        for expression in [
            compare(
                ComparisonOperator::Less,
                enum_literal("A"),
                enum_literal("B"),
            ),
            compare(ComparisonOperator::Equal, boolean, enum_literal("A")),
        ] {
            let statement = unchecked_statement(vec![enum_declaration()], vec![], expression);
            let errors = lower_boolean_statements(&[statement], &[]).expect_err("refused");
            assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedConstruct);
        }
    }
}
