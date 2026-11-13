//! Translation of the `analysis_claim` nodes of a contract package into analysis
//! statements (FR-009, "Translation of an analysis_claim to an analysis statement").
//!
//! A [`ContractPackageV1`] holds a graph of checked terms and names no analysis
//! request. [`translate_analysis_claims`] builds, in memory, one
//! `ContractPackage<TypedExpression>` with one requirement and one clause per
//! analysable clause, and takes one consistency request from it. Nothing is
//! written to disk.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use quire_contract_model::{
    BooleanOperator, CheckedJson, CheckedLiteralValue, CheckedNodeId, CheckedNodeKind,
    CheckedSemanticNodeV2, CheckedTerm, ClaimForm, Clause, ClauseId, ClauseKind,
    ComparisonOperator, ContractPackage, ContractPackageV1, DeclarationEnvironment, Diagnostic,
    ExecutionPoint, ExecutionPointName, Expression, ExpressionKind, PackageId, Requirement,
    RequirementId, RequirementRef, RequirementRevision, ScalarTypeForm, SchemaVersion,
    SourceDocumentId, SourceIdentity, SourceLocation, SourceRevision, SourceSpan, StateObservation,
    SymbolName, TypedExpression, ValueDeclaration, ValueDeclarationKind, ValueForm, ValueType,
    MAX_EXPRESSION_NODES,
};

use crate::{lower_boolean_statements, smt::MAX_EXPRESSION_DEPTH, AnalysisRequest, StatementInput};

const SOURCE_DOCUMENT: &str = "contract-package";
const REQUIREMENT_ID: &str = "analysis";
const ANCHOR_NAME: &str = "analysis";

const OP_NOT: &str = "quire.op.boolean.not";
const OP_AND: &str = "quire.op.boolean.and";
const OP_OR: &str = "quire.op.boolean.or";
const OP_IMPLIES: &str = "quire.op.boolean.implies";
const OP_EQ: &str = "quire.op.boolean.eq";
const OP_NE: &str = "quire.op.boolean.ne";

/// Why a package cannot be translated. Each kind is a distinct, stable cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationErrorKind {
    /// `ContractProvenanceV1.source.name` is not a valid package id.
    PackageName,
    /// The package holds no node of kind `claim` with form `analysis_claim`.
    NoAnalysableClause,
    /// An `analysis_claim` body is not a Boolean-typed term.
    NonBooleanClaim,
    /// A Boolean-typed term the translation cannot lower, or a reference cycle.
    CannotLower,
    /// The translated tree of a clause exceeds the expression node bound.
    ExpressionNodeBound,
    /// The translated tree of a clause exceeds the expression depth bound.
    ExpressionDepthBound,
    /// A node has no usable source region.
    SourceRegion,
    /// An analysis constructor or validator refused a clause.
    ClauseRejected,
    /// A package, environment, or request constructor refused the whole build.
    Construction,
}

/// A package that cannot be translated, with the node or clause it concerns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationError {
    kind: TranslationErrorKind,
    node: Option<usize>,
    clause: Option<String>,
    path: String,
    message: String,
}

impl TranslationError {
    fn at_node(kind: TranslationErrorKind, node: usize, message: impl Into<String>) -> Self {
        Self {
            kind,
            node: Some(node),
            clause: None,
            path: format!("semantic_graph.nodes[{node}]"),
            message: message.into(),
        }
    }

    fn at_clause(node: usize, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: TranslationErrorKind::ClauseRejected,
            node: Some(node),
            clause: Some(clause_name(node)),
            path: path.into(),
            message: message.into(),
        }
    }

    fn general(
        kind: TranslationErrorKind,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            node: None,
            clause: None,
            path: path.into(),
            message: message.into(),
        }
    }

    /// The stable failure code.
    pub const fn kind(&self) -> TranslationErrorKind {
        self.kind
    }

    /// The zero-based index in `semantic_graph.nodes` the error names, if any.
    pub const fn node(&self) -> Option<usize> {
        self.node
    }

    /// The clause id `claim-N` the error names, if any.
    pub fn clause(&self) -> Option<&str> {
        self.clause.as_deref()
    }

    /// The path of the error: `semantic_graph.nodes[N]` for a node error, the
    /// path of the refusing validator for a clause error.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The human-readable cause.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for TranslationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.clause {
            Some(clause) => write!(formatter, "{clause} at {}: {}", self.path, self.message),
            None if self.path.is_empty() => formatter.write_str(&self.message),
            None => write!(formatter, "{}: {}", self.path, self.message),
        }
    }
}

impl std::error::Error for TranslationError {}

/// The in-memory result of translating a contract package.
#[derive(Clone, Debug)]
pub struct AnalysisTranslation {
    package: ContractPackage<TypedExpression>,
    environment: DeclarationEnvironment,
    request: AnalysisRequest,
}

impl AnalysisTranslation {
    /// The package of one requirement holding one clause per analysable clause.
    pub fn package(&self) -> &ContractPackage<TypedExpression> {
        &self.package
    }

    /// The declarations of the Boolean parameters the clauses reach.
    pub fn environment(&self) -> &DeclarationEnvironment {
        &self.environment
    }

    /// The consistency request over every clause, without assumptions or bindings.
    pub fn request(&self) -> &AnalysisRequest {
        &self.request
    }

    /// The consistency request, consuming the translation.
    pub fn into_request(self) -> AnalysisRequest {
        self.request
    }
}

fn clause_name(node: usize) -> String {
    format!("claim-{node}")
}

fn diagnostic_error(node: usize, diagnostic: &Diagnostic) -> TranslationError {
    TranslationError::at_clause(node, diagnostic.path.clone(), diagnostic.message.clone())
}

fn construction(diagnostic: &Diagnostic) -> TranslationError {
    TranslationError::general(
        TranslationErrorKind::Construction,
        diagnostic.path.clone(),
        diagnostic.message.clone(),
    )
}

/// Translates every `analysis_claim` of an admitted package into one consistency
/// request.
///
/// The clauses are the claim nodes in graph order, with clause id `claim-N` for
/// node `N`, all under requirement `analysis` revision `1` and the synthetic
/// `Handler` anchor `analysis`. Each clause is lowered once on its own so that a
/// lowering refusal names its `claim-N`. `Expression` nodes are counted in pre-order
/// (an application before its operands), and the operation identity is read with
/// `CheckedApplicationOperation::identity`, because `read()` returns `NotAdmitted` for a
/// package read by `ContractPackageWireV1::read`. The function is pure: the same package
/// yields the same clauses, identities, declarations and spans.
pub fn translate_analysis_claims(
    package: &ContractPackageV1,
) -> Result<AnalysisTranslation, TranslationError> {
    let wire = package.wire();
    let nodes = wire.semantic_graph.nodes.as_slice();
    let kinds = nodes
        .iter()
        .map(|node| package.node_kind(&node.node_id))
        .collect::<Vec<_>>();
    let claims = kinds
        .iter()
        .enumerate()
        .filter(|(_, kind)| matches!(kind, Some(CheckedNodeKind::Claim(ClaimForm::AnalysisClaim))))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if claims.is_empty() {
        return Err(TranslationError::general(
            TranslationErrorKind::NoAnalysableClause,
            "semantic_graph.nodes",
            "the package holds no analysis_claim node",
        ));
    }

    let name = &*wire.package.source.name;
    let package_id = PackageId::new(name).map_err(|diagnostic| {
        TranslationError::general(
            TranslationErrorKind::PackageName,
            "package.source.name",
            format!("package name {name:?}: {}", diagnostic.message),
        )
    })?;
    let source = SourceIdentity::new(
        SourceDocumentId::new(SOURCE_DOCUMENT).map_err(|diagnostic| construction(&diagnostic))?,
        SourceRevision::new(1).map_err(|diagnostic| construction(&diagnostic))?,
    );
    let spans = SpanSource {
        package,
        identity: &source,
    };

    let mut walker = Walker {
        nodes,
        terms: nodes
            .iter()
            .map(|node| package.term(&node.node_id))
            .collect(),
        kinds: &kinds,
        index: nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (&node.node_id, position))
            .collect(),
        in_progress: BTreeSet::new(),
        entered: Vec::new(),
        produced: 0,
        parameters: BTreeSet::new(),
        span: zero_span(&source)?,
    };
    let mut bodies = Vec::with_capacity(claims.len());
    for &claim in &claims {
        let span = spans.node(claim)?;
        bodies.push((claim, span.clone(), walker.claim(claim, span)?));
    }

    let mut declarations = Vec::with_capacity(walker.parameters.len());
    for &parameter in &walker.parameters {
        declarations.push(ValueDeclaration::new(
            parameter_symbol(parameter, parameter)?,
            ValueDeclarationKind::Input,
            ValueType::Boolean,
            spans.node(parameter)?,
        ));
    }

    let requirement_id = RequirementId::new(REQUIREMENT_ID).map_err(|d| construction(&d))?;
    let revision = RequirementRevision::new(1).map_err(|d| construction(&d))?;
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let environment = DeclarationEnvironment::new(owner, vec![], declarations, vec![])
        .map_err(|diagnostics| first_construction(&diagnostics))?;
    let anchor = ExecutionPoint::Handler {
        name: ExecutionPointName::new(ANCHOR_NAME).map_err(|d| construction(&d))?,
    };

    let mut clauses = Vec::with_capacity(bodies.len());
    for (claim, span, expression) in bodies {
        let typed = environment
            .check_expression(&expression, &ValueType::Boolean, &anchor, true)
            .map_err(|diagnostics| first_clause(claim, &diagnostics))?;
        let id = ClauseId::new(clause_name(claim)).map_err(|d| diagnostic_error(claim, &d))?;
        clauses.push(
            Clause::new(id, ClauseKind::Assertion, Some(anchor.clone()), span, typed)
                .map_err(|d| diagnostic_error(claim, &d))?,
        );
    }
    let requirement_span = zero_span(&source)?;
    let requirement = Requirement::<TypedExpression>::new(
        &package_id,
        requirement_id,
        revision,
        requirement_span,
        clauses,
    )
    .map_err(|d| construction(&d))?;
    let contract = ContractPackage::new(package_id, SchemaVersion::V1_1, source, vec![requirement])
        .map_err(|diagnostics| first_construction(&diagnostics))?;

    let requirement = &contract.requirements()[0];
    let mut statements = Vec::with_capacity(claims.len());
    for (&claim, clause) in claims.iter().zip(requirement.clauses()) {
        let statement =
            StatementInput::from_clause(&contract, requirement, clause, environment.clone())
                .map_err(|errors| first_lowering(claim, &errors))?;
        lower_boolean_statements(std::slice::from_ref(&statement), &[])
            .map_err(|errors| first_lowering(claim, &errors))?;
        statements.push(statement);
    }
    let request =
        AnalysisRequest::consistency(Vec::new(), statements, Vec::new()).map_err(|errors| {
            let first = errors.first();
            TranslationError::general(
                TranslationErrorKind::Construction,
                first.map_or("", |error| error.path()),
                first.map_or("the request is refused", |error| error.message()),
            )
        })?;
    Ok(AnalysisTranslation {
        package: contract,
        environment,
        request,
    })
}

fn first_construction(diagnostics: &[Diagnostic]) -> TranslationError {
    diagnostics.first().map_or_else(
        || {
            TranslationError::general(
                TranslationErrorKind::Construction,
                "",
                "the package is refused",
            )
        },
        construction,
    )
}

fn first_clause(node: usize, diagnostics: &[Diagnostic]) -> TranslationError {
    diagnostics.first().map_or_else(
        || TranslationError::at_clause(node, "", "the clause is refused"),
        |diagnostic| diagnostic_error(node, diagnostic),
    )
}

fn first_lowering(node: usize, errors: &[crate::LoweringError]) -> TranslationError {
    errors.first().map_or_else(
        || TranslationError::at_clause(node, "", "the clause is refused"),
        |error| TranslationError::at_clause(node, error.path(), error.message()),
    )
}

fn zero_span(source: &SourceIdentity) -> Result<SourceSpan, TranslationError> {
    let location = SourceLocation::new(source.clone(), 1, 1, 0).map_err(|d| construction(&d))?;
    SourceSpan::new(location.clone(), location).map_err(|d| construction(&d))
}

fn parameter_symbol(parameter: usize, node: usize) -> Result<SymbolName, TranslationError> {
    SymbolName::new(format!("n{parameter}")).map_err(|diagnostic| {
        TranslationError::at_node(TranslationErrorKind::CannotLower, node, diagnostic.message)
    })
}

/// Spans of graph nodes, built from the first region of their first occurrence.
struct SpanSource<'a> {
    package: &'a ContractPackageV1,
    identity: &'a SourceIdentity,
}

impl SpanSource<'_> {
    fn node(&self, node: usize) -> Result<SourceSpan, TranslationError> {
        let region = self
            .package
            .wire()
            .source_map
            .nodes
            .get(node)
            .and_then(|entry| entry.occurrences.first())
            .and_then(|occurrence| occurrence.regions.first())
            .ok_or_else(|| {
                TranslationError::at_node(
                    TranslationErrorKind::SourceRegion,
                    node,
                    "the node has no source region",
                )
            })?;
        let location = |offset| {
            SourceLocation::new(self.identity.clone(), 1, 1, offset).map_err(|diagnostic| {
                TranslationError::at_node(
                    TranslationErrorKind::SourceRegion,
                    node,
                    diagnostic.message,
                )
            })
        };
        SourceSpan::new(location(region.start)?, location(region.end)?).map_err(|diagnostic| {
            TranslationError::at_node(TranslationErrorKind::SourceRegion, node, diagnostic.message)
        })
    }
}

/// The recursive translation of one claim at a time.
struct Walker<'a> {
    nodes: &'a [CheckedSemanticNodeV2<CheckedJson>],
    terms: Vec<Option<&'a CheckedTerm>>,
    kinds: &'a [Option<CheckedNodeKind>],
    index: BTreeMap<&'a CheckedNodeId, usize>,
    /// Nodes whose body is being translated, including the claim itself.
    in_progress: BTreeSet<usize>,
    /// Nodes entered through references by the translation in progress, in order.
    entered: Vec<usize>,
    /// `Expression` nodes produced for the current claim, each inlined copy counted.
    produced: u32,
    /// Distinct `value/parameter` nodes reached by any claim.
    parameters: BTreeSet<usize>,
    /// The span every `Expression` of the current claim carries.
    span: SourceSpan,
}

impl<'a> Walker<'a> {
    fn claim(&mut self, claim: usize, span: SourceSpan) -> Result<Expression, TranslationError> {
        self.span = span;
        self.produced = 0;
        self.in_progress.clear();
        self.entered.clear();
        let Some(body) = self.terms[claim] else {
            return Err(cannot_lower(claim, "the node has no typed body"));
        };
        if !self.is_boolean_term(body) {
            return Err(TranslationError::at_node(
                TranslationErrorKind::NonBooleanClaim,
                claim,
                "the analysis_claim body is not a Boolean-typed term",
            ));
        }
        self.in_progress.insert(claim);
        self.expression(body, claim, 1)
    }

    fn lookup(&self, id: &CheckedNodeId) -> Option<usize> {
        self.index.get(id).copied()
    }

    fn kind(&self, node: usize) -> Option<CheckedNodeKind> {
        self.kinds.get(node).copied().flatten()
    }

    fn is_boolean_type(&self, id: &CheckedNodeId) -> bool {
        self.lookup(id).and_then(|node| self.kind(node))
            == Some(CheckedNodeKind::ScalarType(ScalarTypeForm::Boolean))
    }

    /// The type of a term per FR-009: a literal's `type`, the `semantic_type` of
    /// a reference target, an application's `result_type`.
    fn is_boolean_term(&self, term: &CheckedTerm) -> bool {
        match term {
            CheckedTerm::Literal { type_id, .. } => self.is_boolean_type(type_id),
            CheckedTerm::Reference { target } => self
                .lookup(target)
                .is_some_and(|node| self.is_boolean_type(&self.nodes[node].semantic_type)),
            CheckedTerm::Application { result_type, .. } => self.is_boolean_type(result_type),
            _ => false,
        }
    }

    /// Translates `term`, a term of the body of `node`, at tree depth `depth`.
    fn expression(
        &mut self,
        term: &'a CheckedTerm,
        node: usize,
        depth: usize,
    ) -> Result<Expression, TranslationError> {
        let mark = self.entered.len();
        let result = self.follow(term, node, depth);
        for entered in self.entered.drain(mark..) {
            self.in_progress.remove(&entered);
        }
        result
    }

    /// Follows reference hops to an expression or literal node in a loop, so that a
    /// long chain of references does not deepen the stack; only operands recurse.
    fn follow(
        &mut self,
        mut term: &'a CheckedTerm,
        mut node: usize,
        depth: usize,
    ) -> Result<Expression, TranslationError> {
        loop {
            let CheckedTerm::Reference { target } = term else {
                return self.produce(term, node, depth);
            };
            let Some(target_node) = self.lookup(target) else {
                return Err(cannot_lower(node, "the reference target is not a node"));
            };
            match self.kind(target_node) {
                Some(
                    CheckedNodeKind::Expression(_) | CheckedNodeKind::Value(ValueForm::Literal),
                ) => {
                    if !self.in_progress.insert(target_node) {
                        return Err(cannot_lower(target_node, "reference cycle"));
                    }
                    self.entered.push(target_node);
                    term = self.terms[target_node]
                        .ok_or_else(|| cannot_lower(target_node, "the node has no typed body"))?;
                    node = target_node;
                }
                Some(CheckedNodeKind::Value(ValueForm::Parameter)) => {
                    if !self.is_boolean_type(&self.nodes[target_node].semantic_type) {
                        return Err(cannot_lower(target_node, "the parameter is not Boolean"));
                    }
                    self.count(node, depth)?;
                    self.parameters.insert(target_node);
                    return Ok(self.make(ExpressionKind::ValueReference {
                        name: parameter_symbol(target_node, node)?,
                        observation: StateObservation::Current,
                    }));
                }
                _ => {
                    return Err(cannot_lower(
                        target_node,
                        "the node is neither an expression, a literal nor a Boolean parameter",
                    ))
                }
            }
        }
    }

    fn produce(
        &mut self,
        term: &'a CheckedTerm,
        node: usize,
        depth: usize,
    ) -> Result<Expression, TranslationError> {
        match term {
            CheckedTerm::Literal { type_id, .. } => {
                let value = match term.literal_read().map(|read| read.value) {
                    Some(CheckedLiteralValue::Boolean(value)) if self.is_boolean_type(type_id) => {
                        value
                    }
                    _ => return Err(cannot_lower(node, "the literal is not a Boolean literal")),
                };
                self.count(node, depth)?;
                Ok(self.make(ExpressionKind::BooleanLiteral { value }))
            }
            CheckedTerm::Application {
                operation,
                arguments,
                ..
            } => {
                let Some(identity) = operation.identity() else {
                    return Err(cannot_lower(node, "the operation has no catalog identity"));
                };
                let Some(operation) = Operation::from_identity(identity) else {
                    return Err(cannot_lower(
                        node,
                        format!("operation {identity} is outside the Boolean subset"),
                    ));
                };
                if arguments.len() != operation.arity() {
                    return Err(cannot_lower(
                        node,
                        format!("operation {identity} takes {} arguments", operation.arity()),
                    ));
                }
                self.count(node, depth)?;
                let mut operands = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    operands.push(self.expression(argument, node, depth + 1)?);
                }
                let kind = operation
                    .build(operands)
                    .ok_or_else(|| cannot_lower(node, "the operand count is wrong"))?;
                Ok(self.make(kind))
            }
            _ => Err(cannot_lower(node, "the term form cannot be lowered")),
        }
    }

    /// Counts one produced `Expression` at `depth`, refusing the first over a bound.
    fn count(&mut self, node: usize, depth: usize) -> Result<(), TranslationError> {
        self.produced = self.produced.saturating_add(1);
        if self.produced > MAX_EXPRESSION_NODES {
            return Err(TranslationError::at_node(
                TranslationErrorKind::ExpressionNodeBound,
                node,
                format!("the translation exceeds {MAX_EXPRESSION_NODES} expression nodes"),
            ));
        }
        if depth > MAX_EXPRESSION_DEPTH {
            return Err(TranslationError::at_node(
                TranslationErrorKind::ExpressionDepthBound,
                node,
                format!("the translation exceeds depth {MAX_EXPRESSION_DEPTH}"),
            ));
        }
        Ok(())
    }

    fn make(&self, kind: ExpressionKind) -> Expression {
        Expression::new(kind, self.span.clone())
    }
}

fn cannot_lower(node: usize, message: impl Into<String>) -> TranslationError {
    TranslationError::at_node(TranslationErrorKind::CannotLower, node, message)
}

/// A supported operation of the Boolean subset, by catalog identity.
#[derive(Clone, Copy)]
enum Operation {
    Not,
    Binary(Binary),
}

#[derive(Clone, Copy)]
enum Binary {
    Boolean(BooleanOperator),
    Compare(ComparisonOperator),
}

impl Operation {
    fn from_identity(identity: &str) -> Option<Self> {
        Some(match identity {
            OP_NOT => Self::Not,
            OP_AND => Self::Binary(Binary::Boolean(BooleanOperator::TotalAnd)),
            OP_OR => Self::Binary(Binary::Boolean(BooleanOperator::TotalOr)),
            OP_IMPLIES => Self::Binary(Binary::Boolean(BooleanOperator::Implication)),
            OP_EQ => Self::Binary(Binary::Compare(ComparisonOperator::Equal)),
            OP_NE => Self::Binary(Binary::Compare(ComparisonOperator::NotEqual)),
            _ => return None,
        })
    }

    const fn arity(self) -> usize {
        match self {
            Self::Not => 1,
            Self::Binary(_) => 2,
        }
    }

    /// The expression over the translated operands, `None` for a wrong operand count.
    fn build(self, mut operands: Vec<Expression>) -> Option<ExpressionKind> {
        match self {
            Self::Not => Some(ExpressionKind::BooleanNot {
                operand: Box::new(operands.pop().filter(|_| operands.is_empty())?),
            }),
            Self::Binary(binary) => {
                let right = Box::new(operands.pop()?);
                let left = Box::new(operands.pop().filter(|_| operands.is_empty())?);
                Some(match binary {
                    Binary::Boolean(operator) => ExpressionKind::Boolean {
                        operator,
                        left,
                        right,
                    },
                    Binary::Compare(operator) => ExpressionKind::Compare {
                        operator,
                        left,
                        right,
                    },
                })
            }
        }
    }
}
