use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use quire_contract_ir::{
    BooleanOperator, CanonicalDigest, CanonicalProfile, Clause, ClauseRef, ComparisonOperator,
    ContractPackage, DeclarationEnvironment, ExecutionPoint, Expression, ExpressionKind,
    IntegerDomain, OverflowPolicy, Requirement, RequirementRef, SourceSpan, StateObservation,
    SymbolName, TypeDeclaration, TypedExpression, ValueDeclarationKind, ValueType,
};
use sha2::{Digest as _, Sha256};

/// The accepted engine-neutral analysis model profile.
pub const ANALYSIS_MODEL_PROFILE: &str = "quire.analysis-model/v1";
/// The first exact SMT-LIB2 encoding profile.
pub const SMTLIB2_PROFILE: &str = "quire.smtlib2/v1";
/// Exact accepted contract-IR source revision.
pub const CONTRACT_IR_REVISION: &str = "bb5d30cbb1519b7ac286250114c96ba967661cba";
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LoweringErrorCode {
    InvalidStatement,
    InvalidBinding,
    DuplicateStatement,
    UnsupportedConstruct,
    UnsupportedType,
    ResourceLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringError {
    code: LoweringErrorCode,
    path: String,
    message: String,
}

impl LoweringError {
    fn new(code: LoweringErrorCode, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }

    pub const fn code(&self) -> LoweringErrorCode {
        self.code
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
    clause_digest: CanonicalDigest,
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
                clause_digest: canonical.digest(),
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
    pub const fn clause_digest(&self) -> CanonicalDigest {
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
    logic: "QF_UF",
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
    ],
    unsupported_constructs: &[
        "integer",
        "rational",
        "text",
        "enum",
        "record",
        "option",
        "collection",
        "field_access",
        "option_access",
        "collection_access",
        "pure_function_call",
        "arithmetic",
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
    pub clause_digest: CanonicalDigest,
    pub source: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableMap {
    pub symbol: String,
    pub origins: Vec<String>,
    pub binding_group: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryBundle {
    pub profile: &'static str,
    pub logic: &'static str,
    pub query: String,
    pub analysis_request_digest: AnalysisDigest,
    pub query_digest: AnalysisDigest,
    pub binding_set_digest: AnalysisDigest,
    pub assertions: Vec<AssertionMap>,
    pub variables: Vec<VariableMap>,
}

pub fn lower_boolean_statements(
    statements: &[StatementInput],
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
    let mut lowered = Vec::with_capacity(statements.len());
    let mut errors = Vec::new();

    for statement in statements {
        let statement_digest = statement_digest(statement, binding_set_digest);
        let assertion_name = assertion_symbol(statement, statement_digest);
        match lower_expression(statement, &binding_lookup, &mut encountered, &mut variables) {
            Ok(expression) => {
                lowered.push((assertion_name, statement_digest, statement, expression));
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
            LOWERING_REQUEST_KIND.as_bytes(),
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
    query.push_str("(set-info :source |quire.smtlib2/v1|)\n");
    query.push_str("(set-option :produce-models true)\n");
    query.push_str("(set-logic QF_UF)\n");
    for symbol in variables.keys() {
        query.push_str("(declare-fun ");
        query.push_str(symbol);
        query.push_str(" () Bool)\n");
    }
    let mut assertions = Vec::with_capacity(lowered.len());
    for (name, _, statement, expression) in lowered {
        query.push_str("(assert (! ");
        query.push_str(&expression);
        query.push_str(" :named ");
        query.push_str(&name);
        query.push_str("))\n");
        assertions.push(AssertionMap {
            name,
            clause: statement.clause.clone(),
            clause_digest: statement.clause_digest,
            source: statement.source.clone(),
        });
    }
    query.push_str("(check-sat)\n");
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
        logic: CAPABILITY_CONTRACT.logic,
        query,
        analysis_request_digest,
        query_digest,
        binding_set_digest,
        assertions,
        variables: variables.into_values().collect(),
    })
}

type BindingLookup = BTreeMap<String, String>;

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
) -> Result<String, LoweringError> {
    fn lower(
        expression: &Expression,
        statement: &StatementInput,
        bindings: &BindingLookup,
        encountered: &mut BTreeSet<String>,
        variables: &mut BTreeMap<String, VariableMap>,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<String, LoweringError> {
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
        let recurse = |child: &Expression,
                       encountered: &mut BTreeSet<String>,
                       variables: &mut BTreeMap<String, VariableMap>,
                       nodes: &mut usize| {
            lower(
                child,
                statement,
                bindings,
                encountered,
                variables,
                depth + 1,
                nodes,
            )
        };
        match expression.kind() {
            ExpressionKind::BooleanLiteral { value } => Ok(value.to_string()),
            ExpressionKind::ValueReference { name, observation } => {
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
                if declaration.value_type() != &ValueType::Boolean {
                    return Err(unsupported(
                        "expression.value_reference",
                        "only Boolean value references are exact in quire.smtlib2/v1",
                    ));
                }
                let kind = declaration_kind(declaration.kind());
                let point = execution_point_key(&statement.execution_point);
                let digest = type_shape_digest(&statement.environment, declaration.value_type())?;
                let key = variable_key(
                    statement.environment.owner(),
                    kind,
                    observation_name(*observation),
                    name.as_str(),
                    &point,
                    digest,
                );
                let (symbol, binding_group) = match bindings.get(&key) {
                    Some(group) => {
                        encountered.insert(key.clone());
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
                let entry = variables
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
                Ok(symbol)
            }
            ExpressionKind::BooleanNot { operand } => Ok(format!(
                "(not {})",
                recurse(operand, encountered, variables, nodes)?
            )),
            ExpressionKind::Boolean {
                operator,
                left,
                right,
            } => {
                let op = match operator {
                    BooleanOperator::ShortCircuitAnd | BooleanOperator::TotalAnd => "and",
                    BooleanOperator::ShortCircuitOr | BooleanOperator::TotalOr => "or",
                    BooleanOperator::Implication => "=>",
                };
                Ok(format!(
                    "({op} {} {})",
                    recurse(left, encountered, variables, nodes)?,
                    recurse(right, encountered, variables, nodes)?
                ))
            }
            ExpressionKind::Compare {
                operator,
                left,
                right,
            } if matches!(
                operator,
                ComparisonOperator::Equal | ComparisonOperator::NotEqual
            ) =>
            {
                let left = recurse(left, encountered, variables, nodes)?;
                let right = recurse(right, encountered, variables, nodes)?;
                Ok(match operator {
                    ComparisonOperator::Equal => format!("(= {left} {right})"),
                    ComparisonOperator::NotEqual => format!("(not (= {left} {right}))"),
                    _ => unreachable!(),
                })
            }
            other => Err(unsupported("expression", construct_name(other))),
        }
    }
    let mut nodes = 0;
    lower(
        &statement.expression,
        statement,
        bindings,
        encountered,
        variables,
        1,
        &mut nodes,
    )
}

fn unsupported(path: &str, message: &str) -> LoweringError {
    LoweringError::new(LoweringErrorCode::UnsupportedConstruct, path, message)
}

fn construct_name(kind: &ExpressionKind) -> &'static str {
    match kind {
        ExpressionKind::IntegerLiteral { .. } => "integer literals are unsupported",
        ExpressionKind::RationalLiteral { .. } => "rational literals are unsupported",
        ExpressionKind::TextLiteral { .. } => "text literals are unsupported",
        ExpressionKind::EnumLiteral { .. } => "enum literals are unsupported",
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
            "arithmetic is unsupported"
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
