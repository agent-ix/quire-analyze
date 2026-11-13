//! Requirement-tagged tests for integer-range quantifier lowering (FR-030, FR-032, NFR-030,
//! NFR-032, TC-030). Quantified-model replay is a separate requirement (FR-033); the cases that
//! need a verdict on a bounded quantifier assert the model stays unverified.

#![cfg(target_os = "linux")]

mod fake_script;

use ix_trace_rs::trace;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use quire_analyze::{
    classify_analysis, compare_solver_records, execute_solver, lower_analysis_request,
    lower_boolean_statements, render_differential_report, validate_differential_report,
    validate_report_document, AdapterLimits, AnalysisFormula, AnalysisRequest, AnalysisStatus,
    CancellationToken, DifferentialDisposition, ExplanationState, LoweringError, LoweringErrorCode,
    QuantifierDomainClass, QuantifierEncoding, QuantifierReason, QueryBundle, RangeBound,
    RangeQuantifier, ReplayFailureReason, SolverConfig, SolverEngine, SolverOutcome,
    StatementInput, CAPABILITY_CONTRACT,
};
use quire_contract_model::{
    AnchorName, Clause, ClauseId, ClauseKind, ComparisonOperator, ContractPackage,
    DeclarationEnvironment, EnumDeclaration, EnumVariantDeclaration, ExecutionPoint, Expression,
    ExpressionKind, IntegerDomain, IntegerType, NumericOperator, OverflowPolicy, PackageId,
    QuantifierKind, Requirement, RequirementId, RequirementRef, RequirementRevision, SchemaVersion,
    SourceDocumentId, SourceIdentity, SourceLocation, SourceRevision, SourceSpan, StateObservation,
    SymbolName, TypeDeclaration, TypedExpression, ValueDeclaration, ValueDeclarationKind,
    ValueType,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn span() -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("range-test").expect("source id"),
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

fn symbol(name: &str) -> SymbolName {
    SymbolName::new(name).expect("symbol")
}

fn node(kind: ExpressionKind) -> Expression {
    Expression::new(kind, span())
}

fn wide() -> IntegerType {
    IntegerType::new(IntegerDomain::Signed, -1000, 1000, OverflowPolicy::Reject).expect("type")
}

fn element_type() -> IntegerType {
    IntegerType::new(
        IntegerDomain::Signed,
        i128::MIN,
        i128::MAX,
        OverflowPolicy::Reject,
    )
    .expect("element type")
}

fn lit(value: i128) -> Expression {
    node(ExpressionKind::IntegerLiteral {
        value,
        value_type: wide(),
    })
}

fn var(name: &str) -> Expression {
    node(ExpressionKind::ValueReference {
        name: symbol(name),
        observation: StateObservation::Current,
    })
}

fn local(name: &str) -> Expression {
    node(ExpressionKind::LocalReference { name: symbol(name) })
}

fn compare(operator: ComparisonOperator, left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Compare {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn numeric(operator: NumericOperator, left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Numeric {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn term(expression: Expression) -> AnalysisFormula {
    AnalysisFormula::Term(Box::new(expression))
}

fn quantifier(
    identity: &str,
    kind: QuantifierKind,
    local_name: &str,
    lower: RangeBound,
    upper: RangeBound,
    body: AnalysisFormula,
) -> AnalysisFormula {
    AnalysisFormula::Range(Box::new(RangeQuantifier {
        identity: identity.to_owned(),
        kind,
        local: symbol(local_name),
        local_type: element_type(),
        lower,
        upper,
        body,
    }))
}

fn literal_range(
    identity: &str,
    kind: QuantifierKind,
    lower: i128,
    upper: i128,
    body: AnalysisFormula,
) -> AnalysisFormula {
    quantifier(
        identity,
        kind,
        "i",
        RangeBound::Literal(lower),
        RangeBound::Literal(upper),
        body,
    )
}

/// `i <= x` over the local `i`.
fn below_x() -> AnalysisFormula {
    term(compare(ComparisonOperator::LessEqual, local("i"), var("x")))
}

fn roles() -> TypeDeclaration {
    let variants = ["Admin", "Guest"]
        .iter()
        .map(|variant| EnumVariantDeclaration::new(symbol(variant), span()))
        .collect();
    TypeDeclaration::Enum {
        declaration: EnumDeclaration::new(symbol("Role"), span(), variants).expect("enum"),
    }
}

fn input(name: &str, value_type: ValueType) -> ValueDeclaration {
    ValueDeclaration::new(
        symbol(name),
        ValueDeclarationKind::Input,
        value_type,
        span(),
    )
}

/// One statement whose analysis-model body is `formula`: integer inputs `x` and `n` (0..=100),
/// a Boolean `flag` and an enum `role`.
fn statement(label: &str, formula: AnalysisFormula) -> StatementInput {
    let package_id = PackageId::new("agent-ix/range-test").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-{label}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let counter = IntegerType::new(IntegerDomain::Signed, 0, 100, OverflowPolicy::Reject)
        .expect("counter type");
    let environment = DeclarationEnvironment::new(
        owner,
        vec![roles()],
        vec![
            input("x", ValueType::Integer { value: wide() }),
            input("n", ValueType::Integer { value: counter }),
            input("flag", ValueType::Boolean),
            input(
                "role",
                ValueType::Enum {
                    name: symbol("Role"),
                },
            ),
        ],
        vec![],
    )
    .expect("environment");
    let checked = environment
        .check_expression(
            &node(ExpressionKind::BooleanLiteral { value: true }),
            &ValueType::Boolean,
            &point(),
            true,
        )
        .expect("typed");
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
    .expect("statement")
    .with_formula(formula)
}

fn lower(statements: &[StatementInput]) -> QueryBundle {
    lower_boolean_statements(statements, &[]).expect("lower")
}

fn lower_one(label: &str, formula: AnalysisFormula) -> QueryBundle {
    lower(&[statement(label, formula)])
}

fn refusal(label: &str, formula: AnalysisFormula) -> Vec<LoweringError> {
    lower_boolean_statements(&[statement(label, formula)], &[]).expect_err("refused")
}

fn encodings(query: &QueryBundle) -> Vec<(QuantifierEncoding, u128)> {
    query
        .quantifiers()
        .iter()
        .map(|entry| (entry.encoding, entry.instance_count))
        .collect()
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

/// The two fixtures of the golden queries: a universal and an existential literal range.
fn expanded_fixture() -> Vec<StatementInput> {
    vec![
        statement(
            "all",
            literal_range("Q-all", QuantifierKind::ForAll, 1, 3, below_x()),
        ),
        statement(
            "any",
            literal_range(
                "Q-any",
                QuantifierKind::Exists,
                1,
                3,
                term(compare(
                    ComparisonOperator::Equal,
                    numeric(NumericOperator::Multiply, local("i"), lit(2)),
                    var("x"),
                )),
            ),
        ),
    ]
}

fn declared_forall() -> AnalysisFormula {
    quantifier(
        "Q-declared",
        QuantifierKind::ForAll,
        "i",
        RangeBound::Literal(0),
        RangeBound::Declared(symbol("n")),
        term(compare(ComparisonOperator::LessEqual, local("i"), var("x"))),
    )
}

fn bounded_fixture() -> StatementInput {
    statement("declared", declared_forall())
}

/// `formula` together with `x < 0`, in one statement so both constrain the same `x`.
fn with_negative_x(label: &str, formula: AnalysisFormula) -> StatementInput {
    statement(
        label,
        AnalysisFormula::And(
            Box::new(formula),
            Box::new(term(compare(ComparisonOperator::Less, var("x"), lit(0)))),
        ),
    )
}

/// FR-032-AC-1, FR-032-AC-3 and FR-031-AC-3 (by the shared expansion): a literal range within
/// the budget is expanded to one body instance per element, in ascending order, matching its
/// golden query, with no solver quantifier token.
#[trace("TC-030", "FR-032-AC-1", "FR-032-AC-3", "FR-030-AC-1")]
#[test]
fn literal_range_within_budget_is_expanded_to_the_golden_query() {
    let query = lower(&expanded_fixture());
    assert_eq!(query.logic(), "QF_LIA");
    assert_eq!(query.query(), include_str!("golden/range-expanded-v2.smt2"));
    assert!(!query.query().contains("forall"));
    assert!(!query.query().contains("exists"));
    let entries = query.quantifiers();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].source, "Q-all");
    assert_eq!(entries[0].kind, QuantifierKind::ForAll);
    assert_eq!(entries[0].domain, QuantifierDomainClass::Range);
    assert_eq!(entries[0].encoding, QuantifierEncoding::Expanded);
    assert_eq!(entries[0].instance_count, 3);
    assert_eq!(entries[0].lower, RangeBound::Literal(1));
    assert_eq!(entries[0].upper, RangeBound::Literal(3));
    assert_eq!(entries[1].source, "Q-any");
    assert_eq!(entries[1].kind, QuantifierKind::Exists);
}

/// FR-032-AC-2, FR-032-AC-4 and FR-032-AC-6: a declared bound lowers to a bounded solver
/// quantifier with a finite interval guard, matching its golden query, under `LIA`.
#[trace("TC-030", "FR-032-AC-2", "FR-032-AC-4", "FR-032-AC-6", "FR-030-AC-1")]
#[test]
fn declared_bound_range_lowers_to_a_guarded_bounded_quantifier() {
    let query = lower(&[bounded_fixture()]);
    assert_eq!(query.logic(), "LIA");
    assert_eq!(query.query(), include_str!("golden/range-bounded-v2.smt2"));
    let entry = &query.quantifiers()[0];
    assert_eq!(entry.encoding, QuantifierEncoding::Bounded);
    assert_eq!(entry.instance_count, 0);
    assert_eq!(entry.lower, RangeBound::Literal(0));
    assert_eq!(entry.upper, RangeBound::Declared(symbol("n")));
}

/// FR-032-AC-4: every emitted solver quantifier carries the interval guard, in both forms.
#[trace("TC-030", "FR-032-AC-4")]
#[test]
fn every_bounded_quantifier_carries_its_interval_guard() {
    let both = statement(
        "both",
        AnalysisFormula::And(
            Box::new(literal_range(
                "Q-u",
                QuantifierKind::ForAll,
                0,
                2000,
                below_x(),
            )),
            Box::new(literal_range(
                "Q-e",
                QuantifierKind::Exists,
                0,
                2000,
                below_x(),
            )),
        ),
    );
    let text = lower(&[both]).query().to_owned();
    assert_eq!(count(&text, "(forall (("), 1);
    assert_eq!(count(&text, "(exists (("), 1);
    assert!(text.contains("(forall ((bv_0 Int)) (=> (and (<= 0 bv_0) (<= bv_0 2000))"));
    assert!(text.contains("(exists ((bv_0 Int)) (and (<= 0 bv_0) (<= bv_0 2000)"));
}

/// FR-032-AC-6: a bounded quantifier whose body mentions an enum declares `UFDTLIA`.
#[trace("TC-030", "FR-032-AC-6", "FR-005-AC-6")]
#[test]
fn bounded_quantifier_over_an_enum_body_declares_ufdtlia() {
    let enum_body = AnalysisFormula::And(
        Box::new(below_x()),
        Box::new(term(compare(
            ComparisonOperator::Equal,
            var("role"),
            node(ExpressionKind::EnumLiteral {
                enumeration: symbol("Role"),
                variant: symbol("Admin"),
            }),
        ))),
    );
    let query = lower_one(
        "enum",
        literal_range("Q-enum", QuantifierKind::Exists, 0, 5000, enum_body.clone()),
    );
    assert_eq!(query.logic(), "UFDTLIA");
    assert!(query.query().contains("(set-logic UFDTLIA)"));
    let expanded = lower_one(
        "enum-small",
        literal_range("Q-enum", QuantifierKind::Exists, 0, 5, enum_body),
    );
    assert_eq!(expanded.logic(), "QF_UFDTLIA");
}

/// NFR-030-AC-1: instance counts summing to exactly 1,024 expand; one more takes the bounded
/// form for a literal range. The budget is summed over all clauses.
#[trace("TC-030", "NFR-030-AC-1", "FR-032-AC-3")]
#[test]
fn budget_boundary_is_exactly_1024_instances() {
    let single = |width: i128| {
        lower_one(
            "single",
            literal_range("Q-w", QuantifierKind::ForAll, 1, width, below_x()),
        )
    };
    assert_eq!(
        encodings(&single(1024)),
        [(QuantifierEncoding::Expanded, 1024)]
    );
    assert_eq!(encodings(&single(1025)), [(QuantifierEncoding::Bounded, 0)]);
    assert_eq!(single(1025).logic(), "LIA");

    let pair = |second: i128| {
        lower(&[
            statement(
                "a",
                literal_range("Q-a", QuantifierKind::ForAll, 1, 1000, below_x()),
            ),
            statement(
                "b",
                literal_range("Q-b", QuantifierKind::ForAll, 1, second, below_x()),
            ),
        ])
    };
    let fits = pair(24);
    assert!(fits
        .quantifiers()
        .iter()
        .all(|entry| entry.encoding == QuantifierEncoding::Expanded));
    let over = pair(25);
    let bounded = over
        .quantifiers()
        .iter()
        .filter(|entry| entry.encoding == QuantifierEncoding::Bounded)
        .count();
    assert_eq!(bounded, 1);
    assert_eq!(over.logic(), "LIA");
}

/// NFR-030-AC-1: nested counts multiply by the enclosing expanded counts: 32 + 32 x 31 is 1,024
/// and expands; an inner width of 32 makes 1,056 and the inner range is bounded.
#[trace("TC-030", "NFR-030-AC-1")]
#[test]
fn nested_instance_counts_meet_the_budget_exactly() {
    let nested = |inner: i128| {
        lower_one(
            "nested",
            literal_range(
                "Q-outer",
                QuantifierKind::ForAll,
                1,
                32,
                quantifier(
                    "Q-inner",
                    QuantifierKind::Exists,
                    "j",
                    RangeBound::Literal(1),
                    RangeBound::Literal(inner),
                    term(compare(
                        ComparisonOperator::LessEqual,
                        numeric(NumericOperator::Add, local("i"), local("j")),
                        var("x"),
                    )),
                ),
            ),
        )
    };
    let fits = nested(31);
    assert_eq!(
        encodings(&fits),
        [
            (QuantifierEncoding::Expanded, 32),
            (QuantifierEncoding::Expanded, 992)
        ]
    );
    assert_eq!(fits.logic(), "QF_LIA");
    let over = nested(32);
    assert_eq!(
        encodings(&over),
        [
            (QuantifierEncoding::Expanded, 32),
            (QuantifierEncoding::Bounded, 0)
        ]
    );
    assert_eq!(over.logic(), "LIA");
    assert_eq!(count(over.query(), "(exists (("), 32);
}

/// NFR-030-AC-2: an outer width-100 range expands and an inner width-100 range inside it is
/// bounded, because 100 x 100 exceeds the 924 left; the map records the outer 100 instances.
#[trace("TC-030", "NFR-030-AC-2", "FR-032-AC-3")]
#[test]
fn nested_width_100_example_expands_outer_and_bounds_inner() {
    let query = lower_one(
        "hundred",
        literal_range(
            "Q-outer",
            QuantifierKind::ForAll,
            1,
            100,
            quantifier(
                "Q-inner",
                QuantifierKind::ForAll,
                "j",
                RangeBound::Literal(1),
                RangeBound::Literal(100),
                term(compare(
                    ComparisonOperator::LessEqual,
                    numeric(NumericOperator::Add, local("i"), local("j")),
                    var("x"),
                )),
            ),
        ),
    );
    assert_eq!(
        encodings(&query),
        [
            (QuantifierEncoding::Expanded, 100),
            (QuantifierEncoding::Bounded, 0)
        ]
    );
    assert_eq!(count(query.query(), "(forall (("), 100);
}

/// NFR-030-AC-3: ranges of width 2^64 and 2^128, and nested ranges whose product exceeds 2^128,
/// exceed the budget without wrapping or panicking, and take the bounded form.
#[trace("TC-030", "NFR-030-AC-3")]
#[test]
fn huge_widths_exceed_the_budget_without_wrapping() {
    let two_pow_64 = 1_i128 << 64;
    for (lower, upper) in [
        (0, two_pow_64 - 1),
        (i128::MIN, i128::MAX),
        (i128::MIN, i128::MAX - 1),
        (-1, i128::MAX),
    ] {
        let query = lower_one(
            "huge",
            literal_range("Q-huge", QuantifierKind::ForAll, lower, upper, below_x()),
        );
        assert_eq!(encodings(&query), [(QuantifierEncoding::Bounded, 0)]);
    }
    let nested = lower_one(
        "huge-nested",
        literal_range(
            "Q-outer",
            QuantifierKind::ForAll,
            i128::MIN,
            i128::MAX,
            quantifier(
                "Q-inner",
                QuantifierKind::ForAll,
                "j",
                RangeBound::Literal(i128::MIN),
                RangeBound::Literal(i128::MAX),
                term(compare(
                    ComparisonOperator::LessEqual,
                    numeric(NumericOperator::Add, local("i"), local("j")),
                    var("x"),
                )),
            ),
        ),
    );
    assert_eq!(
        encodings(&nested),
        [
            (QuantifierEncoding::Bounded, 0),
            (QuantifierEncoding::Bounded, 0)
        ]
    );
    // Two nested solver quantifiers name distinct bound variables.
    assert!(nested.query().contains("bv_0"));
    assert!(nested.query().contains("bv_1"));
}

/// FR-030-AC-4: a universal quantifier over an empty range is true, an existential one false,
/// and an empty range records zero instances.
#[trace("TC-030", "FR-030-AC-4")]
#[test]
fn empty_range_is_true_for_universal_and_false_for_existential() {
    let all = lower_one(
        "empty-all",
        literal_range("Q-all", QuantifierKind::ForAll, 5, 4, below_x()),
    );
    assert!(all.query().contains(":named"));
    assert!(all.query().contains("(assert (! true :named"));
    assert_eq!(encodings(&all), [(QuantifierEncoding::Expanded, 0)]);
    let any = lower_one(
        "empty-any",
        literal_range("Q-any", QuantifierKind::Exists, 5, 4, below_x()),
    );
    assert!(any.query().contains("(assert (! false :named"));
    assert_eq!(any.logic(), "QF_LIA");
}

/// FR-032-AC-2 and FR-030-AC-2: bounds that are not provably finite are refused with their
/// reason class and the quantifier's identity, and no query exists.
#[trace("TC-030", "FR-032-AC-2", "FR-030-AC-2")]
#[test]
fn non_finite_bounds_are_unsupported() {
    let cases = [
        (RangeBound::Unbounded, RangeBound::Literal(3)),
        (RangeBound::Literal(0), RangeBound::Unbounded),
        // A Boolean input has no integer bounds, and an undeclared name has none either.
        (RangeBound::Literal(0), RangeBound::Declared(symbol("flag"))),
        (
            RangeBound::Literal(0),
            RangeBound::Declared(symbol("missing")),
        ),
    ];
    for (lower, upper) in cases {
        let errors = refusal(
            "infinite",
            quantifier(
                "Q-inf",
                QuantifierKind::ForAll,
                "i",
                lower,
                upper,
                below_x(),
            ),
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedConstruct);
        let refused = errors[0].quantifier_refusal().expect("refusal");
        assert_eq!(refused.identity, "Q-inf");
        assert_eq!(refused.reason, QuantifierReason::NonFiniteBound);
        assert_eq!(refused.reason.as_str(), "non-finite-bound");
    }
}

/// FR-030-AC-3: a refused quantifier nested inside admitted ones refuses the whole request, and
/// a refused quantifier in a second statement leaves no partial query.
#[trace("TC-030", "FR-030-AC-3")]
#[test]
fn a_refused_nested_quantifier_refuses_the_whole_request() {
    let nested = literal_range(
        "Q-outer",
        QuantifierKind::ForAll,
        1,
        3,
        quantifier(
            "Q-bad",
            QuantifierKind::Exists,
            "j",
            RangeBound::Literal(0),
            RangeBound::Unbounded,
            below_x(),
        ),
    );
    let errors = refusal("nested-bad", nested);
    assert_eq!(
        errors[0].quantifier_refusal().expect("refusal").identity,
        "Q-bad"
    );
    let good = statement(
        "good",
        literal_range("Q-good", QuantifierKind::ForAll, 1, 3, below_x()),
    );
    let bad = statement(
        "bad",
        quantifier(
            "Q-bad",
            QuantifierKind::ForAll,
            "i",
            RangeBound::Unbounded,
            RangeBound::Literal(3),
            below_x(),
        ),
    );
    assert!(lower_boolean_statements(&[good, bad], &[]).is_err());
}

/// FR-032-AC-5 and FR-031-AC-6 (shadowing): lowering twice is byte-identical, statement order
/// does not matter, and an inner quantifier that reuses the bound name shadows the outer one.
#[trace("TC-030", "FR-032-AC-5", "FR-031-AC-6")]
#[test]
fn lowering_is_deterministic_and_inner_quantifiers_shadow() {
    let forward = lower(&expanded_fixture());
    let again = lower(&expanded_fixture());
    assert_eq!(forward.query(), again.query());
    let mut reversed = expanded_fixture();
    reversed.reverse();
    let backward = lower(&reversed);
    assert_eq!(forward.query(), backward.query());
    assert_eq!(forward.quantifiers(), backward.quantifiers());

    // forall i in 1..=2. exists i in 5..=6. i <= x : the inner i is 5 or 6, never the outer 1 or 2.
    let shadowed = lower_one(
        "shadow",
        literal_range(
            "Q-outer",
            QuantifierKind::ForAll,
            1,
            2,
            literal_range("Q-inner", QuantifierKind::Exists, 5, 6, below_x()),
        ),
    );
    let text = shadowed.query();
    assert!(text.contains(" 5 "));
    assert!(text.contains(" 6 "));
    assert!(!text.contains("(<= 1 "));
    assert!(!text.contains("(<= 2 "));
}

/// FR-032-AC-2: a bounded quantifier whose declared bound is an input also records that input
/// in the variable map and its range assertion bounds the guard.
#[trace("TC-030", "FR-032-AC-2")]
#[test]
fn declared_bound_is_a_query_variable_with_a_finite_declared_range() {
    let query = lower(&[bounded_fixture()]);
    let text = query.query();
    assert_eq!(query.variables().len(), 2);
    assert!(text.contains("(assert (and (<= 0 "));
}

/// Capability contract: the range quantifier is an exact construct.
#[trace("TC-030", "FR-030-AC-1")]
#[test]
fn capability_contract_lists_the_range_quantifier() {
    assert!(CAPABILITY_CONTRACT
        .exact_constructs
        .contains(&"range_quantifier"));
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quire-range-{label}-{}-{counter}",
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

fn script_record(
    query: &QueryBundle,
    action: &str,
    engine: SolverEngine,
    limits: AdapterLimits,
) -> quire_analyze::SolverRecord {
    let directory = TempDirectory::new("solver");
    let path = directory.path().join("solver");
    let body = format!(
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ] || [ \"$1\" = \"--version\" ]; then printf 'fake 1.0\\n'; exit 0; fi\n/bin/cat >/dev/null; {action}\n"
    );
    fake_script::install(&path, &body);
    let config = SolverConfig::new(engine, &path, limits).expect("config");
    execute_solver(query, &config, &CancellationToken::default())
}

fn response_record(
    query: &QueryBundle,
    response: &str,
    engine: SolverEngine,
) -> quire_analyze::SolverRecord {
    script_record(
        query,
        &format!("printf '%s' '{response}'"),
        engine,
        AdapterLimits::default(),
    )
}

fn analysis_query(statements: Vec<StatementInput>) -> QueryBundle {
    let request = AnalysisRequest::consistency(vec![], statements, vec![]).expect("request");
    lower_analysis_request(&request).expect("lower")
}

fn symbol_of(query: &QueryBundle, index: usize) -> String {
    query.variables()[index].symbol.clone()
}

/// NFR-032-AC-1: a solver that exceeds its wall-time limit on a bounded-quantifier query yields
/// `timeout`, never `satisfied` or `refuted`.
#[trace("TC-030", "NFR-032-AC-1")]
#[test]
fn a_stalled_solver_on_a_quantified_query_is_a_timeout() {
    let query = analysis_query(vec![bounded_fixture()]);
    let limits = AdapterLimits {
        wall_time_ms: 50,
        cleanup_time_ms: 1_000,
        graceful_cleanup_ms: 20,
        monitor_interval_ms: 2,
        ..AdapterLimits::default()
    };
    let record = script_record(&query, "/bin/sleep 30", SolverEngine::Z3, limits);
    assert_eq!(record.outcome(), SolverOutcome::TimedOut);
    let conclusion = classify_analysis(&query, &record);
    assert_eq!(conclusion.status(), AnalysisStatus::Timeout);
}

/// NFR-032-AC-2: a solver answering `unknown` on a bounded-quantifier query yields `unknown`
/// for each engine, and the pair is not a conclusive agreement.
#[trace("TC-030", "NFR-032-AC-2")]
#[test]
fn unknown_on_a_quantified_query_is_inconclusive() {
    let query = analysis_query(vec![bounded_fixture()]);
    let z3 = response_record(&query, "unknown\n", SolverEngine::Z3);
    let cvc5 = response_record(&query, "unknown\n", SolverEngine::Cvc5);
    assert_eq!(
        classify_analysis(&query, &z3).status(),
        AnalysisStatus::Unknown
    );
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert!(!run.is_conclusive());
}

/// FR-032-AC-1 (controlled responses) and FR-005-AC-6: an unsatisfiable bounded query agrees on
/// unsat, and its report names `LIA`; a report naming a logic the feature set does not require
/// is refused.
#[trace("TC-030", "FR-032-AC-1", "FR-005-AC-6", "FR-032-AC-6")]
#[test]
fn bounded_query_reports_the_quantified_logic_and_validation_requires_it() {
    // forall i in 0..=n. i <= x together with x < 0 is unsatisfiable (i = 0 fails).
    let query = analysis_query(vec![with_negative_x("negative", declared_forall())]);
    assert_eq!(query.logic(), "LIA");
    let z3 = response_record(&query, "unsat\n", SolverEngine::Z3);
    let cvc5 = response_record(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Refuted));
    let report = render_differential_report(&query, &run).expect("report");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
    assert_eq!(value["logic"], "LIA");
    validate_report_document(&report).expect("document valid");
    validate_differential_report(&report, &query, &run).expect("valid");
    for wrong in ["QF_LIA", "UFDTLIA", "QF_UFDTLIA", "QF_UF"] {
        let mut mutated = value.clone();
        mutated["logic"] = serde_json::Value::String(wrong.to_owned());
        let bytes = serde_json::to_vec(&mutated).expect("bytes");
        assert!(validate_report_document(&bytes).is_err(), "{wrong}");
        assert!(validate_differential_report(&bytes, &query, &run).is_err());
    }
}

/// FR-032-AC-1 (controlled responses): a satisfiable expanded query is replayed exactly, because
/// expansion is quantifier-free: a model satisfying every instance verifies and one that fails
/// the expanded disjunction does not.
#[trace("TC-030", "FR-032-AC-1", "FR-032-AC-3")]
#[test]
fn expanded_range_models_replay_against_every_instance() {
    let query = analysis_query(vec![statement(
        "any",
        literal_range(
            "Q-any",
            QuantifierKind::Exists,
            1,
            3,
            term(compare(
                ComparisonOperator::Equal,
                numeric(NumericOperator::Multiply, local("i"), lit(2)),
                var("x"),
            )),
        ),
    )]);
    let x = symbol_of(&query, 0);
    let good = format!("sat\n(model (define-fun {x} () Int 4))\n");
    let conclusion = classify_analysis(&query, &response_record(&query, &good, SolverEngine::Z3));
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    assert!(conclusion.verified_model().is_some());
    let bad = format!("sat\n(model (define-fun {x} () Int 5))\n");
    let conclusion = classify_analysis(&query, &response_record(&query, &bad, SolverEngine::Z3));
    assert!(conclusion.verified_model().is_none());
    assert_eq!(
        conclusion.replay_failure().expect("failure").reason(),
        ReplayFailureReason::AssertionFalse
    );
}

/// FR-032-AC-1 (controlled responses): a satisfiable bounded query is decided sat, but its
/// model stays unverified until quantified replay exists.
#[trace("TC-030", "FR-032-AC-1")]
#[test]
fn bounded_quantifier_models_stay_unverified() {
    let query = analysis_query(vec![bounded_fixture()]);
    let symbols: Vec<_> = (0..query.variables().len())
        .map(|index| symbol_of(&query, index))
        .collect();
    let model = format!(
        "sat\n(model {})\n",
        symbols
            .iter()
            .map(|symbol| format!("(define-fun {symbol} () Int 7)"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let record = response_record(&query, &model, SolverEngine::Z3);
    let conclusion = classify_analysis(&query, &record);
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    assert!(conclusion.verified_model().is_none());
    assert_eq!(conclusion.explanation(), ExplanationState::Incomplete);
    assert_eq!(
        conclusion.replay_failure().expect("failure").reason(),
        ReplayFailureReason::SortNotReplayable
    );
}

/// FR-032-AC-1: real solvers decide the expanded and bounded fixtures; run only where installed.
#[trace("TC-030", "FR-032-AC-1", "NFR-032-AC-3")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn real_z3_cvc5_decide_range_fixtures() {
    let z3_path = PathBuf::from(std::env::var_os("QUIRE_Z3").expect("QUIRE_Z3 is required"));
    let cvc5_path = PathBuf::from(std::env::var_os("QUIRE_CVC5").expect("QUIRE_CVC5 is required"));
    let z3 = SolverConfig::new(SolverEngine::Z3, z3_path, AdapterLimits::default()).expect("z3");
    let cvc5 =
        SolverConfig::new(SolverEngine::Cvc5, cvc5_path, AdapterLimits::default()).expect("cvc5");
    let token = CancellationToken::default();
    let enum_body = AnalysisFormula::And(
        Box::new(below_x()),
        Box::new(term(compare(
            ComparisonOperator::Equal,
            var("role"),
            node(ExpressionKind::EnumLiteral {
                enumeration: symbol("Role"),
                variant: symbol("Admin"),
            }),
        ))),
    );
    let unsat = [
        analysis_query(vec![statement(
            "all",
            AnalysisFormula::And(
                Box::new(literal_range(
                    "Q-all",
                    QuantifierKind::ForAll,
                    1,
                    3,
                    below_x(),
                )),
                Box::new(term(compare(ComparisonOperator::Less, var("x"), lit(3)))),
            ),
        )]),
        analysis_query(vec![with_negative_x("negative", declared_forall())]),
        // exists i in 0..=2000. i <= x, with x < 0: unsatisfiable, bounded, with a datatype.
        analysis_query(vec![with_negative_x(
            "enum",
            literal_range("Q-enum", QuantifierKind::Exists, 0, 2000, enum_body.clone()),
        )]),
    ];
    for query in &unsat {
        let run = quire_analyze::execute_differential(query, &z3, &cvc5, &token).expect("run");
        assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
        assert_eq!(
            run.agreed_status(),
            Some(AnalysisStatus::Refuted),
            "{}",
            query.query()
        );
        let report = render_differential_report(query, &run).expect("report");
        validate_differential_report(&report, query, &run).expect("valid report");
    }
    assert_eq!(unsat[2].logic(), "UFDTLIA");
    // Satisfiable bounded and expanded fixtures: both solvers answer sat.
    let sat = [
        analysis_query(vec![bounded_fixture()]),
        analysis_query(expanded_fixture()),
    ];
    for query in &sat {
        for config in [&z3, &cvc5] {
            let record = execute_solver(query, config, &token);
            assert_eq!(record.outcome(), SolverOutcome::Sat);
        }
    }
}

/// NFR-032-AC-3: a real solver under a wall-time limit too short to finish a quantified query is
/// reported inconclusive, never satisfied or refuted.
#[trace("TC-030", "NFR-032-AC-3")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn real_solver_timeout_on_a_quantified_query_is_inconclusive() {
    let z3_path = PathBuf::from(std::env::var_os("QUIRE_Z3").expect("QUIRE_Z3 is required"));
    let limits = AdapterLimits {
        wall_time_ms: 1,
        cleanup_time_ms: 1_000,
        graceful_cleanup_ms: 20,
        monitor_interval_ms: 1,
        ..AdapterLimits::default()
    };
    let z3 = SolverConfig::new(SolverEngine::Z3, z3_path, limits).expect("z3");
    let query = analysis_query(vec![bounded_fixture()]);
    let record = execute_solver(&query, &z3, &CancellationToken::default());
    let status = classify_analysis(&query, &record).status();
    assert!(
        matches!(status, AnalysisStatus::Timeout | AnalysisStatus::ToolError),
        "{status:?}"
    );
}
