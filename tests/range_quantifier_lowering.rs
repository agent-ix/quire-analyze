//! Requirement-tagged tests for quantifier lowering over integer ranges, enum domains and finite
//! collections (FR-030, FR-031, FR-032, NFR-030, NFR-032, TC-030). Quantified-model replay is a
//! separate requirement (FR-033), which replays the original bound after lowering.

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
    CancellationToken, CollectionKind, DifferentialDisposition, DomainElement, DomainQuantifier,
    ExplanationState, LoweringError, LoweringErrorCode, QuantifierDomain, QuantifierDomainClass,
    QuantifierEncoding, QuantifierEntryDomain, QuantifierReason, QueryBundle, RangeBound,
    RangeQuantifier, ReplayFailureReason, SolverConfig, SolverEngine, SolverOutcome,
    StatementInput, UnboundedDomain, CAPABILITY_CONTRACT,
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
    let variants = ["Guest", "Admin"]
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
            input("i", ValueType::Integer { value: wide() }),
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
    .expect("formula")
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
    assert_eq!(entries[0].domain(), QuantifierDomainClass::Range);
    assert_eq!(entries[0].encoding, QuantifierEncoding::Expanded);
    assert_eq!(entries[0].instance_count, 3);
    assert_eq!(
        entries[0].domain_data,
        QuantifierEntryDomain::Range {
            lower: RangeBound::Literal(1),
            upper: RangeBound::Literal(3),
        }
    );
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
    assert_eq!(
        entry.domain_data,
        QuantifierEntryDomain::Range {
            lower: RangeBound::Literal(0),
            upper: RangeBound::Declared(symbol("n")),
        }
    );
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
#[trace("TC-030", "FR-032-AC-5")]
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
#[trace("TC-030")]
#[test]
fn capability_contract_lists_the_range_quantifier() {
    assert!(CAPABILITY_CONTRACT
        .exact_constructs
        .contains(&"range_quantifier"));
}

/// FR-031-AC-6 (shared expansion): a free variable named like a quantifier's bound variable is
/// not captured. The input `i` and the bound variable `i` lower to distinct symbols, in the
/// expanded form (elements 1 and 2) and the bounded form (`bv_0`).
#[trace("TC-030", "FR-031-AC-6")]
#[test]
fn a_free_variable_named_like_a_bound_variable_is_not_captured() {
    let against_input = |identity: &str, upper: i128| {
        literal_range(
            identity,
            QuantifierKind::ForAll,
            1,
            upper,
            term(compare(ComparisonOperator::LessEqual, local("i"), var("i"))),
        )
    };
    let both = AnalysisFormula::And(
        Box::new(against_input("Q-small", 2)),
        Box::new(against_input("Q-large", 2000)),
    );
    let query = lower_one("capture", both);
    assert_eq!(
        query.query(),
        include_str!("golden/range-free-variable-v2.smt2")
    );
    assert_eq!(query.variables().len(), 1);
    let input_symbol = &query.variables()[0].symbol;
    assert!(query.query().contains(&format!("(<= 1 {input_symbol})")));
    assert!(query.query().contains(&format!("(<= 2 {input_symbol})")));
    assert!(query.query().contains(&format!("(<= bv_0 {input_symbol})")));
    assert!(!query.query().contains("(<= bv_0 bv_0)"));
}

/// NFR-030-AC-1 and FR-030 canonical order: the budget is spent in assertion-map order, then in
/// depth-first pre-order inside a statement, so a contested budget gives the same encodings
/// whatever order the caller lists the statements in.
#[trace("TC-030", "NFR-030-AC-1", "FR-032-AC-9")]
#[test]
fn a_contested_budget_is_spent_in_canonical_order() {
    let build = |reversed: bool| {
        let mut statements = vec![
            statement(
                "a",
                literal_range("Q-a", QuantifierKind::ForAll, 1, 1000, below_x()),
            ),
            statement(
                "b",
                literal_range("Q-b", QuantifierKind::ForAll, 1, 25, below_x()),
            ),
        ];
        if reversed {
            statements.reverse();
        }
        lower(&statements)
    };
    let forward = build(false);
    let backward = build(true);
    assert_eq!(forward.query(), backward.query());
    let described: Vec<_> = forward
        .quantifiers()
        .iter()
        .map(|entry| (entry.source.as_str(), entry.encoding, entry.instance_count))
        .collect();
    assert_eq!(
        described,
        [
            ("Q-a", QuantifierEncoding::Expanded, 1000),
            ("Q-b", QuantifierEncoding::Bounded, 0)
        ]
    );

    // Depth-first pre-order: A (100) then its body B (5 x 100 = 500), then the sibling C (900)
    // finds 600 spent and is bounded. Breadth-first would expand C and bound B.
    let nested = lower_one(
        "preorder",
        AnalysisFormula::And(
            Box::new(literal_range(
                "Q-a",
                QuantifierKind::ForAll,
                1,
                100,
                quantifier(
                    "Q-b",
                    QuantifierKind::ForAll,
                    "j",
                    RangeBound::Literal(1),
                    RangeBound::Literal(5),
                    below_x(),
                ),
            )),
            Box::new(literal_range(
                "Q-c",
                QuantifierKind::ForAll,
                1,
                900,
                below_x(),
            )),
        ),
    );
    assert_eq!(
        encodings(&nested),
        [
            (QuantifierEncoding::Expanded, 100),
            (QuantifierEncoding::Expanded, 500),
            (QuantifierEncoding::Bounded, 0)
        ]
    );
}

/// NFR-030-AC-3: a product that overflows `u128` counts as exceeding the budget. An inner range
/// of width 2^128 - 1 under an expanded outer range of 2 would need 2^129 - 2 instances.
#[trace("TC-030", "NFR-030-AC-3")]
#[test]
fn a_nested_count_that_overflows_is_bounded_not_wrapped() {
    let query = lower_one(
        "product",
        literal_range(
            "Q-outer",
            QuantifierKind::ForAll,
            1,
            2,
            quantifier(
                "Q-inner",
                QuantifierKind::ForAll,
                "j",
                RangeBound::Literal(i128::MIN),
                RangeBound::Literal(i128::MAX - 1),
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
            (QuantifierEncoding::Expanded, 2),
            (QuantifierEncoding::Bounded, 0)
        ]
    );
}

/// FR-032-AC-7: the formula attached to a statement is validated: an unbound bound-variable
/// reference, an empty identity and a range outside the bound variable's type are refused.
#[trace("TC-030", "FR-032-AC-7")]
#[test]
fn an_invalid_formula_is_refused_when_attached() {
    let base = || {
        let package_id = PackageId::new("agent-ix/range-test").expect("package");
        let requirement_id = RequirementId::new("REQ-valid").expect("requirement");
        let revision = RequirementRevision::new(1).expect("revision");
        let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
        let narrow = IntegerType::new(IntegerDomain::Signed, 0, 100, OverflowPolicy::Reject)
            .expect("narrow");
        let environment = DeclarationEnvironment::new(
            owner,
            vec![],
            vec![
                input("x", ValueType::Integer { value: wide() }),
                input("n", ValueType::Integer { value: narrow }),
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
            ClauseId::new("C-valid").expect("clause"),
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
    };
    let narrow_type =
        || IntegerType::new(IntegerDomain::Signed, 0, 255, OverflowPolicy::Reject).expect("type");
    let range = |identity: &str, lower: RangeBound, upper: RangeBound, body: AnalysisFormula| {
        AnalysisFormula::Range(Box::new(RangeQuantifier {
            identity: identity.to_owned(),
            kind: QuantifierKind::ForAll,
            local: symbol("i"),
            local_type: narrow_type(),
            lower,
            upper,
            body,
        }))
    };
    let in_scope = term(compare(ComparisonOperator::LessEqual, local("i"), var("x")));
    // Valid: a range inside the type, and a declared bound inside the type.
    for formula in [
        range(
            "Q",
            RangeBound::Literal(0),
            RangeBound::Literal(255),
            in_scope.clone(),
        ),
        range(
            "Q",
            RangeBound::Literal(0),
            RangeBound::Declared(symbol("n")),
            in_scope.clone(),
        ),
        range(
            "Q",
            RangeBound::Literal(500),
            RangeBound::Literal(400),
            in_scope.clone(),
        ),
    ] {
        base().with_formula(formula).expect("valid");
    }
    // The bound variable is not in scope outside its quantifier.
    let unbound = term(compare(ComparisonOperator::LessEqual, local("i"), var("x")));
    // A range outside the type, an empty identity and a declared bound outside the type.
    let wide_declared = range(
        "Q",
        RangeBound::Literal(0),
        RangeBound::Declared(symbol("x")),
        in_scope.clone(),
    );
    for invalid in [
        unbound,
        range(
            "Q",
            RangeBound::Literal(0),
            RangeBound::Literal(256),
            in_scope.clone(),
        ),
        range(
            "",
            RangeBound::Literal(0),
            RangeBound::Literal(3),
            in_scope.clone(),
        ),
        wide_declared,
    ] {
        let errors = base().with_formula(invalid).expect_err("refused");
        assert!(errors
            .iter()
            .all(|error| error.code() == LoweringErrorCode::InvalidStatement));
    }
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
    let failure = conclusion.replay_failure().expect("failure");
    assert_eq!(failure.reason(), ReplayFailureReason::AssertionFalse);
    let quantifier = failure.quantifier().expect("quantifier failure details");
    assert_eq!(quantifier.source(), "Q-any");
    assert_eq!(quantifier.element(), None);
    assert_eq!(quantifier.evaluated_count(), 3);
    assert!(!quantifier.holds());
}

/// FR-033-AC-1 and AC-4: universal range replay visits every passing member, then stops on its
/// first failing member and reports the original range identity.
#[trace("TC-030", "FR-033-AC-1", "FR-033-AC-4")]
#[test]
fn universal_range_replay_reports_first_failure_and_full_pass_count() {
    let query = analysis_query(vec![statement(
        "all",
        literal_range("Q-all", QuantifierKind::ForAll, 1, 3, below_x()),
    )]);
    let x = symbol_of(&query, 0);
    let passing = format!("sat\n(model (define-fun {x} () Int 3))\n");
    let conclusion =
        classify_analysis(&query, &response_record(&query, &passing, SolverEngine::Z3));
    let result = conclusion
        .verified_model()
        .expect("all range elements hold")
        .quantifier_results()
        .first()
        .expect("quantifier replay result");
    assert_eq!(result.source(), "Q-all");
    assert_eq!(result.evaluated_count(), 3);
    assert!(result.holds());

    let failing = format!("sat\n(model (define-fun {x} () Int 2))\n");
    let conclusion =
        classify_analysis(&query, &response_record(&query, &failing, SolverEngine::Z3));
    let result = conclusion
        .replay_failure()
        .expect("universal range fails")
        .quantifier()
        .expect("quantifier failure details");
    assert_eq!(result.source(), "Q-all");
    assert_eq!(result.element(), Some("3"));
    assert_eq!(result.evaluated_count(), 3);
    assert!(!result.holds());

    let cvc5_response = format!("sat\n(define-fun {x} () Int 2)\n");
    let cvc5 = response_record(&query, &cvc5_response, SolverEngine::Cvc5);
    let z3 = response_record(&query, &failing, SolverEngine::Z3);
    let differential = compare_solver_records(&query, &z3, &cvc5);
    let report = render_differential_report(&query, &differential).expect("report");
    validate_differential_report(&report, &query, &differential).expect("valid report");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
    assert!(value["engines"][0]["diagnostic"]
        .as_str()
        .expect("replay diagnostic")
        .contains("Q-all"));
    assert!(value["engines"][0]["diagnostic"]
        .as_str()
        .expect("replay diagnostic")
        .contains("element 3"));
}

/// FR-033-AC-6 and AC-9: a model satisfying a declared range replays and is verified.
#[trace("TC-030", "FR-032-AC-8", "FR-033-AC-6", "FR-033-AC-9")]
#[test]
fn bounded_quantifier_models_replay_from_declared_bounds() {
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
    let model = conclusion.verified_model().expect("verified model");
    let replay = model
        .quantifier_results()
        .first()
        .expect("quantifier replay result");
    assert_eq!(replay.source(), "Q-declared");
    assert_eq!(replay.evaluated_count(), 8);
    assert!(replay.holds());
}

/// FR-033-AC-6: every unusable declared range bound is incomplete and names that bound symbol;
/// an inverted interval is the empty domain.
#[trace("TC-030", "FR-033-AC-6")]
#[test]
fn declared_range_bound_defects_are_named_and_inverted_range_is_empty() {
    let query = analysis_query(vec![bounded_fixture()]);
    let x = query
        .variables()
        .iter()
        .find(|variable| variable.origins.iter().any(|origin| origin.contains("|x|")))
        .expect("x symbol")
        .symbol
        .clone();
    let n = query
        .variables()
        .iter()
        .find(|variable| variable.origins.iter().any(|origin| origin.contains("|n|")))
        .expect("n symbol")
        .symbol
        .clone();
    let cases = [
        (
            format!("sat\n(model (define-fun {x} () Int 7))\n"),
            ReplayFailureReason::BoundUndecodable,
        ),
        (
            format!(
                "sat\n(model (define-fun {x} () Int 7) (define-fun {n} () Int 1.5))\n"
            ),
            ReplayFailureReason::BoundUndecodable,
        ),
        (
            format!(
                "sat\n(model (define-fun {x} () Int 7) (define-fun {n} () Int 7) (define-fun {n} () Int 7))\n"
            ),
            ReplayFailureReason::BoundUndecodable,
        ),
        (
            format!("sat\n(model (define-fun {x} () Int 7) (define-fun {n} () Int 101))\n"),
            ReplayFailureReason::BoundUndecodable,
        ),
    ];
    for (response, reason) in cases {
        let conclusion = classify_analysis(
            &query,
            &response_record(&query, &response, SolverEngine::Z3),
        );
        let failure = conclusion.replay_failure().expect("bound is undecodable");
        assert_eq!(failure.reason(), reason, "{response}");
        assert_eq!(failure.subject(), Some(n.as_str()), "{response}");
    }

    let empty = statement(
        "empty",
        quantifier(
            "Q-empty",
            QuantifierKind::ForAll,
            "i",
            RangeBound::Literal(10),
            RangeBound::Declared(symbol("n")),
            term(node(ExpressionKind::BooleanLiteral { value: false })),
        ),
    );
    let query = analysis_query(vec![empty]);
    let n = query
        .variables()
        .iter()
        .find(|variable| variable.origins.iter().any(|origin| origin.contains("|n|")))
        .expect("n symbol")
        .symbol
        .clone();
    let response = format!("sat\n(model (define-fun {n} () Int 7))\n");
    let conclusion = classify_analysis(
        &query,
        &response_record(&query, &response, SolverEngine::Z3),
    );
    let result = conclusion
        .verified_model()
        .expect("inverted range is empty and universal holds")
        .quantifier_results()
        .first()
        .expect("empty range replay result");
    assert_eq!(result.source(), "Q-empty");
    assert_eq!(result.evaluated_count(), 0);
    assert!(result.holds());
}

/// FR-033-AC-2 and AC-9: quantified integer replay composes with datatype witness replay.
#[trace("TC-030", "FR-032-AC-8", "FR-033-AC-2", "FR-033-AC-9")]
#[test]
fn bounded_quantifier_over_an_enum_body_is_verified_on_first_match() {
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
    let query = analysis_query(vec![statement(
        "enum-model",
        literal_range("Q-enum", QuantifierKind::Exists, 0, 5000, enum_body),
    )]);
    assert_eq!(query.logic(), "UFDTLIA");
    let sort = &query.enum_sorts()[0];
    let admin = sort
        .constructors
        .iter()
        .find(|constructor| constructor.variant == "Admin")
        .expect("Admin")
        .symbol
        .clone();
    let definitions: Vec<_> = query
        .variables()
        .iter()
        .map(|variable| {
            if variable
                .origins
                .iter()
                .any(|origin| origin.contains("|role|"))
            {
                format!("(define-fun {} () {} {admin})", variable.symbol, sort.sort)
            } else {
                format!("(define-fun {} () Int 7)", variable.symbol)
            }
        })
        .collect();
    let model = format!("sat\n(model {})\n", definitions.join(" "));
    let conclusion = classify_analysis(&query, &response_record(&query, &model, SolverEngine::Z3));
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    let verified = conclusion.verified_model().expect("verified model");
    let replay = verified
        .quantifier_results()
        .first()
        .expect("quantifier replay result");
    assert_eq!(replay.source(), "Q-enum");
    assert_eq!(replay.element(), Some("0"));
    assert_eq!(replay.evaluated_count(), 1);
    assert!(replay.holds());

    let cvc5_model = format!("sat\n({})\n", definitions.join(" "));
    let z3 = response_record(&query, &model, SolverEngine::Z3);
    let cvc5 = response_record(&query, &cvc5_model, SolverEngine::Cvc5);
    let differential = compare_solver_records(&query, &z3, &cvc5);
    let report = render_differential_report(&query, &differential).expect("report");
    validate_differential_report(&report, &query, &differential).expect("valid report");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
    let diagnostic = value["engines"][0]["diagnostic"]
        .as_str()
        .expect("quantifier replay report");
    assert!(diagnostic.contains("Q-enum"));
    assert!(diagnostic.contains("element 0"));
}

/// FR-033-AC-1, AC-2 and AC-4: finite Boolean and enum collections replay their original member
/// order and expose the selected member with the source identity.
#[trace("TC-030", "FR-033-AC-1", "FR-033-AC-2", "FR-033-AC-4")]
#[test]
fn finite_collection_quantifiers_report_the_selected_typed_member() {
    let bool_domain = || QuantifierDomain::Collection {
        kind: CollectionKind::Sequence,
        elements: vec![DomainElement::Boolean(false), DomainElement::Boolean(true)],
    };
    let bool_body = || equals(var("flag"), local("b"));
    for (kind, model_value, holds, element, count) in [
        (QuantifierKind::ForAll, "false", false, Some("true"), 2),
        (QuantifierKind::Exists, "false", true, Some("false"), 1),
    ] {
        let formula = domain_quantifier(
            "Q-bool-collection",
            kind,
            "b",
            ValueType::Boolean,
            bool_domain(),
            bool_body(),
        );
        let query = analysis_query(vec![statement("bool-collection", formula)]);
        let flag = query
            .variables()
            .iter()
            .find(|variable| {
                variable
                    .origins
                    .iter()
                    .any(|origin| origin.contains("|flag|"))
            })
            .expect("flag symbol")
            .symbol
            .clone();
        let response = format!("sat\n(model (define-fun {flag} () Bool {model_value}))\n");
        let conclusion = classify_analysis(
            &query,
            &response_record(&query, &response, SolverEngine::Z3),
        );
        if holds {
            let result = conclusion
                .verified_model()
                .expect("quantified model verifies")
                .quantifier_results()
                .first()
                .expect("quantifier replay result");
            assert_eq!(result.source(), "Q-bool-collection");
            assert_eq!(result.element(), element);
            assert_eq!(result.evaluated_count(), count);
        } else {
            let result = conclusion
                .replay_failure()
                .expect("universal model fails replay")
                .quantifier()
                .expect("quantifier failure details");
            assert_eq!(result.source(), "Q-bool-collection");
            assert_eq!(result.element(), element);
            assert_eq!(result.evaluated_count(), count);
        }
    }

    let enum_domain = || QuantifierDomain::Enum(symbol("Role"));
    let enum_body = || term(compare(ComparisonOperator::Equal, var("role"), local("r")));
    for (kind, model_variant, holds, element, count) in [
        (QuantifierKind::ForAll, "Guest", false, Some("Admin"), 2),
        (QuantifierKind::Exists, "Admin", true, Some("Admin"), 2),
    ] {
        let formula = domain_quantifier(
            "Q-enum-domain",
            kind,
            "r",
            role_type(),
            enum_domain(),
            enum_body(),
        );
        let query = analysis_query(vec![statement("enum-domain", formula)]);
        let role = query
            .variables()
            .iter()
            .find(|variable| {
                variable
                    .origins
                    .iter()
                    .any(|origin| origin.contains("|role|"))
            })
            .expect("role symbol")
            .symbol
            .clone();
        let sort = &query.enum_sorts()[0];
        let constructor = sort
            .constructors
            .iter()
            .find(|constructor| constructor.variant == model_variant)
            .expect("enum constructor")
            .symbol
            .clone();
        let response = format!(
            "sat\n(model (define-fun {role} () {} {constructor}))\n",
            sort.sort
        );
        let conclusion = classify_analysis(
            &query,
            &response_record(&query, &response, SolverEngine::Z3),
        );
        if holds {
            let result = conclusion
                .verified_model()
                .expect("quantified model verifies")
                .quantifier_results()
                .first()
                .expect("quantifier replay result");
            assert_eq!(result.source(), "Q-enum-domain");
            assert_eq!(result.element(), element);
            assert_eq!(result.evaluated_count(), count);
        } else {
            let result = conclusion
                .replay_failure()
                .expect("universal model fails replay")
                .quantifier()
                .expect("quantifier failure details");
            assert_eq!(result.source(), "Q-enum-domain");
            assert_eq!(result.element(), element);
            assert_eq!(result.evaluated_count(), count);
        }
    }
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
    // Satisfiable bounded and expanded fixtures: both solvers agree on sat, and both returned
    // models verify only after replaying every original quantified assertion.
    let sat = [
        analysis_query(vec![bounded_fixture()]),
        analysis_query(expanded_fixture()),
    ];
    for query in &sat {
        let run = quire_analyze::execute_differential(query, &z3, &cvc5, &token).expect("run");
        assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
        assert_eq!(run.agreed_status(), Some(AnalysisStatus::Satisfied));
        for conclusion in [run.z3(), run.cvc5()] {
            assert_eq!(conclusion.solver().outcome(), SolverOutcome::Sat);
            assert_eq!(conclusion.explanation(), ExplanationState::Verified);
            assert!(conclusion.verified_model().is_some());
        }
    }
}

/// NFR-032-AC-3: each real solver under a wall-time limit too short to finish a quantified query
/// is reported `timeout` or `unknown`, never satisfied, refuted or a tool error.
#[trace("TC-030", "NFR-032-AC-3")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn real_solver_timeout_on_a_quantified_query_is_inconclusive() {
    let limits = AdapterLimits {
        wall_time_ms: 1,
        cleanup_time_ms: 1_000,
        graceful_cleanup_ms: 20,
        monitor_interval_ms: 1,
        ..AdapterLimits::default()
    };
    let query = analysis_query(vec![bounded_fixture()]);
    for (engine, variable) in [
        (SolverEngine::Z3, "QUIRE_Z3"),
        (SolverEngine::Cvc5, "QUIRE_CVC5"),
    ] {
        let path = PathBuf::from(std::env::var_os(variable).expect("solver path is required"));
        let config = SolverConfig::new(engine, path, limits).expect("config");
        let record = execute_solver(&query, &config, &CancellationToken::default());
        let status = classify_analysis(&query, &record).status();
        assert!(
            matches!(status, AnalysisStatus::Timeout | AnalysisStatus::Unknown),
            "{variable}: {status:?}"
        );
    }
}

// ---- Enum domains and finite collections (FR-031) ----

fn role_type() -> ValueType {
    ValueType::Enum {
        name: symbol("Role"),
    }
}

fn variant(name: &str) -> Expression {
    node(ExpressionKind::EnumLiteral {
        enumeration: symbol("Role"),
        variant: symbol(name),
    })
}

fn member(name: &str) -> DomainElement {
    DomainElement::Enum {
        enumeration: symbol("Role"),
        variant: symbol(name),
    }
}

fn integers(values: &[i128]) -> Vec<DomainElement> {
    values.iter().copied().map(DomainElement::Integer).collect()
}

fn domain_quantifier(
    identity: &str,
    kind: QuantifierKind,
    local_name: &str,
    local_type: ValueType,
    domain: QuantifierDomain,
    body: AnalysisFormula,
) -> AnalysisFormula {
    AnalysisFormula::Domain(Box::new(DomainQuantifier {
        identity: identity.to_owned(),
        kind,
        local: symbol(local_name),
        local_type,
        domain,
        body,
    }))
}

fn over_roles(identity: &str, kind: QuantifierKind, body: AnalysisFormula) -> AnalysisFormula {
    domain_quantifier(
        identity,
        kind,
        "r",
        role_type(),
        QuantifierDomain::Enum(symbol("Role")),
        body,
    )
}

fn over_integers(
    identity: &str,
    kind: QuantifierKind,
    collection: CollectionKind,
    values: &[i128],
    body: AnalysisFormula,
) -> AnalysisFormula {
    domain_quantifier(
        identity,
        kind,
        "i",
        ValueType::integer(element_type()),
        QuantifierDomain::Collection {
            kind: collection,
            elements: integers(values),
        },
        body,
    )
}

fn and(left: AnalysisFormula, right: AnalysisFormula) -> AnalysisFormula {
    AnalysisFormula::And(Box::new(left), Box::new(right))
}

fn implies(left: AnalysisFormula, right: AnalysisFormula) -> AnalysisFormula {
    AnalysisFormula::Implies(Box::new(left), Box::new(right))
}

fn equals(left: Expression, right: Expression) -> AnalysisFormula {
    term(compare(ComparisonOperator::Equal, left, right))
}

/// `forall r in Role. role = r => x <= 5`, which holds exactly when `x <= 5`.
fn enum_universal() -> AnalysisFormula {
    over_roles(
        "Q-roles-all",
        QuantifierKind::ForAll,
        implies(
            equals(var("role"), local("r")),
            term(compare(ComparisonOperator::LessEqual, var("x"), lit(5))),
        ),
    )
}

/// `exists r in Role. role = r and r = Guest`, which holds exactly when `role = Guest`.
fn enum_existential() -> AnalysisFormula {
    over_roles(
        "Q-roles-any",
        QuantifierKind::Exists,
        and(
            equals(var("role"), local("r")),
            equals(local("r"), variant("Guest")),
        ),
    )
}

/// `forall i in {2, 3}. i <= x`, which holds exactly when `x >= 3`.
fn collection_universal() -> AnalysisFormula {
    over_integers(
        "Q-ints-all",
        QuantifierKind::ForAll,
        CollectionKind::Set,
        &[3, 2],
        below_x(),
    )
}

/// `exists i in {2, 5}. i * 2 = x`, which holds exactly when `x` is 4 or 10.
fn collection_existential() -> AnalysisFormula {
    over_integers(
        "Q-ints-any",
        QuantifierKind::Exists,
        CollectionKind::Bag,
        &[5, 2],
        equals(
            numeric(NumericOperator::Multiply, local("i"), lit(2)),
            var("x"),
        ),
    )
}

fn x_is(value: i128) -> AnalysisFormula {
    equals(var("x"), lit(value))
}

fn role_is(name: &str) -> AnalysisFormula {
    equals(var("role"), variant(name))
}

/// One assignment of the inputs the domain fixtures read.
#[derive(Clone, Debug)]
struct Inputs {
    x: i128,
    role: &'static str,
    flag: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Boolean(bool),
    Integer(i128),
    Variant(String),
}

/// An independent evaluator of the analysis-model formula: it enumerates each domain from the
/// declarations written here and never reads the lowering.
fn holds(formula: &AnalysisFormula, inputs: &Inputs, scope: &mut Vec<(String, Value)>) -> bool {
    match formula {
        AnalysisFormula::Term(expression) => {
            evaluate(expression, inputs, scope) == Value::Boolean(true)
        }
        AnalysisFormula::Not(operand) => !holds(operand, inputs, scope),
        AnalysisFormula::And(left, right) => {
            holds(left, inputs, scope) && holds(right, inputs, scope)
        }
        AnalysisFormula::Or(left, right) => {
            holds(left, inputs, scope) || holds(right, inputs, scope)
        }
        AnalysisFormula::Implies(left, right) => {
            !holds(left, inputs, scope) || holds(right, inputs, scope)
        }
        AnalysisFormula::Range(_) => panic!("the oracle decides domain quantifiers only"),
        AnalysisFormula::Domain(quantifier) => {
            let members: Vec<Value> = match &quantifier.domain {
                QuantifierDomain::Enum(_) => ["Admin", "Guest"]
                    .into_iter()
                    .map(|name| Value::Variant(name.to_owned()))
                    .collect(),
                QuantifierDomain::Collection { elements, .. } => elements
                    .iter()
                    .map(|element| match element {
                        DomainElement::Boolean(value) => Value::Boolean(*value),
                        DomainElement::Integer(value) => Value::Integer(*value),
                        DomainElement::Enum { variant, .. } => {
                            Value::Variant(variant.as_str().to_owned())
                        }
                    })
                    .collect(),
                QuantifierDomain::Unbounded(_) => panic!("an unbounded domain has no verdict"),
            };
            let mut results = members.into_iter().map(|value| {
                scope.push((quantifier.local.as_str().to_owned(), value));
                let result = holds(&quantifier.body, inputs, scope);
                scope.pop();
                result
            });
            if quantifier.kind == QuantifierKind::ForAll {
                results.all(|result| result)
            } else {
                results.any(|result| result)
            }
        }
    }
}

fn evaluate(expression: &Expression, inputs: &Inputs, scope: &[(String, Value)]) -> Value {
    match expression.kind() {
        ExpressionKind::IntegerLiteral { value, .. } => Value::Integer(*value),
        ExpressionKind::EnumLiteral { variant, .. } => Value::Variant(variant.as_str().to_owned()),
        ExpressionKind::ValueReference { name, .. } => match name.as_str() {
            "x" => Value::Integer(inputs.x),
            "role" => Value::Variant(inputs.role.to_owned()),
            "flag" => Value::Boolean(inputs.flag),
            other => panic!("the oracle reads x, role and flag, not {other}"),
        },
        ExpressionKind::LocalReference { name } => scope
            .iter()
            .rev()
            .find(|(bound, _)| bound == name.as_str())
            .expect("bound")
            .1
            .clone(),
        ExpressionKind::Numeric { left, right, .. } => {
            let (Value::Integer(left), Value::Integer(right)) = (
                evaluate(left, inputs, scope),
                evaluate(right, inputs, scope),
            ) else {
                panic!("integer operands");
            };
            Value::Integer(left * right)
        }
        ExpressionKind::Compare {
            operator,
            left,
            right,
        } => {
            let (left, right) = (
                evaluate(left, inputs, scope),
                evaluate(right, inputs, scope),
            );
            Value::Boolean(match operator {
                ComparisonOperator::Equal => left == right,
                ComparisonOperator::LessEqual => match (left, right) {
                    (Value::Integer(left), Value::Integer(right)) => left <= right,
                    _ => panic!("integer operands"),
                },
                _ => panic!("the oracle decides = and <= only"),
            })
        }
        _ => panic!("construct outside the oracle"),
    }
}

/// Every assignment the fixtures can distinguish.
fn assignments() -> Vec<Inputs> {
    let mut all = Vec::new();
    for x in -2..=11 {
        for role in ["Admin", "Guest"] {
            for flag in [false, true] {
                all.push(Inputs { x, role, flag });
            }
        }
    }
    all
}

/// The solver model that assigns `inputs`, in the lowered query's own symbols.
fn model_for(query: &QueryBundle, inputs: &Inputs) -> String {
    let definitions: Vec<_> = query
        .variables()
        .iter()
        .map(|variable| {
            let origin = |name: &str| {
                variable
                    .origins
                    .iter()
                    .any(|origin| origin.contains(&format!("|{name}|")))
            };
            if origin("role") {
                let sort = &query.enum_sorts()[0];
                let constructor = sort
                    .constructors
                    .iter()
                    .find(|constructor| constructor.variant == inputs.role)
                    .expect("constructor");
                format!(
                    "(define-fun {} () {} {})",
                    variable.symbol, sort.sort, constructor.symbol
                )
            } else if origin("flag") {
                format!("(define-fun {} () Bool {})", variable.symbol, inputs.flag)
            } else if origin("x") {
                let value = if inputs.x < 0 {
                    format!("(- {})", inputs.x.unsigned_abs())
                } else {
                    inputs.x.to_string()
                };
                format!("(define-fun {} () Int {value})", variable.symbol)
            } else {
                panic!("unexpected variable {:?}", variable.origins)
            }
        })
        .collect();
    format!("sat\n(model {})\n", definitions.join(" "))
}

/// The fixtures with a hand-derived expected status: the formula, and whether it is satisfiable.
fn domain_fixtures() -> Vec<(&'static str, AnalysisFormula, bool)> {
    vec![
        ("enum-forall-sat", and(enum_universal(), x_is(3)), true),
        ("enum-forall-unsat", and(enum_universal(), x_is(9)), false),
        ("enum-exists-sat", enum_existential(), true),
        (
            "enum-exists-unsat",
            and(enum_existential(), role_is("Admin")),
            false,
        ),
        (
            "collection-forall-sat",
            and(collection_universal(), x_is(3)),
            true,
        ),
        (
            "collection-forall-unsat",
            and(collection_universal(), x_is(2)),
            false,
        ),
        (
            "collection-exists-sat",
            and(collection_existential(), x_is(10)),
            true,
        ),
        (
            "collection-exists-unsat",
            and(collection_existential(), x_is(7)),
            false,
        ),
    ]
}

/// FR-031-AC-1 and FR-031-AC-2: the enumeration oracle gives each fixture its recorded status,
/// and the lowered query agrees with the oracle on every assignment, because a model verifies
/// against the query's replay exactly when the oracle holds.
#[trace("TC-030", "FR-031-AC-1", "FR-031-AC-2", "FR-030-AC-1")]
#[test]
fn enumeration_oracle_decides_domain_fixtures_and_agrees_with_the_lowered_query() {
    for (name, formula, expected) in domain_fixtures() {
        let satisfiable = assignments()
            .iter()
            .any(|inputs| holds(&formula, inputs, &mut Vec::new()));
        assert_eq!(satisfiable, expected, "{name}: recorded status");
        let query = analysis_query(vec![statement(name, formula.clone())]);
        for inputs in assignments() {
            let model = model_for(&query, &inputs);
            let conclusion =
                classify_analysis(&query, &response_record(&query, &model, SolverEngine::Z3));
            assert_eq!(
                conclusion.verified_model().is_some(),
                holds(&formula, &inputs, &mut Vec::new()),
                "{name} at {inputs:?}"
            );
        }
    }
}

/// FR-031-AC-1 and FR-031-AC-4: an enum universal and existential fixture lower to their golden
/// query under the datatype quantifier-free logic. The golden bytes are fixed, so every run is
/// compared with the same bytes in its own process.
#[trace("TC-030", "FR-031-AC-1", "FR-031-AC-4", "FR-030-AC-1")]
#[test]
fn enum_domains_lower_to_the_golden_query() {
    let build = || {
        lower(&[
            statement("all", enum_universal()),
            statement("any", enum_existential()),
        ])
    };
    let query = build();
    assert_eq!(query.logic(), "QF_UFDTLIA");
    assert_eq!(query.query(), include_str!("golden/enum-domain-v2.smt2"));
    assert_eq!(query.query(), build().query());
    let entries = query.quantifiers();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].source, "Q-roles-all");
    assert_eq!(entries[0].domain(), QuantifierDomainClass::Enum);
    assert_eq!(entries[0].encoding, QuantifierEncoding::Expanded);
    assert_eq!(entries[0].instance_count, 2);
    assert_eq!(
        entries[0].domain_data,
        QuantifierEntryDomain::Enum {
            elements: vec![member("Guest"), member("Admin")],
        }
    );
    assert_eq!(entries[1].kind, QuantifierKind::Exists);
}

/// FR-031-AC-2 and FR-031-AC-4: a universal and an existential integer collection lower to their
/// golden query.
#[trace("TC-030", "FR-031-AC-2", "FR-031-AC-4", "FR-030-AC-1")]
#[test]
fn collection_domains_lower_to_the_golden_query() {
    let build = || {
        lower(&[
            statement("all", collection_universal()),
            statement("any", collection_existential()),
        ])
    };
    let query = build();
    assert_eq!(query.logic(), "QF_LIA");
    assert_eq!(
        query.query(),
        include_str!("golden/collection-domain-v2.smt2")
    );
    assert_eq!(query.query(), build().query());
    let mut reversed = vec![
        statement("all", collection_universal()),
        statement("any", collection_existential()),
    ];
    reversed.reverse();
    assert_eq!(lower(&reversed).query(), query.query());
    assert!(query
        .quantifiers()
        .iter()
        .all(|entry| entry.domain() == QuantifierDomainClass::Collection));
}

/// FR-031-AC-3: a domain fixture's query has no solver quantifier token and one body instance
/// per member, in domain order.
#[trace("TC-030", "FR-031-AC-3")]
#[test]
fn expansion_has_no_quantifier_token_and_one_instance_per_member_in_order() {
    let universal = lower_one("all", collection_universal());
    let text = universal.query();
    assert!(!text.contains("forall") && !text.contains("exists"));
    // The set {3, 2} is enumerated 2 then 3.
    let two = text.find("(<= 2 ").expect("instance for 2");
    let three = text.find("(<= 3 ").expect("instance for 3");
    assert!(two < three);
    assert_eq!(count(text, "(<= 2 "), 1);
    assert_eq!(count(text, "(<= 3 "), 1);

    let roles = lower_one("roles", enum_universal());
    assert!(!roles.query().contains("forall"));
    assert_eq!(count(roles.query(), "(=> "), 2);
}

/// The query of a Boolean collection quantifier over a `flag = b` body.
fn boolean_query(collection: CollectionKind, values: &[bool]) -> String {
    let formula = domain_quantifier(
        "Q-bool",
        QuantifierKind::Exists,
        "b",
        ValueType::Boolean,
        QuantifierDomain::Collection {
            kind: collection,
            elements: values.iter().copied().map(DomainElement::Boolean).collect(),
        },
        equals(var("flag"), local("b")),
    );
    lower_one("bool", formula).query().to_owned()
}

fn integer_query(collection: CollectionKind, values: &[i128]) -> String {
    lower_one(
        "ints",
        over_integers(
            "Q-ints",
            QuantifierKind::ForAll,
            collection,
            values,
            below_x(),
        ),
    )
    .query()
    .to_owned()
}

fn role_query(collection: CollectionKind, names: &[&str]) -> String {
    let formula = domain_quantifier(
        "Q-role-set",
        QuantifierKind::ForAll,
        "r",
        role_type(),
        QuantifierDomain::Collection {
            kind: collection,
            elements: names.iter().map(|name| member(name)).collect(),
        },
        equals(var("role"), local("r")),
    );
    lower_one("roles", formula).query().to_owned()
}

/// The asserted term of a query's last named assertion, without its generated name.
fn body_of(query: &str) -> &str {
    let start = query.rfind("(assert (! ").expect("an assertion") + "(assert (! ".len();
    let end = query[start..].find(" :named").expect("a name") + start;
    &query[start..end]
}

/// FR-031-AC-2: a `set` and an `ordered_set` collapse a repeat to its first occurrence, a `bag`
/// and a `sequence` keep every occurrence, a `set` and a `bag` enumerate in ascending canonical
/// order (false before true, declaration order, ascending integer), and an `ordered_set` and a
/// `sequence` in written order.
#[trace("TC-030", "FR-031-AC-2")]
#[test]
fn collections_enumerate_in_canonical_or_written_order_with_the_duplicate_rule() {
    use CollectionKind::{Bag, OrderedSet, Sequence, Set};
    // set: a repeat adds nothing, order is canonical whatever the written order.
    assert_eq!(integer_query(Set, &[3, 1, 3]), integer_query(Set, &[1, 3]));
    assert_eq!(integer_query(Set, &[3, 1]), integer_query(Set, &[1, 3]));
    assert_eq!(
        boolean_query(Set, &[true, false, true]),
        boolean_query(Set, &[false, true])
    );
    assert_eq!(
        body_of(&boolean_query(Set, &[true, false])),
        body_of(&boolean_query(Sequence, &[false, true]))
    );
    // A bag keeps repeated occurrences but orders the Boolean values false before true.
    assert_eq!(
        body_of(&boolean_query(Bag, &[true, false, true])),
        body_of(&boolean_query(Sequence, &[false, true, true]))
    );
    assert_eq!(
        role_query(Set, &["Guest", "Admin"]),
        role_query(Set, &["Admin", "Guest"])
    );
    assert_eq!(
        body_of(&role_query(Set, &["Admin", "Guest"])),
        body_of(&role_query(Sequence, &["Guest", "Admin"]))
    );
    // bag: every occurrence is an instance, in canonical order.
    assert_eq!(
        body_of(&integer_query(Bag, &[3, 1, 3])),
        body_of(&integer_query(Sequence, &[1, 3, 3]))
    );
    // Role declares Guest before Admin. The bag's order must follow declaration order rather
    // than variant-name order.
    assert_eq!(
        body_of(&role_query(Bag, &["Admin", "Guest"])),
        body_of(&role_query(Sequence, &["Guest", "Admin"]))
    );
    assert_ne!(integer_query(Bag, &[3, 1, 3]), integer_query(Bag, &[1, 3]));
    // sequence: every occurrence, as written.
    assert_ne!(
        integer_query(Sequence, &[3, 1, 3]),
        integer_query(Sequence, &[1, 3])
    );
    let sequence = integer_query(Sequence, &[3, 1, 3]);
    assert_eq!(count(&sequence, "(<= 3 "), 2);
    assert!(sequence.find("(<= 3 ") < sequence.find("(<= 1 "));
    // ordered_set: written order, a repeat collapsing to its first occurrence.
    let ordered = integer_query(OrderedSet, &[3, 1, 3]);
    assert_eq!(ordered, integer_query(OrderedSet, &[3, 1]));
    assert_eq!(count(&ordered, "(<= 3 "), 1);
    assert!(ordered.find("(<= 3 ") < ordered.find("(<= 1 "));
    assert_ne!(body_of(&ordered), body_of(&integer_query(Set, &[3, 1])));
    assert_ne!(
        body_of(&role_query(OrderedSet, &["Admin", "Guest"])),
        body_of(&role_query(Set, &["Admin", "Guest"]))
    );
}

/// FR-031-AC-5: the quantifier map retains the source identity and typed members in instance order.
#[trace("TC-030", "FR-031-AC-5")]
#[test]
fn quantifier_map_retains_typed_members_in_instance_order() {
    let formula = over_integers(
        "Q-source",
        QuantifierKind::ForAll,
        CollectionKind::Sequence,
        &[2, 7, 4],
        below_x(),
    );
    let query = lower_one("failing", formula.clone());
    let entry = &query.quantifiers()[0];
    assert_eq!(entry.source, "Q-source");
    assert_eq!(
        entry.domain_data,
        QuantifierEntryDomain::Collection {
            elements: integers(&[2, 7, 4]),
        }
    );
}

/// FR-031-AC-6: an inner quantifier that reuses the bound variable's name shadows it, and a free
/// input named like a bound variable is not captured: both lower to distinct symbols, matching
/// the golden query.
#[trace("TC-030", "FR-031-AC-6")]
#[test]
fn domain_expansion_is_capture_avoiding() {
    let against_input = over_integers(
        "Q-free",
        QuantifierKind::ForAll,
        CollectionKind::Sequence,
        &[1, 2],
        term(compare(ComparisonOperator::LessEqual, local("i"), var("i"))),
    );
    // forall i in {1, 2}. exists i in {5, 6}. i <= x : the inner i is 5 or 6.
    let shadowed = over_integers(
        "Q-outer",
        QuantifierKind::ForAll,
        CollectionKind::Sequence,
        &[1, 2],
        over_integers(
            "Q-inner",
            QuantifierKind::Exists,
            CollectionKind::Sequence,
            &[5, 6],
            below_x(),
        ),
    );
    let query = lower(&[
        statement("free", against_input),
        statement("shadow", shadowed),
    ]);
    assert_eq!(query.query(), include_str!("golden/domain-capture-v2.smt2"));
    let input_symbol = query
        .variables()
        .iter()
        .find(|variable| variable.origins.iter().any(|origin| origin.contains("|i|")));
    let input_symbol = &input_symbol.expect("input i").symbol;
    assert!(query.query().contains(&format!("(<= 1 {input_symbol})")));
    assert!(query.query().contains(&format!("(<= 2 {input_symbol})")));
    assert!(!query.query().contains("(<= 5 5)"));
}

/// FR-030-AC-4: a universal quantifier over an empty collection is true and an existential one
/// is false, with zero instances.
#[trace("TC-030", "FR-030-AC-4")]
#[test]
fn an_empty_domain_is_true_for_universal_and_false_for_existential() {
    let all = analysis_query(vec![statement(
        "empty-all",
        over_integers(
            "Q-all",
            QuantifierKind::ForAll,
            CollectionKind::Set,
            &[],
            below_x(),
        ),
    )]);
    assert!(all.query().contains("(assert (! true :named"));
    assert_eq!(encodings(&all), [(QuantifierEncoding::Expanded, 0)]);
    let inputs = Inputs {
        x: 5,
        role: "Guest",
        flag: false,
    };
    let model = model_for(&all, &inputs);
    let all_record = response_record(&all, &model, SolverEngine::Z3);
    assert_eq!(
        all_record.outcome(),
        SolverOutcome::Sat,
        "stdout={} stderr={} diagnostic={:?}",
        String::from_utf8_lossy(all_record.stdout()),
        String::from_utf8_lossy(all_record.stderr()),
        all_record.diagnostic()
    );
    let all_conclusion = classify_analysis(&all, &all_record);
    assert_eq!(all_conclusion.status(), AnalysisStatus::Satisfied);
    assert!(all_conclusion.verified_model().is_some());
    let all_cvc5 = response_record(&all, &model, SolverEngine::Cvc5);
    let all_run = compare_solver_records(&all, &all_record, &all_cvc5);
    assert_eq!(all_run.agreed_status(), Some(AnalysisStatus::Satisfied));
    let any = analysis_query(vec![statement(
        "empty-any",
        over_integers(
            "Q-any",
            QuantifierKind::Exists,
            CollectionKind::Bag,
            &[],
            below_x(),
        ),
    )]);
    assert!(any.query().contains("(assert (! false :named"));
    assert_eq!(any.logic(), "QF_LIA");
    let any_z3 = response_record(&any, "unsat\n", SolverEngine::Z3);
    let any_cvc5 = response_record(&any, "unsat\n", SolverEngine::Cvc5);
    let any_run = compare_solver_records(&any, &any_z3, &any_cvc5);
    assert_eq!(any_run.agreed_status(), Some(AnalysisStatus::Refuted));
}

/// FR-030-AC-2 and ticket acceptance: a domain with no finite exact members is refused as
/// unsupported with its reason class and the quantifier's source identity, and no query exists.
#[trace("TC-030", "FR-030-AC-2")]
#[test]
fn unbounded_text_and_other_domains_are_unsupported() {
    let cases = [
        (UnboundedDomain::Integer, "unbounded-domain"),
        (UnboundedDomain::Text, "text-domain"),
        (UnboundedDomain::Rational, "rational-domain"),
        (UnboundedDomain::UnfixedCollection, "unfixed-collection"),
        (UnboundedDomain::UnencodableElement, "unencodable-element"),
    ];
    for (class, reason) in cases {
        let errors = refusal(
            "refused",
            domain_quantifier(
                "Q-refused",
                QuantifierKind::ForAll,
                "i",
                ValueType::integer(element_type()),
                QuantifierDomain::Unbounded(class),
                below_x(),
            ),
        );
        assert_eq!(errors.len(), 1, "{reason}");
        assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedConstruct);
        let refused = errors[0].quantifier_refusal().expect("refusal");
        assert_eq!(refused.identity, "Q-refused");
        assert_eq!(refused.reason.as_str(), reason);
    }
}

/// FR-030-AC-3: a refused domain nested inside admitted ones refuses the whole request.
#[trace("TC-030", "FR-030-AC-3")]
#[test]
fn a_refused_domain_inside_admitted_quantifiers_refuses_the_whole_request() {
    let nested = over_roles(
        "Q-outer",
        QuantifierKind::ForAll,
        literal_range(
            "Q-range",
            QuantifierKind::Exists,
            1,
            3,
            domain_quantifier(
                "Q-text",
                QuantifierKind::ForAll,
                "t",
                ValueType::Boolean,
                QuantifierDomain::Unbounded(UnboundedDomain::Text),
                below_x(),
            ),
        ),
    );
    let errors = refusal("nested", nested);
    assert_eq!(
        errors[0].quantifier_refusal().expect("refusal").identity,
        "Q-text"
    );
}

/// NFR-030-AC-1 and FR-030-AC-5: a collection of exactly 1,024 members expands and one of 1,025
/// is refused as `expansion-ceiling`; the budget is shared with ranges and nested counts
/// multiply.
#[trace("TC-030", "NFR-030-AC-1", "FR-030-AC-5")]
#[test]
fn collection_expansion_meets_the_budget_exactly_and_refuses_one_above() {
    let members = |count: i128| (1..=count).collect::<Vec<_>>();
    let single = |count: i128| {
        over_integers(
            "Q-big",
            QuantifierKind::ForAll,
            CollectionKind::Sequence,
            &members(count),
            below_x(),
        )
    };
    assert_eq!(
        encodings(&lower_one("fits", single(1024))),
        [(QuantifierEncoding::Expanded, 1024)]
    );
    let errors = refusal("over", single(1025));
    let refused = errors[0].quantifier_refusal().expect("refusal");
    assert_eq!(refused.identity, "Q-big");
    assert_eq!(refused.reason, QuantifierReason::ExpansionCeiling);
    assert_eq!(refused.reason.as_str(), "expansion-ceiling");

    // The range before it in pre-order spends 1,000; a collection of 24 fits and 25 does not.
    let shared = |second: i128| {
        and(
            literal_range("Q-range", QuantifierKind::ForAll, 1, 1000, below_x()),
            over_integers(
                "Q-coll",
                QuantifierKind::ForAll,
                CollectionKind::Sequence,
                &members(second),
                below_x(),
            ),
        )
    };
    lower_one("shared-fits", shared(24));
    assert_eq!(
        refusal("shared-over", shared(25))[0]
            .quantifier_refusal()
            .expect("refusal")
            .reason,
        QuantifierReason::ExpansionCeiling
    );

    // A range of 600 expands; an enum domain of 2 inside it would need 1,200 and is refused,
    // where a range there would have been emitted as a bounded quantifier.
    let nested = literal_range(
        "Q-outer",
        QuantifierKind::ForAll,
        1,
        600,
        over_roles(
            "Q-inner",
            QuantifierKind::ForAll,
            equals(var("role"), local("r")),
        ),
    );
    let errors = refusal("nested-enum", nested);
    assert_eq!(
        errors[0].quantifier_refusal().expect("refusal").identity,
        "Q-inner"
    );
}

/// NFR-030-AC-3: a range too wide to expand is bounded and consumes no budget, so a domain
/// inside it counts from its own width, with no overflow.
#[trace("TC-030", "NFR-030-AC-3")]
#[test]
fn a_domain_inside_a_bounded_range_counts_from_its_own_width() {
    let query = lower_one(
        "bounded-outer",
        literal_range(
            "Q-outer",
            QuantifierKind::ForAll,
            i128::MIN,
            i128::MAX,
            over_roles(
                "Q-roles",
                QuantifierKind::ForAll,
                equals(var("role"), local("r")),
            ),
        ),
    );
    assert_eq!(
        encodings(&query),
        [
            (QuantifierEncoding::Bounded, 0),
            (QuantifierEncoding::Expanded, 2)
        ]
    );
    assert_eq!(query.logic(), "UFDTLIA");
}

/// FR-030-AC-6: a collection's element sort must be encodable even when its literal is empty.
#[trace("TC-030", "FR-030-AC-6")]
#[test]
fn empty_collection_domains_with_unencodable_sorts_are_refused_when_attached() {
    let invalid_empty_sorts = [
        ValueType::Text,
        ValueType::Rational {
            value: quire_contract_model::RationalType::new(-1, 1, 2).expect("rational type"),
        },
        ValueType::Record {
            name: symbol("Record"),
        },
        ValueType::option(ValueType::Boolean),
        ValueType::collection(
            quire_contract_model::CollectionType::new(ValueType::Boolean, 4)
                .expect("collection type"),
        ),
    ];
    for local_type in invalid_empty_sorts {
        let formula = domain_quantifier(
            "Q-empty-invalid",
            QuantifierKind::ForAll,
            "value",
            local_type,
            QuantifierDomain::Collection {
                kind: CollectionKind::Set,
                elements: vec![],
            },
            term(node(ExpressionKind::BooleanLiteral { value: true })),
        );
        let errors = statement_without_formula()
            .with_formula(formula)
            .expect_err("unencodable empty domain refused");
        assert!(errors
            .iter()
            .all(|error| error.code() == LoweringErrorCode::InvalidStatement));
    }
}

/// FR-030-AC-6: a domain's local type, enum declaration and every member are validated when
/// a domain formula is attached.
#[trace("TC-030", "FR-030-AC-6")]
#[test]
fn an_invalid_domain_formula_is_refused_when_attached() {
    let invalid = [
        domain_quantifier(
            "Q",
            QuantifierKind::ForAll,
            "r",
            ValueType::Boolean,
            QuantifierDomain::Enum(symbol("Role")),
            term(compare(ComparisonOperator::Equal, local("r"), local("r"))),
        ),
        domain_quantifier(
            "Q",
            QuantifierKind::ForAll,
            "r",
            ValueType::Enum {
                name: symbol("Missing"),
            },
            QuantifierDomain::Enum(symbol("Missing")),
            term(compare(ComparisonOperator::Equal, local("r"), local("r"))),
        ),
        domain_quantifier(
            "Q",
            QuantifierKind::ForAll,
            "i",
            ValueType::Boolean,
            QuantifierDomain::Collection {
                kind: CollectionKind::Set,
                elements: integers(&[1]),
            },
            term(compare(ComparisonOperator::Equal, local("i"), local("i"))),
        ),
        domain_quantifier(
            "Q",
            QuantifierKind::ForAll,
            "r",
            role_type(),
            QuantifierDomain::Collection {
                kind: CollectionKind::Set,
                elements: vec![member("Nobody")],
            },
            term(compare(ComparisonOperator::Equal, local("r"), local("r"))),
        ),
        domain_quantifier(
            "",
            QuantifierKind::ForAll,
            "r",
            role_type(),
            QuantifierDomain::Enum(symbol("Role")),
            term(compare(ComparisonOperator::Equal, local("r"), local("r"))),
        ),
    ];
    for formula in invalid {
        let errors = statement_without_formula()
            .with_formula(formula)
            .expect_err("refused");
        assert!(errors
            .iter()
            .all(|error| error.code() == LoweringErrorCode::InvalidStatement));
    }
}

fn statement_without_formula() -> StatementInput {
    // The formula the helper attaches is valid and is replaced by the caller's.
    statement(
        "plain",
        term(node(ExpressionKind::BooleanLiteral { value: true })),
    )
}

/// Capability contract: enum and collection quantifiers are exact constructs.
#[trace("TC-030")]
#[test]
fn capability_contract_lists_the_domain_quantifiers() {
    for construct in ["enum_quantifier", "collection_quantifier"] {
        assert!(CAPABILITY_CONTRACT.exact_constructs.contains(&construct));
    }
}

/// FR-031-AC-1 (controlled responses): an `unsat` answer on an expanded domain query agrees
/// between the engines and is a refutation.
#[trace("TC-030", "FR-031-AC-1")]
#[test]
fn an_unsat_response_on_a_domain_fixture_is_a_refutation() {
    let query = analysis_query(vec![statement("unsat", and(enum_universal(), x_is(9)))]);
    let z3 = response_record(&query, "unsat\n", SolverEngine::Z3);
    let cvc5 = response_record(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Refuted));
}

/// FR-031-AC-1 and FR-031-AC-2: real solvers decide each domain fixture as its recorded status;
/// run only where installed.
#[trace("TC-030", "FR-031-AC-1", "FR-031-AC-2")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn real_z3_cvc5_decide_domain_fixtures() {
    let z3_path = PathBuf::from(std::env::var_os("QUIRE_Z3").expect("QUIRE_Z3 is required"));
    let cvc5_path = PathBuf::from(std::env::var_os("QUIRE_CVC5").expect("QUIRE_CVC5 is required"));
    let z3 = SolverConfig::new(SolverEngine::Z3, z3_path, AdapterLimits::default()).expect("z3");
    let cvc5 =
        SolverConfig::new(SolverEngine::Cvc5, cvc5_path, AdapterLimits::default()).expect("cvc5");
    let token = CancellationToken::default();
    for (name, formula, expected) in domain_fixtures() {
        let query = analysis_query(vec![statement(name, formula)]);
        for config in [&z3, &cvc5] {
            let record = execute_solver(&query, config, &token);
            let want = if expected {
                SolverOutcome::Sat
            } else {
                SolverOutcome::Unsat
            };
            assert_eq!(record.outcome(), want, "{name}\n{}", query.query());
        }
        let run = quire_analyze::execute_differential(&query, &z3, &cvc5, &token).expect("run");
        assert_eq!(
            run.disposition(),
            DifferentialDisposition::Agreement,
            "{name}"
        );
        if expected {
            for conclusion in [run.z3(), run.cvc5()] {
                assert_eq!(
                    conclusion.explanation(),
                    ExplanationState::Verified,
                    "{name}"
                );
                assert!(conclusion.verified_model().is_some(), "{name}");
            }
        }
    }
}
