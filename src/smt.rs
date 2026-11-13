use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use quire_contract_model::{
    BooleanOperator, CanonicalProfile, Clause, ClauseRef, ComparisonOperator, ContractPackage,
    DeclarationEnvironment, ExecutionPoint, Expression, ExpressionKind, IntegerDomain, IntegerType,
    NumericOperator, OverflowPolicy, RecordDeclaration, RecordLiteralField, Requirement,
    RequirementRef, SourceSpan, StateObservation, SymbolName, TypeDeclaration, TypedExpression,
    ValueDeclarationKind, ValueType,
};
use sha2::{Digest as _, Sha256};

use crate::datatype_replay::{ReplayDatum, ReplayItem, ReplayShape, ReplayValue};

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
/// Bound on fields of one record.
pub const MAX_RECORD_FIELDS: usize = 64;
/// Bound on the nesting depth of datatypes.
pub const MAX_DATATYPE_DEPTH: usize = 8;
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
        "record",
        "record_equality",
        "field_access",
        "option",
        "option_equality",
        "option_presence",
        "option_access",
    ],
    unsupported_constructs: &[
        "non_linear_arithmetic",
        "integer_division",
        "integer_remainder",
        "saturating_arithmetic",
        "constant_overflow",
        "rational",
        "text",
        "collection",
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

/// The encoded symbols of one record datatype, derived from the declaration environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordSortMap {
    /// The SMT-LIB sort symbol.
    pub sort: String,
    /// The declaring package.
    pub package: String,
    /// The declared record name.
    pub name: String,
    /// The SMT-LIB constructor symbol.
    pub constructor: String,
    /// The fields, in ascending field-name order.
    pub fields: Vec<RecordFieldMap>,
}

/// One record field and its encoded selector symbol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordFieldMap {
    /// The declared field name.
    pub field: String,
    /// The SMT-LIB selector symbol.
    pub selector: String,
    /// The SMT-LIB sort of the field: `Bool`, `Int` or a datatype sort symbol.
    pub sort: String,
}

/// The encoded symbols of one option datatype. One datatype serves every option over the same
/// payload sort.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptionSortMap {
    /// The SMT-LIB sort symbol.
    pub sort: String,
    /// The SMT-LIB sort of the payload: `Bool`, `Int` or a datatype sort symbol.
    pub payload_sort: String,
    /// The SMT-LIB symbol of the `none` constructor.
    pub none: String,
    /// The SMT-LIB symbol of the `some` constructor.
    pub some: String,
    /// The SMT-LIB symbol of the `some` selector.
    pub selector: String,
}

/// The datatypes and datatype-sorted variables a query uses.
#[derive(Default)]
struct DatatypeSet {
    enums: BTreeMap<String, EnumSortMap>,
    records: BTreeMap<String, RecordSortMap>,
    options: BTreeMap<String, OptionSortMap>,
    variable_sorts: BTreeMap<String, String>,
    /// The declared type of each datatype-sorted variable, by variable symbol.
    variable_shapes: BTreeMap<String, ReplayShape>,
    /// The range predicates of typed shapes with integer parts, by predicate symbol.
    range_predicates: BTreeMap<String, RangePredicate>,
    /// The predicate of each typed shape already examined, by shape identity; `None` when the
    /// shape has no integer part.
    range_checked: BTreeMap<String, Option<String>>,
    /// The range predicate asserted for each datatype-sorted variable, by variable symbol.
    variable_predicates: BTreeMap<String, String>,
}

/// A non-recursive predicate that holds when every integer part of a value is in its range.
struct RangePredicate {
    sort: String,
    body: String,
    /// The predicates the body calls.
    dependencies: Vec<String>,
}

impl DatatypeSet {
    fn len(&self) -> usize {
        self.enums.len() + self.records.len() + self.options.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The range predicate definitions in dependency order, ties broken by the predicate symbol.
    fn range_definitions(&self) -> Vec<String> {
        let mut pending: Vec<(&String, &RangePredicate)> = self.range_predicates.iter().collect();
        let mut ordered = Vec::with_capacity(pending.len());
        while let Some(position) = pending.iter().position(|(_, predicate)| {
            predicate
                .dependencies
                .iter()
                .all(|dependency| !pending.iter().any(|(symbol, _)| *symbol == dependency))
        }) {
            let (symbol, predicate) = pending.remove(position);
            ordered.push(format!(
                "(define-fun {symbol} ((x {})) Bool {})\n",
                predicate.sort, predicate.body
            ));
        }
        ordered
    }

    /// The declarations in dependency order, ties broken by the encoded sort symbol.
    fn declarations(&self) -> Vec<String> {
        let mut pending: BTreeMap<String, (Vec<&str>, String)> = BTreeMap::new();
        for sort in self.enums.values() {
            let mut text = format!("(declare-datatypes (({} 0)) ((", sort.sort);
            for (index, constructor) in sort.constructors.iter().enumerate() {
                if index > 0 {
                    text.push(' ');
                }
                text.push('(');
                text.push_str(&constructor.symbol);
                text.push(')');
            }
            text.push_str(")))\n");
            pending.insert(sort.sort.clone(), (Vec::new(), text));
        }
        for sort in self.records.values() {
            let mut text = format!(
                "(declare-datatypes (({} 0)) ((({}",
                sort.sort, sort.constructor
            );
            for field in &sort.fields {
                text.push_str(&format!(" ({} {})", field.selector, field.sort));
            }
            text.push_str("))))\n");
            let dependencies = sort
                .fields
                .iter()
                .map(|field| field.sort.as_str())
                .collect();
            pending.insert(sort.sort.clone(), (dependencies, text));
        }
        for sort in self.options.values() {
            let text = format!(
                "(declare-datatypes (({} 0)) ((({}) ({} ({} {})))))\n",
                sort.sort, sort.none, sort.some, sort.selector, sort.payload_sort
            );
            pending.insert(sort.sort.clone(), (vec![sort.payload_sort.as_str()], text));
        }
        let mut ordered = Vec::with_capacity(pending.len());
        while let Some(symbol) = pending
            .iter()
            .find(|(_, (dependencies, _))| {
                dependencies
                    .iter()
                    .all(|dependency| !pending.contains_key(*dependency))
            })
            .map(|(symbol, _)| symbol.clone())
        {
            if let Some((_, text)) = pending.remove(&symbol) {
                ordered.push(text);
            }
        }
        ordered
    }
}

/// A concrete model assignment: Boolean and integer values by query symbol.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ReplayAssignment {
    pub(crate) booleans: BTreeMap<String, bool>,
    pub(crate) integers: BTreeMap<String, i128>,
    /// Enum, record and option values by query symbol.
    pub(crate) datums: BTreeMap<String, ReplayValue>,
}

/// Why a replay expression produced no verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReplayFault {
    /// An intermediate result left the 128-bit signed range.
    Overflow,
    /// A referenced symbol has no value in the assignment.
    Unassigned,
    /// An `Unwrap` was evaluated on an absent option.
    AbsentOption,
    /// A term evaluated to a value of the wrong type for its position.
    IllTyped,
}

/// An integer-valued replay term, evaluated with checked 128-bit signed arithmetic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReplayTerm {
    Constant(i128),
    Variable(String),
    Negate(Box<Self>),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Scale {
        factor: i128,
        term: Box<Self>,
    },
    /// An integer read out of a datatype value.
    Datum(Box<ReplayDatum>),
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
            Self::Datum(datum) => match datum.evaluate(values)? {
                ReplayValue::Int(value) => Ok(value),
                _ => Err(ReplayFault::IllTyped),
            },
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
    /// A datatype-valued term. Evaluated as a Boolean it must read a Boolean out of the datatype.
    Datum(Box<ReplayDatum>),
    /// An integer-valued term lowered where a typed value is carried (a record field or payload).
    Integer(ReplayTerm),
    /// Whether an option value is present.
    IsPresent(Box<ReplayDatum>),
    /// Equality of two datatype values, or inequality when `negated`.
    DatumEqual {
        left: Box<ReplayDatum>,
        right: Box<ReplayDatum>,
        negated: bool,
    },
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
            Self::Datum(datum) => match datum.evaluate(values)? {
                ReplayValue::Bool(value) => Ok(value),
                _ => Err(ReplayFault::IllTyped),
            },
            Self::Integer(_) => Err(ReplayFault::IllTyped),
            Self::IsPresent(option) => match option.evaluate(values)? {
                ReplayValue::Option(value) => Ok(value.is_some()),
                _ => Err(ReplayFault::IllTyped),
            },
            Self::DatumEqual {
                left,
                right,
                negated,
            } => Ok((left.evaluate(values)? == right.evaluate(values)?) != *negated),
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
    record_sorts: Vec<RecordSortMap>,
    option_sorts: Vec<OptionSortMap>,
    datatype_shapes: BTreeMap<String, ReplayShape>,
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

    /// The symbols of datatype-sorted variables, in ascending order.
    pub(crate) fn datatype_shapes(&self) -> &BTreeMap<String, ReplayShape> {
        &self.datatype_shapes
    }


    /// Every datatype sort symbol the query declares, in ascending order.
    /// The enum datatypes the query declares, in ascending sort-symbol order.
    pub fn enum_sorts(&self) -> &[EnumSortMap] {
        &self.enum_sorts
    }

    /// The record datatypes the query declares, in ascending sort-symbol order.
    pub fn record_sorts(&self) -> &[RecordSortMap] {
        &self.record_sorts
    }

    /// The option datatypes the query declares, in ascending sort-symbol order.
    pub fn option_sorts(&self) -> &[OptionSortMap] {
        &self.option_sorts
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
    let logic = if datatypes.is_empty() {
        SMTLIB2_LOGIC
    } else {
        SMTLIB2_DATATYPE_LOGIC
    };
    query.push_str(logic);
    query.push_str(")\n");
    for declaration in datatypes
        .declarations()
        .into_iter()
        .chain(datatypes.range_definitions())
    {
        query.push_str(&declaration);
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
    for (symbol, predicate) in &datatypes.variable_predicates {
        query.push_str(&format!("(assert ({predicate} {symbol}))\n"));
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
        record_sorts: datatypes.records.into_values().collect(),
        option_sorts: datatypes.options.into_values().collect(),
        datatype_shapes: datatypes.variable_shapes,
        replay_assertions,
        integer_ranges: ranges,
    })
}

type BindingLookup = BTreeMap<String, String>;

struct LoweredExpression {
    smt: String,
    replay: ReplayExpression,
}

impl ReplayExpression {
    /// The datatype-valued form of a lowered datatype term.
    fn into_datum(self) -> Result<ReplayDatum, LoweringError> {
        match self {
            Self::Datum(datum) => Ok(*datum),
            _ => Err(LoweringError::new(
                LoweringErrorCode::InvalidStatement,
                "expression",
                "validated datatype operand no longer lowers to a datatype term",
            )),
        }
    }

    /// The typed item a lowered term stands for as a record field or option payload.
    fn into_item(self) -> ReplayItem {
        match self {
            Self::Datum(datum) => ReplayItem::Datum(*datum),
            Self::Integer(term) => ReplayItem::Int(term),
            other => ReplayItem::Bool(other),
        }
    }
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
        built_records: BTreeMap::new(),
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
    /// The record sorts already built for this statement's environment, by record name.
    built_records: BTreeMap<SymbolName, BuiltSort>,
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
                let replay = if self.datatypes.variable_shapes.contains_key(&symbol) {
                    ReplayExpression::Datum(Box::new(ReplayDatum::Variable(symbol.clone())))
                } else {
                    ReplayExpression::Variable(symbol.clone())
                };
                Ok(LoweredExpression {
                    smt: symbol,
                    replay,
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
                    replay: ReplayExpression::Datum(Box::new(ReplayDatum::Enum(
                        constructor.clone(),
                    ))),
                    smt: constructor,
                })
            }
            ExpressionKind::RecordLiteral { record, fields } => {
                self.lower_record_literal(record, fields, depth, path)
            }
            ExpressionKind::FieldAccess { base, field } => {
                self.lower_field_access(base, field, depth, path)
            }
            ExpressionKind::OptionNone { value_type } => self.lower_option_none(value_type),
            ExpressionKind::OptionSome { value_type, value } => {
                self.lower_option_some(value_type, value, depth, path)
            }
            ExpressionKind::IsPresent { option } | ExpressionKind::Unwrap { option } => {
                self.lower_option_read(expression, option, depth, path)
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
            } => self.lower_comparison(operator, left, right, depth, path),
            other => Err(unsupported(&path.render(), construct_name(other))),
        }
    }

    fn lower_record_literal(
        &mut self,
        record: &SymbolName,
        fields: &[RecordLiteralField],
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
        let built = self.record_sort(record, 1, "")?;
        let map = self.datatypes.records[&built.sort].clone();
        if fields.len() != map.fields.len() {
            return Err(LoweringError::new(
                LoweringErrorCode::InvalidStatement,
                "expression.record_literal",
                "validated record literal no longer matches its declaration",
            ));
        }
        let mut smt = format!("({}", map.constructor);
        let mut items = Vec::with_capacity(map.fields.len());
        for field in &map.fields {
            let value = fields
                .iter()
                .find(|candidate| candidate.name().as_str() == field.field)
                .ok_or_else(|| {
                    LoweringError::new(
                        LoweringErrorCode::InvalidStatement,
                        "expression.record_literal",
                        "validated record field no longer resolves",
                    )
                })?;
            let value = self.lower_term(value.value(), depth + 1, &path.child("field"))?;
            smt.push(' ');
            smt.push_str(&value.smt);
            items.push(value.replay.into_item());
        }
        smt.push(')');
        Ok(LoweredExpression {
            smt,
            replay: ReplayExpression::Datum(Box::new(ReplayDatum::Record(items))),
        })
    }

    fn lower_field_access(
        &mut self,
        base: &Expression,
        field: &SymbolName,
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
        let Some(ValueType::Record { name }) = term_type(&self.statement.environment, base) else {
            return Err(unsupported(
                &path.render(),
                "field access requires a record-typed base",
            ));
        };
        let base = self.lower(base, depth + 1, &path.child("base"))?;
        let built = self.record_sort(&name, 1, "")?;
        let (index, selector) = self.datatypes.records[&built.sort]
            .fields
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.field == field.as_str())
            .map(|(index, candidate)| (index, candidate.selector.clone()))
            .ok_or_else(|| {
                LoweringError::new(
                    LoweringErrorCode::InvalidStatement,
                    "expression.field_access",
                    "validated record field no longer resolves",
                )
            })?;
        Ok(LoweredExpression {
            smt: format!("({selector} {})", base.smt),
            replay: ReplayExpression::Datum(Box::new(ReplayDatum::Field {
                base: Box::new(base.replay.into_datum()?),
                index,
            })),
        })
    }

    fn lower_option_none(
        &mut self,
        value_type: &ValueType,
    ) -> Result<LoweredExpression, LoweringError> {
        let map = self.option_map(value_type, "expression.option_none")?;
        Ok(LoweredExpression {
            smt: map.none,
            replay: ReplayExpression::Datum(Box::new(ReplayDatum::None)),
        })
    }

    fn lower_option_some(
        &mut self,
        value_type: &ValueType,
        value: &Expression,
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
        let map = self.option_map(value_type, "expression.option_some")?;
        let value = self.lower_term(value, depth + 1, &path.child("value"))?;
        Ok(LoweredExpression {
            smt: format!("({} {})", map.some, value.smt),
            replay: ReplayExpression::Datum(Box::new(ReplayDatum::Some(Box::new(
                value.replay.into_item(),
            )))),
        })
    }

    fn lower_option_read(
        &mut self,
        expression: &Expression,
        option: &Expression,
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
        let Some(option_type @ ValueType::Option { .. }) =
            term_type(&self.statement.environment, option)
        else {
            return Err(unsupported(
                &path.render(),
                "option access requires an option-typed operand",
            ));
        };
        let operand = self.lower(option, depth + 1, &path.child("option"))?;
        let map = self.option_map(&option_type, "expression.option")?;
        let option = Box::new(operand.replay.into_datum()?);
        let (smt, replay) = if matches!(expression.kind(), ExpressionKind::IsPresent { .. }) {
            (
                format!("((_ is {}) {})", map.some, operand.smt),
                ReplayExpression::IsPresent(option),
            )
        } else {
            (
                format!("({} {})", map.selector, operand.smt),
                ReplayExpression::Datum(Box::new(ReplayDatum::Unwrap(option))),
            )
        };
        Ok(LoweredExpression { smt, replay })
    }

    fn lower_comparison(
        &mut self,
        operator: &ComparisonOperator,
        left: &Expression,
        right: &Expression,
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
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
        let environment = &self.statement.environment;
        let left_type = term_type(environment, left);
        let right_type = term_type(environment, right);
        let left = self.lower(left, depth + 1, &path.child("left"))?;
        let right = self.lower(right, depth + 1, &path.child("right"))?;
        let left_sort = self.comparable_sort(left_type.as_ref())?;
        let right_sort = self.comparable_sort(right_type.as_ref())?;
        let Some(sort) = left_sort.filter(|sort| right_sort.as_ref() == Some(sort)) else {
            return Err(unsupported(
                &path.render(),
                "comparison operands must both be Boolean or both of one datatype sort",
            ));
        };
        if sort != "Bool" {
            let name = if *operator == ComparisonOperator::Equal {
                "="
            } else {
                "distinct"
            };
            return Ok(LoweredExpression {
                smt: format!("({name} {} {})", left.smt, right.smt),
                replay: ReplayExpression::DatumEqual {
                    left: Box::new(left.replay.into_datum()?),
                    right: Box::new(right.replay.into_datum()?),
                    negated: *operator != ComparisonOperator::Equal,
                },
            });
        }
        Ok(if *operator == ComparisonOperator::Equal {
            LoweredExpression {
                smt: format!("(= {} {})", left.smt, right.smt),
                replay: ReplayExpression::Equal(Box::new(left.replay), Box::new(right.replay)),
            }
        } else {
            LoweredExpression {
                smt: format!("(not (= {} {}))", left.smt, right.smt),
                replay: ReplayExpression::NotEqual(Box::new(left.replay), Box::new(right.replay)),
            }
        })
    }

    /// Whether the term is integer-typed. Validation makes both comparison operands agree, so
    /// the left operand decides.
    fn is_integer_term(&self, expression: &Expression) -> bool {
        matches!(
            term_type(&self.statement.environment, expression),
            Some(ValueType::Integer { .. })
        )
    }

    /// Lowers a term of any exact sort: an integer term through linear folding, anything else
    /// through the expression lowering.
    fn lower_term(
        &mut self,
        expression: &Expression,
        depth: usize,
        path: &ExprPath<'_>,
    ) -> Result<LoweredExpression, LoweringError> {
        if !self.is_integer_term(expression) {
            return self.lower(expression, depth, path);
        }
        let environment = &self.statement.environment;
        let types = |name: &SymbolName| integer_declaration_type(environment, name);
        let terms = |term: &Expression| datatype_integer_type(environment, term);
        let folded = fold_integer(expression, path, &types, &terms, depth, &mut self.nodes)?;
        let (smt, term) = self.emit(&folded.tree)?;
        Ok(LoweredExpression {
            smt,
            replay: ReplayExpression::Integer(term),
        })
    }

    /// The SMT-LIB sort of a comparable term type: `Bool`, or the symbol of an enum, record or
    /// option datatype. Any other type is not comparable.
    fn comparable_sort(
        &mut self,
        value_type: Option<&ValueType>,
    ) -> Result<Option<String>, LoweringError> {
        Ok(match value_type {
            Some(ValueType::Boolean) => Some("Bool".to_owned()),
            Some(
                datatype @ (ValueType::Enum { .. }
                | ValueType::Record { .. }
                | ValueType::Option { .. }),
            ) => Some(self.sort_of(datatype, 1, "")?.sort),
            _ => None,
        })
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
        self.reserve_datatype()?;
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

    /// Refuses a datatype that would exceed the per-query bound.
    fn reserve_datatype(&self) -> Result<(), LoweringError> {
        if self.datatypes.len() >= MAX_QUERY_DATATYPES {
            return Err(LoweringError::new(
                LoweringErrorCode::ResourceLimit,
                "type.datatypes",
                "datatype count exceeds the public bound",
            ));
        }
        Ok(())
    }

    /// Builds the sort of a value type, declaring every datatype it needs. `level` is the nesting
    /// depth of the type within the outermost datatype and `path` names it for a refusal.
    fn sort_of(
        &mut self,
        value_type: &ValueType,
        level: usize,
        path: &str,
    ) -> Result<BuiltSort, LoweringError> {
        match value_type {
            ValueType::Boolean => Ok(BuiltSort {
                sort: "Bool".to_owned(),
                height: 0,
                identity: "bool".to_owned(),
            }),
            ValueType::Integer { value } => Ok(BuiltSort {
                sort: "Int".to_owned(),
                height: 0,
                identity: format!(
                    "int:{}:{}:{}:{}",
                    match value.domain() {
                        IntegerDomain::Signed => "signed",
                        IntegerDomain::Unsigned => "unsigned",
                    },
                    value.minimum(),
                    value.maximum(),
                    match value.overflow() {
                        OverflowPolicy::Reject => "reject",
                        OverflowPolicy::Saturate => "saturate",
                    }
                ),
            }),
            ValueType::Text | ValueType::Rational { .. } | ValueType::Collection { .. } => {
                Err(LoweringError::new(
                    LoweringErrorCode::UnsupportedType,
                    path,
                    "a record or option nests a collection, text or rational value, which has no exact datatype field",
                ))
            }
            ValueType::Enum { name } => {
                let map = self.enum_sort(name)?;
                Ok(BuiltSort {
                    identity: format!("enum:{}", map.sort),
                    sort: map.sort,
                    height: 1,
                })
            }
            ValueType::Record { name } => self.record_sort(name, level, path),
            ValueType::Option { value } => self.option_sort(value, level, path),
        }
    }

    /// Declares the record datatype of the statement environment on first use.
    fn record_sort(
        &mut self,
        name: &SymbolName,
        level: usize,
        path: &str,
    ) -> Result<BuiltSort, LoweringError> {
        if let Some(built) = self.built_records.get(name) {
            return Ok(built.clone());
        }
        if level > MAX_DATATYPE_DEPTH {
            return Err(depth_limit());
        }
        let path = if path.is_empty() {
            format!("type.{}", name.as_str())
        } else {
            path.to_owned()
        };
        let statement = self.statement;
        let environment = &statement.environment;
        let declaration = record_declaration(environment, name).ok_or_else(|| {
            LoweringError::new(
                LoweringErrorCode::InvalidBinding,
                "binding.type",
                "record declaration does not resolve",
            )
        })?;
        if declaration.fields().len() > MAX_RECORD_FIELDS {
            return Err(LoweringError::new(
                LoweringErrorCode::ResourceLimit,
                "type.record.fields",
                "record field count exceeds the public bound",
            ));
        }
        let mut declared: Vec<_> = declaration.fields().iter().collect();
        declared.sort_by(|left, right| left.name().cmp(right.name()));
        let mut fields = Vec::with_capacity(declared.len());
        let mut identity = Vec::new();
        let mut height = 0;
        for field in declared {
            let field_path = format!("{path}.{}", field.name().as_str());
            let built = self.sort_of(field.value_type(), level + 1, &field_path)?;
            height = height.max(built.height);
            push_field(&mut identity, field.name().as_str().as_bytes());
            push_field(&mut identity, built.identity.as_bytes());
            fields.push((field.name().as_str(), built.sort));
        }
        let height = height + 1;
        if height > MAX_DATATYPE_DEPTH {
            return Err(depth_limit());
        }
        let package = environment.owner().package().as_str();
        let digest = hex(hash_fields(
            "record-sort",
            &[package.as_bytes(), name.as_str().as_bytes(), &identity],
        )
        .as_bytes());
        let sort = format!("r_{digest}");
        let built = BuiltSort {
            identity: format!("record:{sort}"),
            sort: sort.clone(),
            height,
        };
        if !self.datatypes.records.contains_key(&sort) {
            self.reserve_datatype()?;
            let map = RecordSortMap {
                sort: sort.clone(),
                package: package.to_owned(),
                name: name.as_str().to_owned(),
                constructor: format!("k_{digest}"),
                fields: fields
                    .into_iter()
                    .map(|(field, sort)| RecordFieldMap {
                        selector: format!("f_{digest}_{}", hex(field.as_bytes())),
                        field: field.to_owned(),
                        sort,
                    })
                    .collect(),
            };
            self.datatypes.records.insert(sort, map);
        }
        self.built_records.insert(name.clone(), built.clone());
        Ok(built)
    }

    /// Declares the option datatype over a payload type on first use.
    fn option_sort(
        &mut self,
        payload: &ValueType,
        level: usize,
        path: &str,
    ) -> Result<BuiltSort, LoweringError> {
        if level > MAX_DATATYPE_DEPTH {
            return Err(depth_limit());
        }
        let path = if path.is_empty() { "type.option" } else { path };
        let payload = self.sort_of(payload, level + 1, &format!("{path}.some"))?;
        let height = payload.height + 1;
        if height > MAX_DATATYPE_DEPTH {
            return Err(depth_limit());
        }
        let digest = hex(hash_fields("option-sort", &[payload.sort.as_bytes()]).as_bytes());
        let sort = format!("o_{digest}");
        if !self.datatypes.options.contains_key(&sort) {
            self.reserve_datatype()?;
            self.datatypes.options.insert(
                sort.clone(),
                OptionSortMap {
                    sort: sort.clone(),
                    payload_sort: payload.sort,
                    none: format!("n_{digest}"),
                    some: format!("m_{digest}"),
                    selector: format!("u_{digest}"),
                },
            );
        }
        Ok(BuiltSort {
            sort,
            height,
            identity: format!("option:{}", payload.identity),
        })
    }

    /// The option datatype of an option type; a non-option type no longer matches its validation.
    fn option_map(
        &mut self,
        value_type: &ValueType,
        path: &str,
    ) -> Result<OptionSortMap, LoweringError> {
        if !matches!(value_type, ValueType::Option { .. }) {
            return Err(LoweringError::new(
                LoweringErrorCode::InvalidStatement,
                path,
                "validated option type no longer names an option",
            ));
        }
        let built = self.sort_of(value_type, 1, "")?;
        Ok(self.datatypes.options[&built.sort].clone())
    }

    /// The declared type of a value, with the encoded sort symbols of its datatypes.
    fn replay_shape(&mut self, value_type: &ValueType) -> Result<ReplayShape, LoweringError> {
        Ok(match value_type {
            ValueType::Boolean => ReplayShape::Bool,
            ValueType::Integer { value } => ReplayShape::Int {
                minimum: value.minimum(),
                maximum: value.maximum(),
            },
            ValueType::Enum { .. } => ReplayShape::Enum {
                sort: self.sort_of(value_type, 1, "")?.sort,
            },
            ValueType::Record { name } => {
                let sort = self.sort_of(value_type, 1, "")?.sort;
                let map = self.datatypes.records[&sort].clone();
                let statement = self.statement;
                let declaration = record_declaration(&statement.environment, name);
                let mut fields = Vec::with_capacity(map.fields.len());
                for field in &map.fields {
                    let declared = declaration
                        .and_then(|declaration| {
                            declaration
                                .fields()
                                .iter()
                                .find(|candidate| candidate.name().as_str() == field.field)
                        })
                        .ok_or_else(|| {
                            LoweringError::new(
                                LoweringErrorCode::InvalidStatement,
                                "expression.value_reference",
                                "validated record field no longer resolves",
                            )
                        })?;
                    fields.push(self.replay_shape(declared.value_type())?);
                }
                ReplayShape::Record { sort, fields }
            }
            ValueType::Option { value } => ReplayShape::Option {
                sort: self.sort_of(value_type, 1, "")?.sort,
                payload: Box::new(self.replay_shape(value)?),
            },
            _ => return Err(unsupported(
                "expression.value_reference",
                "only Boolean, integer, enum, record and option parts have an exact replay shape",
            )),
        })
    }

    /// The range predicate of a record or option type, defined on first use, or `None` when the
    /// type has no integer part. One predicate serves every variable of the same typed shape, so
    /// the query grows with the number of shapes and variables, not with the integer leaves.
    fn range_predicate(&mut self, value_type: &ValueType) -> Result<Option<String>, LoweringError> {
        if !matches!(
            value_type,
            ValueType::Record { .. } | ValueType::Option { .. }
        ) {
            return Ok(None);
        }
        let built = self.sort_of(value_type, 1, "")?;
        if let Some(checked) = self.datatypes.range_checked.get(&built.identity) {
            return Ok(checked.clone());
        }
        let symbol = format!(
            "p_{}",
            hex(hash_fields("range-predicate", &[built.identity.as_bytes()]).as_bytes())
        );
        // (selected term, type) pairs the predicate constrains.
        let mut parts: Vec<(String, ValueType)> = Vec::new();
        let guard = match value_type {
            ValueType::Record { name } => {
                let map = self.datatypes.records[&built.sort].clone();
                let statement = self.statement;
                if let Some(declaration) = record_declaration(&statement.environment, name) {
                    for field in &map.fields {
                        if let Some(declared) = declaration
                            .fields()
                            .iter()
                            .find(|candidate| candidate.name().as_str() == field.field)
                        {
                            parts.push((
                                format!("({} x)", field.selector),
                                declared.value_type().clone(),
                            ));
                        }
                    }
                }
                None
            }
            ValueType::Option { value } => {
                let map = self.datatypes.options[&built.sort].clone();
                parts.push((format!("({} x)", map.selector), (**value).clone()));
                Some(format!("((_ is {}) x)", map.some))
            }
            _ => None,
        };
        let mut conjuncts = Vec::new();
        let mut dependencies = Vec::new();
        for (term, part_type) in parts {
            match &part_type {
                ValueType::Integer { value } => conjuncts.push(format!(
                    "(and (<= {} {term}) (<= {term} {}))",
                    numeral(value.minimum()),
                    numeral(value.maximum())
                )),
                _ => {
                    if let Some(called) = self.range_predicate(&part_type)? {
                        conjuncts.push(format!("({called} {term})"));
                        dependencies.push(called);
                    }
                }
            }
        }
        let predicate = match (conjuncts.len(), guard) {
            (0, _) => None,
            (_, guard) => {
                let body = if conjuncts.len() == 1 {
                    conjuncts.remove(0)
                } else {
                    format!("(and {})", conjuncts.join(" "))
                };
                let body = match guard {
                    Some(guard) => format!("(=> {guard} {body})"),
                    None => body,
                };
                self.datatypes.range_predicates.insert(
                    symbol.clone(),
                    RangePredicate {
                        sort: built.sort.clone(),
                        body,
                        dependencies,
                    },
                );
                Some(symbol)
            }
        };
        self.datatypes
            .range_checked
            .insert(built.identity, predicate.clone());
        Ok(predicate)
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
        let terms = |term: &Expression| datatype_integer_type(environment, term);
        let left = fold_integer(
            left,
            &path.child("left"),
            &types,
            &terms,
            depth + 1,
            &mut self.nodes,
        )?;
        let right = fold_integer(
            right,
            &path.child("right"),
            &types,
            &terms,
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
            IntegerTree::Datatype {
                expression,
                depth,
                path,
            } => {
                // `fold_integer` already counted this node, so lowering it must not count it again.
                self.nodes = self.nodes.saturating_sub(1);
                let lowered = self.lower(expression, *depth, &ExprPath::detached(path))?;
                (
                    lowered.smt,
                    ReplayTerm::Datum(Box::new(lowered.replay.into_datum()?)),
                )
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
        let mut datatype_sort = None;
        let mut datatype_value = None;
        let range = match declaration.value_type() {
            ValueType::Boolean => None,
            ValueType::Integer { value } => Some((value.minimum(), value.maximum())),
            datatype @ (ValueType::Enum { .. }
            | ValueType::Record { .. }
            | ValueType::Option { .. }) => {
                let path = match datatype {
                    ValueType::Option { .. } => format!("value.{}", name.as_str()),
                    _ => String::new(),
                };
                datatype_sort = Some(self.sort_of(datatype, 1, &path)?.sort);
                datatype_value = Some(datatype);
                None
            }
            _ => return Err(unsupported(
                "expression.value_reference",
                "only Boolean, integer, enum, record and option value references are exact in quire.smtlib2/v2",
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
        if let Some(sort) = datatype_sort {
            self.datatypes.variable_sorts.insert(symbol.clone(), sort);
        }
        if let Some(value_type) = datatype_value {
            let shape = self.replay_shape(value_type)?;
            self.datatypes.variable_shapes.insert(symbol.clone(), shape);
            if let Some(predicate) = self.range_predicate(value_type)? {
                self.datatypes
                    .variable_predicates
                    .insert(symbol.clone(), predicate);
            }
        }
        Ok(symbol)
    }
}

/// A built datatype sort or scalar sort and what its identity depends on.
#[derive(Clone, Debug)]
struct BuiltSort {
    /// The SMT-LIB sort: `Bool`, `Int` or a datatype sort symbol.
    sort: String,
    /// The datatype nesting depth: 0 for a scalar sort, 1 for an enum.
    height: usize,
    /// The complete semantic identity, which keeps integer ranges apart that share the `Int` sort.
    identity: String,
}

/// The type of a lowered term, when the environment determines it.
fn term_type(environment: &DeclarationEnvironment, expression: &Expression) -> Option<ValueType> {
    match expression.kind() {
        ExpressionKind::BooleanLiteral { .. }
        | ExpressionKind::BooleanNot { .. }
        | ExpressionKind::Boolean { .. }
        | ExpressionKind::Compare { .. }
        | ExpressionKind::IsPresent { .. } => Some(ValueType::Boolean),
        ExpressionKind::IntegerLiteral { value_type, .. } => {
            Some(ValueType::integer(value_type.clone()))
        }
        ExpressionKind::EnumLiteral { enumeration, .. } => Some(ValueType::Enum {
            name: enumeration.clone(),
        }),
        ExpressionKind::RecordLiteral { record, .. } => Some(ValueType::Record {
            name: record.clone(),
        }),
        ExpressionKind::OptionNone { value_type }
        | ExpressionKind::OptionSome { value_type, .. } => Some(value_type.clone()),
        ExpressionKind::ValueReference { name, .. } => environment
            .values()
            .iter()
            .find(|value| value.name() == name)
            .map(|declaration| declaration.value_type().clone()),
        ExpressionKind::FieldAccess { base, field } => {
            let ValueType::Record { name } = term_type(environment, base)? else {
                return None;
            };
            record_declaration(environment, &name)?
                .fields()
                .iter()
                .find(|candidate| candidate.name() == field)
                .map(|candidate| candidate.value_type().clone())
        }
        ExpressionKind::Unwrap { option } => match term_type(environment, option)? {
            ValueType::Option { value } => Some(*value),
            _ => None,
        },
        ExpressionKind::Numeric { left, .. } => term_type(environment, left),
        ExpressionKind::NumericNegate { operand } => term_type(environment, operand),
        _ => None,
    }
}

/// The integer type of a field access or unwrap term, when it reads an integer out of a datatype.
fn datatype_integer_type(
    environment: &DeclarationEnvironment,
    expression: &Expression,
) -> Option<IntegerType> {
    match expression.kind() {
        ExpressionKind::FieldAccess { .. } | ExpressionKind::Unwrap { .. } => {
            match term_type(environment, expression)? {
                ValueType::Integer { value } => Some(value),
                _ => None,
            }
        }
        _ => None,
    }
}

fn record_declaration<'a>(
    environment: &'a DeclarationEnvironment,
    name: &SymbolName,
) -> Option<&'a RecordDeclaration> {
    environment
        .types()
        .iter()
        .find_map(|candidate| match candidate {
            TypeDeclaration::Record { declaration } if declaration.name() == name => {
                Some(declaration)
            }
            _ => None,
        })
}

/// The position of an expression within the statement, rendered only when an error needs it.
struct ExprPath<'a> {
    parent: Option<&'a ExprPath<'a>>,
    segment: &'a str,
}

impl<'a> ExprPath<'a> {
    const ROOT: ExprPath<'static> = ExprPath {
        parent: None,
        segment: "expression",
    };

    /// A path that renders as `rendered`, for a position recorded earlier.
    fn detached(rendered: &'a str) -> ExprPath<'a> {
        ExprPath {
            parent: None,
            segment: rendered,
        }
    }

    fn child(&'a self, segment: &'a str) -> ExprPath<'a> {
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
    /// An integer read out of a datatype, lowered when the tree is emitted.
    Datatype {
        expression: Box<Expression>,
        depth: usize,
        path: String,
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
    terms: &dyn Fn(&Expression) -> Option<IntegerType>,
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
            let operand = fold_integer(
                operand,
                &path.child("operand"),
                types,
                terms,
                depth + 1,
                nodes,
            )?;
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
            let left = fold_integer(left, &path.child("left"), types, terms, depth + 1, nodes)?;
            let right = fold_integer(right, &path.child("right"), types, terms, depth + 1, nodes)?;
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
        ExpressionKind::FieldAccess { .. } | ExpressionKind::Unwrap { .. } => {
            let value_type = terms(expression).ok_or_else(|| {
                unsupported(
                    &path.render(),
                    "only integer datatype reads are exact in an integer term",
                )
            })?;
            Ok(FoldedInteger {
                tree: IntegerTree::Datatype {
                    expression: Box::new(expression.clone()),
                    depth,
                    path: path.render(),
                },
                value_type,
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

fn depth_limit() -> LoweringError {
    LoweringError::new(
        LoweringErrorCode::ResourceLimit,
        "type.depth",
        "datatype nesting depth exceeds the public bound",
    )
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
        ExpressionKind::EnumLiteral { .. }
        | ExpressionKind::OptionNone { .. }
        | ExpressionKind::OptionSome { .. }
        | ExpressionKind::IsPresent { .. }
        | ExpressionKind::Unwrap { .. }
        | ExpressionKind::RecordLiteral { .. }
        | ExpressionKind::FieldAccess { .. } => "construct is supported",
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
        fold_integer(expression, &ExprPath::ROOT, &|_| None, &|_| None, 1, &mut 0)
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

    fn record_declaration_with(name: &str, fields: &[(&str, ValueType)]) -> TypeDeclaration {
        let fields = fields
            .iter()
            .map(|(field, value_type)| {
                quire_contract_model::RecordFieldDeclaration::new(
                    SymbolName::new(*field).expect("symbol"),
                    value_type.clone(),
                    span(),
                )
            })
            .collect();
        TypeDeclaration::Record {
            declaration: RecordDeclaration::new(
                SymbolName::new(name).expect("symbol"),
                span(),
                fields,
            )
            .expect("record"),
        }
    }

    fn reference(name: &str) -> Expression {
        Expression::new(
            ExpressionKind::ValueReference {
                name: SymbolName::new(name).expect("symbol"),
                observation: StateObservation::Current,
            },
            span(),
        )
    }

    fn input(name: &str, value_type: ValueType) -> quire_contract_model::ValueDeclaration {
        quire_contract_model::ValueDeclaration::new(
            SymbolName::new(name).expect("symbol"),
            quire_contract_model::ValueDeclarationKind::Input,
            value_type,
            span(),
        )
    }

    /// FR-020-AC-4: the lowering adds no presence check of its own, so an `Unwrap` that skipped the
    /// upstream check lowers to the selector.
    #[ix_trace_rs::trace("TC-020", "FR-020-AC-4")]
    #[test]
    fn lowering_adds_no_presence_check_over_an_unchecked_unwrap() {
        let option = ValueType::option(ValueType::Boolean);
        let unwrap = Expression::new(
            ExpressionKind::Unwrap {
                option: Box::new(reference("flag")),
            },
            span(),
        );
        let statement = unchecked_statement(vec![], vec![input("flag", option)], unwrap);
        let query = lower_boolean_statements(&[statement], &[]).expect("lowers");
        let selector = &query.option_sorts()[0].selector;
        assert!(query.query().contains(&format!("({selector} v_s_")));
    }

    /// FR-020-AC-5: the lowering refuses a record or option ordering comparison and a mixed-sort
    /// comparison itself, over an expression that skipped statement-build validation.
    #[ix_trace_rs::trace("TC-020", "FR-020-AC-5")]
    #[test]
    fn lowering_refuses_unchecked_record_and_option_comparisons() {
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
        let types = || {
            vec![
                record_declaration_with("A", &[("on", ValueType::Boolean)]),
                record_declaration_with("B", &[("on", ValueType::Boolean)]),
            ]
        };
        let values = || {
            vec![
                input(
                    "a",
                    ValueType::Record {
                        name: SymbolName::new("A").expect("symbol"),
                    },
                ),
                input(
                    "b",
                    ValueType::Record {
                        name: SymbolName::new("B").expect("symbol"),
                    },
                ),
                input("o", ValueType::option(ValueType::Boolean)),
                input("flag", ValueType::Boolean),
            ]
        };
        for expression in [
            compare(ComparisonOperator::Less, reference("a"), reference("a")),
            compare(ComparisonOperator::Less, reference("o"), reference("o")),
            compare(ComparisonOperator::Equal, reference("a"), reference("b")),
            compare(ComparisonOperator::Equal, reference("a"), reference("o")),
            compare(ComparisonOperator::Equal, reference("flag"), reference("o")),
        ] {
            let statement = unchecked_statement(types(), values(), expression);
            let errors = lower_boolean_statements(&[statement], &[]).expect_err("refused");
            assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedConstruct);
        }
    }

    /// NFR-001: an integer read out of a datatype counts once against the expression node bound.
    /// Each comparison below is four nodes (compare, field access, its base, the literal), so 20,000
    /// joined by 19,999 conjunctions is 99,999 nodes and 20,001 is 100,004.
    #[ix_trace_rs::trace("TC-010", "NFR-001-AC-3")]
    #[test]
    fn integer_datatype_reads_count_once_against_the_node_bound() {
        fn join(items: &mut Vec<Expression>) -> Expression {
            while items.len() > 1 {
                let mut next = Vec::with_capacity(items.len() / 2 + 1);
                let mut drained = std::mem::take(items).into_iter();
                while let Some(left) = drained.next() {
                    next.push(match drained.next() {
                        Some(right) => Expression::new(
                            ExpressionKind::Boolean {
                                operator: BooleanOperator::TotalAnd,
                                left: Box::new(left),
                                right: Box::new(right),
                            },
                            span(),
                        ),
                        None => left,
                    });
                }
                *items = next;
            }
            items.remove(0)
        }
        let integer = IntegerType::new(
            quire_contract_model::IntegerDomain::Signed,
            0,
            100,
            OverflowPolicy::Reject,
        )
        .expect("integer type");
        let build = |count: usize| {
            let mut items: Vec<Expression> = (0..count)
                .map(|_| {
                    Expression::new(
                        ExpressionKind::Compare {
                            operator: ComparisonOperator::LessEqual,
                            left: Box::new(Expression::new(
                                ExpressionKind::FieldAccess {
                                    base: Box::new(reference("r")),
                                    field: SymbolName::new("n").expect("symbol"),
                                },
                                span(),
                            )),
                            right: Box::new(Expression::new(
                                ExpressionKind::IntegerLiteral {
                                    value: 5,
                                    value_type: integer.clone(),
                                },
                                span(),
                            )),
                        },
                        span(),
                    )
                })
                .collect();
            let expression = join(&mut items);
            unchecked_statement(
                vec![record_declaration_with(
                    "R",
                    &[("n", ValueType::integer(integer.clone()))],
                )],
                vec![input(
                    "r",
                    ValueType::Record {
                        name: SymbolName::new("R").expect("symbol"),
                    },
                )],
                expression,
            )
        };
        lower_boolean_statements(&[build(20_000)], &[]).expect("99,999 nodes admitted");
        let errors = lower_boolean_statements(&[build(20_001)], &[]).expect_err("over the bound");
        assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);
    }
}
