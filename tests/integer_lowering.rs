//! Requirement-tagged tests for linear integer arithmetic lowering (FR-010, TC-016).

#![cfg(target_os = "linux")]

use ix_trace_rs::trace;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use quire_analyze::{
    classify_analysis, compare_solver_records, execute_solver, lower_analysis_request,
    lower_boolean_statements, render_differential_report, validate_differential_report,
    validate_report_document, AdapterLimits, AnalysisRequest, AnalysisStatus, CancellationToken,
    DifferentialDisposition, ExplanationState, LoweringErrorCode, QueryBundle, ReplayFailureReason,
    SolverConfig, SolverEngine, StatementInput, UnsupportedClass, CAPABILITY_CONTRACT,
    SMTLIB2_PROFILE,
};
use quire_contract_model::{
    AnchorName, BooleanOperator, Clause, ClauseId, ClauseKind, ComparisonOperator, ContractPackage,
    DeclarationEnvironment, ExecutionPoint, Expression, ExpressionKind, IntegerDomain, IntegerType,
    NumericOperator, OverflowPolicy, PackageId, Requirement, RequirementId, RequirementRef,
    RequirementRevision, SchemaVersion, SourceDocumentId, SourceIdentity, SourceLocation,
    SourceRevision, SourceSpan, StateObservation, SymbolName, TypedExpression, ValueDeclaration,
    ValueDeclarationKind, ValueType,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn span() -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("integer-test").expect("source id"),
        SourceRevision::new(1).expect("source revision"),
    );
    SourceSpan::new(
        SourceLocation::new(source.clone(), 1, 1, 0).expect("start"),
        SourceLocation::new(source, 1, 2, 1).expect("end"),
    )
    .expect("span")
}

fn point() -> ExecutionPoint {
    ExecutionPoint::Pre {
        operation: AnchorName::new("check").expect("anchor").into(),
    }
}

fn integer_type(minimum: i128, maximum: i128, overflow: OverflowPolicy) -> IntegerType {
    IntegerType::new(IntegerDomain::Signed, minimum, maximum, overflow).expect("integer type")
}

fn narrow() -> IntegerType {
    integer_type(-1000, 1000, OverflowPolicy::Reject)
}

fn node(kind: ExpressionKind) -> Expression {
    Expression::new(kind, span())
}

fn lit(value: i128, value_type: &IntegerType) -> Expression {
    node(ExpressionKind::IntegerLiteral {
        value,
        value_type: value_type.clone(),
    })
}

fn var(name: &str) -> Expression {
    node(ExpressionKind::ValueReference {
        name: SymbolName::new(name).expect("symbol"),
        observation: StateObservation::Current,
    })
}

fn numeric(operator: NumericOperator, left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Numeric {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn add(left: Expression, right: Expression) -> Expression {
    numeric(NumericOperator::Add, left, right)
}

fn mul(left: Expression, right: Expression) -> Expression {
    numeric(NumericOperator::Multiply, left, right)
}

fn neg(operand: Expression) -> Expression {
    node(ExpressionKind::NumericNegate {
        operand: Box::new(operand),
    })
}

fn compare(operator: ComparisonOperator, left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Compare {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn and(left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Boolean {
        operator: BooleanOperator::ShortCircuitAnd,
        left: Box::new(left),
        right: Box::new(right),
    })
}

/// Guards `-limit <= name <= limit` so checked arithmetic over `name` stays inside its bounds,
/// then continues with `body`.
fn guarded(names: &[&str], limit: i128, body: Expression) -> Expression {
    let ty = narrow();
    names.iter().rev().fold(body, |rest, name| {
        and(
            and(
                compare(
                    ComparisonOperator::GreaterEqual,
                    var(name),
                    lit(-limit, &ty),
                ),
                compare(ComparisonOperator::LessEqual, var(name), lit(limit, &ty)),
            ),
            rest,
        )
    })
}

fn statement(
    label: &str,
    variables: &[(&str, IntegerType)],
    expression: Expression,
) -> StatementInput {
    try_statement(label, variables, expression).expect("statement")
}

fn try_statement(
    label: &str,
    variables: &[(&str, IntegerType)],
    expression: Expression,
) -> Result<StatementInput, String> {
    try_mixed_statement(label, variables, &[], expression)
}

/// Like `try_statement`, with additional Boolean inputs.
fn try_mixed_statement(
    label: &str,
    variables: &[(&str, IntegerType)],
    booleans: &[&str],
    expression: Expression,
) -> Result<StatementInput, String> {
    let package_id = PackageId::new("agent-ix/integer-test").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-{label}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let declarations = variables
        .iter()
        .map(|(name, value)| {
            ValueDeclaration::new(
                SymbolName::new(*name).expect("symbol"),
                ValueDeclarationKind::Input,
                ValueType::Integer {
                    value: value.clone(),
                },
                span(),
            )
        })
        .chain(booleans.iter().map(|name| {
            ValueDeclaration::new(
                SymbolName::new(*name).expect("symbol"),
                ValueDeclarationKind::Input,
                ValueType::Boolean,
                span(),
            )
        }))
        .collect();
    let environment =
        DeclarationEnvironment::new(owner, vec![], declarations, vec![]).expect("environment");
    let checked = environment
        .check_expression(&expression, &ValueType::Boolean, &point(), true)
        .map_err(|diagnostics| format!("{diagnostics:?}"))?;
    let clause = Clause::new(
        ClauseId::new(format!("C-{label}")).expect("clause"),
        ClauseKind::Assertion,
        Some(point()),
        span(),
        checked,
    )
    .expect("clause");
    let requirement = Requirement::<TypedExpression>::new(
        &package_id,
        requirement_id,
        revision,
        span(),
        vec![clause],
    )
    .expect("requirement");
    let package = ContractPackage::new(
        package_id,
        SchemaVersion::V1_1,
        span().source().clone(),
        vec![requirement],
    )
    .expect("package");
    StatementInput::from_clause(
        &package,
        &package.requirements()[0],
        &package.requirements()[0].clauses()[0],
        environment,
    )
    .map_err(|errors| format!("{errors:?}"))
}

fn lower_one(
    label: &str,
    variables: &[(&str, IntegerType)],
    expression: Expression,
) -> QueryBundle {
    let statement = statement(label, variables, expression);
    lower_boolean_statements(&[statement], &[]).expect("lower")
}

/// The single declared symbol of a one-variable query.
fn only_symbol(query: &QueryBundle) -> &str {
    assert_eq!(query.variables().len(), 1);
    &query.variables()[0].symbol
}

/// The text of the single named assertion body.
fn assertion_body(query: &QueryBundle) -> String {
    let line = query
        .query()
        .lines()
        .find(|line| line.contains(":named"))
        .expect("named assertion");
    let body = line
        .strip_prefix("(assert (! ")
        .expect("assert prefix")
        .split(" :named ")
        .next()
        .expect("body");
    body.to_owned()
}

fn sat_fixture() -> Vec<StatementInput> {
    let ty = narrow();
    // x >= 1 and x + y = 10 and y <= 4, guarded so checked addition stays in bounds.
    let body = and(
        compare(ComparisonOperator::GreaterEqual, var("x"), lit(1, &ty)),
        and(
            compare(
                ComparisonOperator::Equal,
                add(var("x"), var("y")),
                lit(10, &ty),
            ),
            compare(ComparisonOperator::LessEqual, var("y"), lit(4, &ty)),
        ),
    );
    vec![statement(
        "sat",
        &[("x", ty.clone()), ("y", ty)],
        guarded(&["x", "y"], 100, body),
    )]
}

fn unsat_fixture() -> Vec<StatementInput> {
    let ty = narrow();
    let body = and(
        compare(ComparisonOperator::Greater, var("x"), lit(5, &ty)),
        compare(ComparisonOperator::Less, var("x"), lit(3, &ty)),
    );
    vec![statement("unsat", &[("x", ty)], body)]
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quire-integer-{label}-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn response_record(
    query: &QueryBundle,
    response: &str,
    engine: SolverEngine,
) -> quire_analyze::SolverRecord {
    response_record_with(query, response, engine, AdapterLimits::default())
}

fn response_record_with(
    query: &QueryBundle,
    response: &str,
    engine: SolverEngine,
    limits: AdapterLimits,
) -> quire_analyze::SolverRecord {
    let directory = TempDirectory::new("response");
    let path = directory.path().join("solver");
    let body = format!(
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ] || [ \"$1\" = \"--version\" ]; then printf 'fake 1.0\\n'; exit 0; fi\n/bin/cat >/dev/null; printf '%s' '{response}'\n"
    );
    fs::write(&path, body).expect("write solver");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions).expect("executable");
    let config = SolverConfig::new(engine, &path, limits).expect("config");
    execute_solver(query, &config, &CancellationToken::default())
}

/// FR-010-AC-1: the contract declares the integer profile and lists each construct once.
#[trace("TC-016", "FR-010-AC-1")]
#[test]
fn capability_contract_declares_the_integer_profile() {
    assert_eq!(CAPABILITY_CONTRACT.profile, "quire.smtlib2/v2");
    assert_eq!(CAPABILITY_CONTRACT.smtlib_version, "2.6");
    assert_eq!(CAPABILITY_CONTRACT.logic, "QF_LIA");
    for exact in [
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
    ] {
        assert!(CAPABILITY_CONTRACT.exact_constructs.contains(&exact));
    }
    for unsupported in CAPABILITY_CONTRACT.unsupported_constructs {
        assert!(!CAPABILITY_CONTRACT.exact_constructs.contains(unsupported));
    }
    for class in [
        UnsupportedClass::NonLinearArithmetic,
        UnsupportedClass::IntegerDivision,
        UnsupportedClass::IntegerRemainder,
        UnsupportedClass::SaturatingArithmetic,
        UnsupportedClass::ConstantOverflow,
    ] {
        assert!(CAPABILITY_CONTRACT
            .unsupported_constructs
            .contains(&class.as_str()));
    }
}

/// FR-010-AC-2/6 and NFR-004-AC-4: the satisfiable fixture lowers to a golden, byte-identical
/// query; range assertions are unnamed and stay out of the assertion map.
#[trace("TC-016", "FR-010-AC-2", "FR-010-AC-6", "NFR-004-AC-4")]
#[test]
fn satisfiable_fixture_lowers_to_the_golden_query() {
    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    assert_eq!(query.profile(), SMTLIB2_PROFILE);
    assert_eq!(query.logic(), "QF_LIA");
    assert_eq!(query.query(), include_str!("golden/integer-sat-v2.smt2"));
    assert_eq!(query.assertions().len(), 1);
    let again = lower_analysis_request(&request).expect("lower again");
    assert_eq!(query.query(), again.query());
    assert_eq!(query.query_digest(), again.query_digest());
    assert_eq!(query.query().matches("() Int)").count(), 2);
    let ranges = query
        .query()
        .lines()
        .filter(|line| line.starts_with("(assert (and (<="))
        .count();
    assert_eq!(ranges, 2);
    assert!(query
        .query()
        .lines()
        .filter(|line| line.starts_with("(assert (and (<="))
        .all(|line| !line.contains(":named")));
}

/// FR-010-AC-6: statement and declaration order do not change the query or its maps.
#[trace("TC-016", "FR-010-AC-6", "NFR-004-AC-4")]
#[test]
fn lowering_is_order_independent() {
    let ty = narrow();
    let first = statement(
        "first",
        &[("a", ty.clone()), ("b", ty.clone())],
        compare(ComparisonOperator::Less, var("a"), var("b")),
    );
    let second = statement(
        "second",
        &[("b", ty.clone()), ("a", ty.clone())],
        compare(ComparisonOperator::NotEqual, var("b"), lit(7, &ty)),
    );
    let forward = lower_boolean_statements(&[first.clone(), second.clone()], &[]).expect("fwd");
    let reverse = lower_boolean_statements(&[second, first], &[]).expect("rev");
    assert_eq!(forward.query(), reverse.query());
    assert_eq!(forward.assertions(), reverse.assertions());
    assert_eq!(forward.variables(), reverse.variables());
    assert_eq!(forward.query_digest(), reverse.query_digest());
}

/// FR-010-AC-2: range assertions form one block after the declarations, in ascending symbol order.
#[trace("TC-016", "FR-010-AC-2", "FR-010-AC-10")]
#[test]
fn range_block_follows_declarations_in_ascending_symbol_order() {
    let full = integer_type(i128::MIN, i128::MAX, OverflowPolicy::Reject);
    let query = lower_one(
        "ranges",
        &[("a", full.clone()), ("b", full.clone())],
        and(
            compare(ComparisonOperator::Less, var("a"), var("b")),
            compare(ComparisonOperator::Equal, var("b"), lit(i128::MIN, &full)),
        ),
    );
    let lines: Vec<&str> = query.query().lines().collect();
    let declarations: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("(declare-fun"))
        .map(|(index, _)| index)
        .collect();
    let ranges: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("(assert (and (<="))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(declarations.len(), 2);
    assert_eq!(ranges.len(), 2);
    assert!(declarations.iter().all(|d| ranges.iter().all(|r| d < r)));
    let named = lines
        .iter()
        .position(|line| line.contains(":named"))
        .expect("named");
    assert!(ranges.iter().all(|r| *r < named));
    let mut symbols: Vec<&str> = query
        .variables()
        .iter()
        .map(|variable| variable.symbol.as_str())
        .collect();
    symbols.sort_unstable();
    for (symbol, line) in symbols.iter().zip(&ranges) {
        assert!(lines[*line].contains(&format!(
            "(<= {symbol} 170141183460469231731687303715884105727)"
        )));
        assert!(lines[*line].contains("(<= (- 170141183460469231731687303715884105728) "));
    }
    // The literal minimum is emitted as the negation of its exact magnitude, never wrapped.
    assert!(
        query.query().contains("(= v_s_")
            && query
                .query()
                .contains("(- 170141183460469231731687303715884105728))")
    );
}

/// FR-010-AC-3: each comparison, negation, addition, subtraction and constant multiplication
/// lowers to the stated form, and a negative literal is a negation of its magnitude.
#[trace("TC-016", "FR-010-AC-3")]
#[test]
fn operators_lower_to_their_stated_forms() {
    let ty = narrow();
    let cases = [
        (ComparisonOperator::Equal, "="),
        (ComparisonOperator::NotEqual, "distinct"),
        (ComparisonOperator::Less, "<"),
        (ComparisonOperator::LessEqual, "<="),
        (ComparisonOperator::Greater, ">"),
        (ComparisonOperator::GreaterEqual, ">="),
    ];
    for (operator, name) in cases {
        let query = lower_one(
            "compare",
            &[("x", ty.clone())],
            compare(operator, var("x"), lit(-6, &ty)),
        );
        let symbol = only_symbol(&query);
        assert_eq!(assertion_body(&query), format!("({name} {symbol} (- 6))"));
    }

    let term = |expression: Expression| {
        let query = lower_one(
            "term",
            &[("x", ty.clone())],
            guarded(
                &["x"],
                10,
                compare(ComparisonOperator::Equal, expression, lit(0, &ty)),
            ),
        );
        let symbol = only_symbol(&query).to_owned();
        (query, symbol)
    };
    let (query, x) = term(neg(var("x")));
    assert!(query.query().contains(&format!("(= (- {x}) 0)")));
    let (query, x) = term(add(var("x"), lit(2, &ty)));
    assert!(query.query().contains(&format!("(= (+ {x} 2) 0)")));
    let (query, x) = term(numeric(NumericOperator::Subtract, var("x"), lit(2, &ty)));
    assert!(query.query().contains(&format!("(= (- {x} 2) 0)")));
    let (query, x) = term(mul(var("x"), lit(2, &ty)));
    assert!(query.query().contains(&format!("(= (* 2 {x}) 0)")));
    let (query, x) = term(mul(lit(-3, &ty), var("x")));
    assert!(query.query().contains(&format!("(= (* (- 3) {x}) 0)")));
}

/// FR-010-AC-5: constant subtrees fold with the constant first and nothing else reassociates.
#[trace("TC-016", "FR-010-AC-5")]
#[test]
fn constant_subtrees_fold_and_other_terms_keep_their_shape() {
    let ty = narrow();
    let folded = |expression: Expression| {
        let query = lower_one(
            "fold",
            &[("x", ty.clone())],
            guarded(
                &["x"],
                10,
                compare(ComparisonOperator::Equal, expression, lit(0, &ty)),
            ),
        );
        let symbol = only_symbol(&query).to_owned();
        (query, symbol)
    };
    let (query, x) = folded(mul(mul(lit(2, &ty), lit(3, &ty)), var("x")));
    assert!(query.query().contains(&format!("(= (* 6 {x}) 0)")));
    let (query, x) = folded(mul(var("x"), lit(6, &ty)));
    assert!(query.query().contains(&format!("(= (* 6 {x}) 0)")));
    let (query, x) = folded(mul(add(lit(1, &ty), lit(2, &ty)), var("x")));
    assert!(query.query().contains(&format!("(= (* 3 {x}) 0)")));
    let (query, x) = folded(add(lit(1, &ty), add(var("x"), lit(2, &ty))));
    assert!(query.query().contains(&format!("(= (+ 1 (+ {x} 2)) 0)")));
}

/// FR-010-AC-4 and NFR-004-AC-1: terms outside linear integer arithmetic are refused with a
/// stable diagnostic naming the class, and no query is produced.
#[trace("TC-016", "FR-010-AC-4", "NFR-004-AC-1")]
#[test]
fn non_linear_and_unsupported_terms_are_refused() {
    let ty = narrow();
    let saturating = integer_type(-1000, 1000, OverflowPolicy::Saturate);
    let refusal = |class: UnsupportedClass,
                   path: &str,
                   variables: &[(&str, IntegerType)],
                   expression: Expression| {
        let statement = statement("refused", variables, expression);
        let errors = lower_boolean_statements(&[statement], &[]).expect_err("unsupported");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedConstruct);
        assert_eq!(errors[0].unsupported_class(), Some(class));
        assert_eq!(errors[0].path(), path);
        // Every renderer that prints the message or the error names the class.
        assert!(errors[0].message().starts_with(class.as_str()));
        assert!(errors[0].to_string().contains(class.as_str()));
    };
    // Facts that a divisor is non-zero and that operands are small come from guards.
    let small = |body: Expression| guarded(&["x", "y"], 10, body);
    refusal(
        UnsupportedClass::NonLinearArithmetic,
        "expression.right.right.left",
        &[("x", ty.clone()), ("y", ty.clone())],
        small(compare(
            ComparisonOperator::Equal,
            mul(var("x"), var("y")),
            lit(0, &ty),
        )),
    );
    let nonzero = and(
        compare(ComparisonOperator::Greater, var("y"), lit(0, &ty)),
        compare(
            ComparisonOperator::Equal,
            numeric(NumericOperator::Divide, var("x"), var("y")),
            lit(0, &ty),
        ),
    );
    refusal(
        UnsupportedClass::IntegerDivision,
        "expression.right.right.right.left",
        &[("x", ty.clone()), ("y", ty.clone())],
        small(nonzero),
    );
    let remainder = and(
        compare(ComparisonOperator::Greater, var("y"), lit(0, &ty)),
        compare(
            ComparisonOperator::Equal,
            numeric(NumericOperator::Remainder, var("x"), var("y")),
            lit(0, &ty),
        ),
    );
    refusal(
        UnsupportedClass::IntegerRemainder,
        "expression.right.right.right.left",
        &[("x", ty.clone()), ("y", ty.clone())],
        small(remainder),
    );
    refusal(
        UnsupportedClass::SaturatingArithmetic,
        "expression.left",
        &[("x", saturating.clone())],
        compare(
            ComparisonOperator::Equal,
            add(var("x"), lit(1, &saturating)),
            lit(0, &saturating),
        ),
    );
}

/// FR-010-AC-10 and NFR-004-AC-2: literals at the 128-bit extremes lower exactly.
#[trace("TC-016", "FR-010-AC-10", "NFR-004-AC-2")]
#[test]
fn extreme_literals_lower_without_wrap_or_clamp() {
    let full = integer_type(i128::MIN, i128::MAX, OverflowPolicy::Reject);
    let query = lower_one(
        "extreme",
        &[("x", full.clone())],
        and(
            compare(
                ComparisonOperator::GreaterEqual,
                var("x"),
                lit(i128::MIN, &full),
            ),
            compare(
                ComparisonOperator::LessEqual,
                var("x"),
                lit(i128::MAX, &full),
            ),
        ),
    );
    let symbol = only_symbol(&query);
    assert!(query.query().contains(&format!(
        "(assert (and (<= (- 170141183460469231731687303715884105728) {symbol}) (<= {symbol} 170141183460469231731687303715884105727)))"
    )));
    assert!(query.query().contains("(>= v_s_"));
    assert!(query
        .query()
        .contains(" (- 170141183460469231731687303715884105728))"));
    assert!(query
        .query()
        .contains(" 170141183460469231731687303715884105727))"));
}

/// FR-010-AC-8: with controlled responses, sat against unsat is a disagreement.
#[trace("TC-016", "FR-010-AC-8")]
#[test]
fn sat_against_unsat_is_a_disagreement() {
    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let z3 = response_record(&query, "sat\n", SolverEngine::Z3);
    let cvc5 = response_record(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert_eq!(run.disposition(), DifferentialDisposition::Disagreement);
    assert!(!run.is_conclusive());
}

/// FR-010-AC-10 (controlled responses): an unsat integer run agrees and its report names the
/// integer profile and logic; a satisfiable integer model that fails replay stays unverified.
#[trace("TC-016", "FR-010-AC-10")]
#[test]
fn unsat_fixture_agrees_and_integer_models_stay_unverified() {
    let request = AnalysisRequest::consistency(vec![], unsat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let z3 = response_record(&query, "unsat\n", SolverEngine::Z3);
    let cvc5 = response_record(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    let report = render_differential_report(&query, &run).expect("report");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
    assert_eq!(value["encodingProfile"], "quire.smtlib2/v2");
    assert_eq!(value["logic"], "QF_LIA");
    validate_differential_report(&report, &query, &run).expect("valid");

    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let symbols = query
        .variables()
        .iter()
        .map(|variable| format!("(define-fun {} () Int 1)", variable.symbol))
        .collect::<Vec<_>>()
        .join(" ");
    let sat = response_record(
        &query,
        &format!("sat\n(model {symbols})\n"),
        SolverEngine::Z3,
    );
    let conclusion = quire_analyze::classify_analysis(&query, &sat);
    assert!(conclusion.verified_model().is_none());
}

/// FR-010-AC-10 and FR-005-AC-6: a report naming v1 or QF_UF is refused.
#[trace("TC-007", "TC-016", "FR-010-AC-10", "FR-005-AC-6")]
#[test]
fn reports_naming_the_boolean_only_profile_are_refused() {
    let request = AnalysisRequest::consistency(vec![], unsat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let z3 = response_record(&query, "unsat\n", SolverEngine::Z3);
    let cvc5 = response_record(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    let report = render_differential_report(&query, &run).expect("report");
    validate_report_document(&report).expect("v2 accepted");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
    assert!(value["assertions"]
        .as_array()
        .expect("assertions")
        .iter()
        .all(|assertion| assertion["clause"].is_string()));
    for (field, stale) in [("encodingProfile", "quire.smtlib2/v1"), ("logic", "QF_UF")] {
        let mut mutated = value.clone();
        mutated[field] = serde_json::Value::String(stale.to_owned());
        let bytes = serde_json::to_vec(&mutated).expect("bytes");
        assert!(validate_report_document(&bytes).is_err(), "{field}");
        assert!(validate_differential_report(&bytes, &query, &run).is_err());
    }
}

/// FR-010-AC-7 and FR-011-AC-1/11: real solvers agree on the unsat fixture, both answer sat on
/// the sat fixture, and each returned model replay-verifies.
#[trace("TC-016", "TC-017", "FR-010-AC-7", "FR-011-AC-1", "FR-011-AC-11")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn real_z3_cvc5_decide_integer_fixtures_identically() {
    let z3_path = PathBuf::from(std::env::var_os("QUIRE_Z3").expect("QUIRE_Z3 is required"));
    let cvc5_path = PathBuf::from(std::env::var_os("QUIRE_CVC5").expect("QUIRE_CVC5 is required"));
    let z3 = SolverConfig::new(SolverEngine::Z3, z3_path, AdapterLimits::default()).expect("z3");
    let cvc5 =
        SolverConfig::new(SolverEngine::Cvc5, cvc5_path, AdapterLimits::default()).expect("cvc5");
    let run_for = |statements: Vec<StatementInput>| {
        let request = AnalysisRequest::consistency(vec![], statements, vec![]).expect("request");
        let query = lower_analysis_request(&request).expect("lower");
        let run =
            quire_analyze::execute_differential(&query, &z3, &cvc5, &CancellationToken::default())
                .expect("run");
        (query, run)
    };
    let (query, run) = run_for(unsat_fixture());
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(
        run.agreed_status(),
        Some(quire_analyze::AnalysisStatus::Refuted)
    );
    let report = render_differential_report(&query, &run).expect("report");
    validate_differential_report(&report, &query, &run).expect("valid");

    // Both engines answer sat and each returned model replays against every assertion.
    let (_, run) = run_for(sat_fixture());
    for conclusion in [run.z3(), run.cvc5()] {
        assert_eq!(
            conclusion.solver().outcome(),
            quire_analyze::SolverOutcome::Sat
        );
        assert_eq!(
            conclusion.explanation(),
            quire_analyze::ExplanationState::Verified
        );
        assert_eq!(
            conclusion
                .verified_model()
                .expect("model")
                .integer_values()
                .len(),
            2
        );
    }
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(
        run.agreed_status(),
        Some(quire_analyze::AnalysisStatus::Satisfied)
    );
}

/// The query symbol declared for the value named `name`; origins end in `|name|point|digest`.
fn symbol_of(query: &QueryBundle, name: &str) -> String {
    query
        .variables()
        .iter()
        .find(|variable| variable.origins[0].split('|').nth(5) == Some(name))
        .expect("declared variable")
        .symbol
        .clone()
}

/// A fixture whose only constraint is `x >= -5`, over `x` declared in `[-5, 5]`.
fn bounded_query() -> QueryBundle {
    let ty = integer_type(-5, 5, OverflowPolicy::Reject);
    let bounded = statement(
        "bounded",
        &[("x", ty.clone())],
        compare(ComparisonOperator::GreaterEqual, var("x"), lit(-5, &ty)),
    );
    let request = AnalysisRequest::consistency(vec![], vec![bounded], vec![]).expect("request");
    lower_analysis_request(&request).expect("lower")
}

fn z3_model(definitions: &[String]) -> String {
    format!("sat\n(model {})\n", definitions.join(" "))
}

fn cvc5_model(definitions: &[String]) -> String {
    format!("sat\n({})\n", definitions.join(" "))
}

fn define(symbol: &str, sort: &str, value: &str) -> String {
    format!("(define-fun {symbol} () {sort} {value})")
}

/// Classifies a `sat` response and asserts the FR-011-AC-8 contract: the logical status and
/// adapter outcome stay `sat`/`satisfied` and the evidence is incomplete, never verified.
fn assert_replay_fails(
    query: &QueryBundle,
    response: &str,
    reason: ReplayFailureReason,
    subject: Option<&str>,
) {
    let record = response_record(query, response, SolverEngine::Z3);
    assert_eq!(record.outcome(), quire_analyze::SolverOutcome::Sat);
    let conclusion = classify_analysis(query, &record);
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied, "{response}");
    assert_eq!(conclusion.explanation(), ExplanationState::Incomplete);
    assert!(conclusion.verified_model().is_none());
    let failure = conclusion.replay_failure().expect("typed failure");
    assert_eq!(failure.reason(), reason, "{response}");
    assert_eq!(failure.subject(), subject, "{response}");
    assert!(conclusion
        .diagnostic()
        .expect("diagnostic")
        .starts_with(reason.as_str()));
}

fn assert_replay_verifies(query: &QueryBundle, response: &str, engine: SolverEngine) {
    let record = response_record(query, response, engine);
    let conclusion = classify_analysis(query, &record);
    assert_eq!(
        conclusion.explanation(),
        ExplanationState::Verified,
        "{response}"
    );
    assert!(conclusion.replay_failure().is_none());
}

/// FR-011-AC-1 and AC-2 (recorded model texts): each solver's spelling, negative values and echoed
/// aliases verify, and the published witness lists every integer value with its origins.
#[trace("TC-017", "FR-011-AC-1", "FR-011-AC-2")]
#[test]
fn recorded_models_in_both_formats_verify_and_publish_values() {
    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let (x, y) = (symbol_of(&query, "x"), symbol_of(&query, "y"));
    let alias = query.assertions()[0].name.clone();
    for (engine, response, expected) in [
        (
            SolverEngine::Z3,
            z3_model(&[
                define(&x, "Int", "7"),
                define(&y, "Int", "3"),
                define(&alias, "Bool", "true"),
            ]),
            (7, 3),
        ),
        (
            SolverEngine::Cvc5,
            cvc5_model(&[define(&x, "Int", "14"), define(&y, "Int", "(- 4)")]),
            (14, -4),
        ),
    ] {
        let record = response_record(&query, &response, engine);
        let conclusion = classify_analysis(&query, &record);
        assert_eq!(
            conclusion.explanation(),
            ExplanationState::Verified,
            "{response}"
        );
        let model = conclusion.verified_model().expect("verified model");
        assert_eq!(model.replayed_assertions().len(), 1);
        let published: Vec<_> = model
            .integer_values()
            .iter()
            .map(|value| (value.symbol().to_owned(), value.value()))
            .collect();
        let mut expected_values = vec![(x.clone(), expected.0), (y.clone(), expected.1)];
        expected_values.sort();
        assert_eq!(published, expected_values);
        assert!(model.values().is_empty());
        assert!(model
            .integer_values()
            .iter()
            .all(|value| value.origins().len() == 1));
    }
}

/// FR-011-AC-2: Boolean and integer symbols mix in one model; each keeps its own sort.
#[trace("TC-017", "FR-011-AC-2")]
#[test]
fn boolean_and_integer_symbols_mix_in_one_model() {
    let ty = narrow();
    let request = AnalysisRequest::consistency(
        vec![],
        vec![try_mixed_statement(
            "mixed",
            &[("x", ty.clone())],
            &["flag"],
            and(
                var("flag"),
                compare(ComparisonOperator::GreaterEqual, var("x"), lit(0, &ty)),
            ),
        )
        .expect("statement")],
        vec![],
    )
    .expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let (x, flag) = (symbol_of(&query, "x"), symbol_of(&query, "flag"));
    let ok = [define(&x, "Int", "4"), define(&flag, "Bool", "true")];
    let record = response_record(&query, &z3_model(&ok), SolverEngine::Z3);
    let conclusion = classify_analysis(&query, &record);
    let model = conclusion.verified_model().expect("verified model");
    assert_eq!(model.integer_values()[0].value(), 4);
    assert!(model.values()[0].value());
    assert_replay_fails(
        &query,
        &z3_model(&[define(&x, "Int", "4"), define(&flag, "Bool", "false")]),
        ReplayFailureReason::AssertionFalse,
        Some(&query.assertions()[0].name),
    );
    assert_replay_fails(
        &query,
        &z3_model(&[define(&x, "Int", "4"), define(&flag, "Int", "1")]),
        ReplayFailureReason::SortMismatch,
        Some(&flag),
    );
}

/// FR-011-AC-3: a model that falsifies the assertion is incomplete, naming that assertion.
#[trace("TC-017", "FR-011-AC-3", "FR-011-AC-8")]
#[test]
fn a_false_assertion_is_reported_incomplete() {
    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let (x, y) = (symbol_of(&query, "x"), symbol_of(&query, "y"));
    let name = query.assertions()[0].name.clone();
    assert_replay_fails(
        &query,
        &z3_model(&[define(&x, "Int", "1"), define(&y, "Int", "1")]),
        ReplayFailureReason::AssertionFalse,
        Some(&name),
    );
}

/// FR-011-AC-4 and AC-8: missing, unknown, repeated and mis-sorted symbols.
#[trace("TC-017", "FR-011-AC-4", "FR-011-AC-8")]
#[test]
fn symbol_set_defects_have_their_reasons() {
    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let (x, y) = (symbol_of(&query, "x"), symbol_of(&query, "y"));
    let ok = [define(&x, "Int", "7"), define(&y, "Int", "3")];
    assert_replay_fails(
        &query,
        &z3_model(&ok[..1]),
        ReplayFailureReason::SymbolMissing,
        Some(&y),
    );
    let mut unknown = ok.to_vec();
    unknown.push(define("extra", "Int", "1"));
    assert_replay_fails(
        &query,
        &z3_model(&unknown),
        ReplayFailureReason::SymbolUnknown,
        Some("extra"),
    );
    let mut repeated = ok.to_vec();
    repeated.push(define(&x, "Int", "7"));
    assert_replay_fails(
        &query,
        &z3_model(&repeated),
        ReplayFailureReason::SymbolRepeated,
        Some(&x),
    );
    assert_replay_fails(
        &query,
        &z3_model(&[define(&x, "Bool", "true"), define(&y, "Int", "3")]),
        ReplayFailureReason::SortMismatch,
        Some(&x),
    );
    assert_replay_fails(
        &query,
        &z3_model(&[define(&x, "Int", "true"), define(&y, "Int", "3")]),
        ReplayFailureReason::SortMismatch,
        Some(&x),
    );
    assert_replay_verifies(&query, &z3_model(&ok), SolverEngine::Z3);
}

/// FR-011-AC-5: one outside each bound fails; each bound itself verifies.
#[trace("TC-017", "FR-011-AC-5", "FR-011-AC-8")]
#[test]
fn declared_bounds_are_checked_directly() {
    let query = bounded_query();
    let x = only_symbol(&query).to_owned();
    for (value, reason) in [
        ("6", Some(ReplayFailureReason::ValueOutOfDomain)),
        ("(- 6)", Some(ReplayFailureReason::ValueOutOfDomain)),
        ("5", None),
        ("(- 5)", None),
    ] {
        let response = z3_model(&[define(&x, "Int", value)]);
        match reason {
            Some(reason) => assert_replay_fails(&query, &response, reason, Some(&x)),
            None => assert_replay_verifies(&query, &response, SolverEngine::Z3),
        }
    }
}

/// FR-011-AC-6: malformed numerals and the numeral boundary, without a panic.
#[trace("TC-017", "FR-011-AC-6", "FR-011-AC-8")]
#[test]
fn malformed_and_oversized_numerals_are_typed_failures() {
    let query = bounded_query();
    let x = only_symbol(&query).to_owned();
    let long = "1".repeat(1025);
    for (sort, value) in [
        ("Int", "1.5"),
        ("Real", "1.0"),
        ("Int", "1e3"),
        ("Int", long.as_str()),
    ] {
        assert_replay_fails(
            &query,
            &z3_model(&[define(&x, sort, value)]),
            ReplayFailureReason::ValueMalformed,
            Some(&x),
        );
    }
    let at_limit = format!("1{}", "0".repeat(1023));
    let two_127 = (1u128 << 127).to_string();
    for value in [at_limit.as_str(), two_127.as_str()] {
        assert_replay_fails(
            &query,
            &z3_model(&[define(&x, "Int", value)]),
            ReplayFailureReason::ValueOutOfDomain,
            Some(&x),
        );
    }
}

/// FR-011-AC-10: the reason precedence over combined defects.
#[trace("TC-017", "FR-011-AC-10")]
#[test]
fn combined_defects_report_the_first_reason() {
    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let (x, y) = (symbol_of(&query, "x"), symbol_of(&query, "y"));
    assert_replay_fails(
        &query,
        &z3_model(&[define(&x, "Int", "1.5")]),
        ReplayFailureReason::ValueMalformed,
        Some(&x),
    );
    // out of domain and a false assertion
    assert_replay_fails(
        &query,
        &z3_model(&[define(&x, "Int", "1001"), define(&y, "Int", "1")]),
        ReplayFailureReason::ValueOutOfDomain,
        Some(&x),
    );
}

/// FR-011-AC-11 (recorded models): both engines' replay-verified models agree, status satisfied.
#[trace("TC-017", "FR-011-AC-11")]
#[test]
fn replay_verified_models_agree() {
    let request = AnalysisRequest::consistency(vec![], sat_fixture(), vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let (x, y) = (symbol_of(&query, "x"), symbol_of(&query, "y"));
    let z3 = response_record(
        &query,
        &z3_model(&[define(&x, "Int", "7"), define(&y, "Int", "3")]),
        SolverEngine::Z3,
    );
    let cvc5 = response_record(
        &query,
        &cvc5_model(&[define(&x, "Int", "14"), define(&y, "Int", "(- 4)")]),
        SolverEngine::Cvc5,
    );
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Satisfied));
    let report = render_differential_report(&query, &run).expect("report");
    validate_differential_report(&report, &query, &run).expect("valid");

    let bad = response_record(
        &query,
        &cvc5_model(&[define(&x, "Int", "1"), define(&y, "Int", "1")]),
        SolverEngine::Cvc5,
    );
    let run = compare_solver_records(&query, &z3, &bad);
    assert_eq!(run.disposition(), DifferentialDisposition::Inconclusive);
}

/// FR-011-AC-7 (no admitted fixture overflows): a guard in the left operand of an implication
/// protects `x + 1`, so the correct model `x = MAX` verifies instead of overflowing.
#[trace("TC-017", "FR-011-AC-7", "NFR-004-AC-2")]
#[test]
fn guarded_arithmetic_at_the_extreme_replays_without_overflow() {
    let full = integer_type(i128::MIN, i128::MAX, OverflowPolicy::Reject);
    let guarded_statement = statement(
        "guarded",
        &[("x", full.clone())],
        node(ExpressionKind::Boolean {
            operator: BooleanOperator::Implication,
            left: Box::new(compare(
                ComparisonOperator::Less,
                var("x"),
                lit(i128::MAX, &full),
            )),
            right: Box::new(compare(
                ComparisonOperator::Greater,
                add(var("x"), lit(1, &full)),
                var("x"),
            )),
        }),
    );
    let request =
        AnalysisRequest::consistency(vec![], vec![guarded_statement], vec![]).expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let x = only_symbol(&query).to_owned();
    assert_replay_verifies(
        &query,
        &z3_model(&[define(&x, "Int", &i128::MAX.to_string())]),
        SolverEngine::Z3,
    );
    assert_replay_verifies(
        &query,
        &cvc5_model(&[define(&x, "Int", &format!("(- {})", 1u128 << 127))]),
        SolverEngine::Cvc5,
    );
}

/// FR-011 (model shape): a text that is not a sequence of `define-fun` entries is
/// `value_malformed` and names no symbol.
#[trace("TC-017", "FR-011-AC-6", "FR-011-AC-8")]
#[test]
fn an_unreadable_model_text_is_malformed_and_names_no_symbol() {
    let query = bounded_query();
    for response in [
        "sat\n(model (assert true))\n",
        "sat\n(model (define-fun x))\n",
    ] {
        assert_replay_fails(&query, response, ReplayFailureReason::ValueMalformed, None);
    }
}

/// NFR-004-AC-3: an integer model above the parsed-model ceiling is a non-conclusive adapter
/// outcome, never a verified witness.
#[trace("TC-017", "NFR-004-AC-3")]
#[test]
fn an_integer_model_above_the_model_ceiling_is_not_conclusive() {
    let query = bounded_query();
    let x = only_symbol(&query).to_owned();
    let response = z3_model(&[define(&x, "Int", "5")]);
    let limits = AdapterLimits {
        model_bytes: 64,
        ..AdapterLimits::default()
    };
    assert!(response.len() > 64);
    let record = response_record_with(&query, &response, SolverEngine::Z3, limits);
    assert_eq!(record.outcome(), quire_analyze::SolverOutcome::ModelLimit);
    let conclusion = classify_analysis(&query, &record);
    assert!(!conclusion.is_conclusive());
    assert_eq!(conclusion.status(), AnalysisStatus::ToolError);
    assert!(conclusion.verified_model().is_none());
    let within = response_record_with(
        &query,
        &response,
        SolverEngine::Z3,
        AdapterLimits {
            model_bytes: 4096,
            ..AdapterLimits::default()
        },
    );
    assert_eq!(within.outcome(), quire_analyze::SolverOutcome::Sat);
}
