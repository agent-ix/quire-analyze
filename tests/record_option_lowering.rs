//! Requirement-tagged tests for record and option datatype lowering (FR-020 record and option
//! parts, NFR-020, TC-020).

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
    DifferentialDisposition, ExplanationState, LoweringError, LoweringErrorCode, QueryBundle,
    ReplayFailureReason, SolverConfig, SolverEngine, SolverOutcome, StatementInput,
    CAPABILITY_CONTRACT,
};
use quire_contract_model::{
    AnchorName, BooleanOperator, Clause, ClauseId, ClauseKind, CollectionType, ComparisonOperator,
    ContractPackage, DeclarationEnvironment, EnumDeclaration, EnumVariantDeclaration,
    ExecutionPoint, Expression, ExpressionKind, IntegerDomain, IntegerType, OverflowPolicy,
    PackageId, RecordDeclaration, RecordFieldDeclaration, RecordLiteralField, Requirement,
    RequirementId, RequirementRef, RequirementRevision, SchemaVersion, SourceDocumentId,
    SourceIdentity, SourceLocation, SourceRevision, SourceSpan, StateObservation, SymbolName,
    TypeDeclaration, TypedExpression, ValueDeclaration, ValueDeclarationKind, ValueType,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn span() -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("record-test").expect("source id"),
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

fn var(name: &str) -> Expression {
    node(ExpressionKind::ValueReference {
        name: symbol(name),
        observation: StateObservation::Current,
    })
}

fn compare(operator: ComparisonOperator, left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Compare {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn boolean(operator: BooleanOperator, left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Boolean {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn variant(enumeration: &str, name: &str) -> Expression {
    node(ExpressionKind::EnumLiteral {
        enumeration: symbol(enumeration),
        variant: symbol(name),
    })
}

fn limit_type() -> IntegerType {
    IntegerType::new(IntegerDomain::Signed, 0, 100, OverflowPolicy::Reject).expect("integer type")
}

fn number(value: i128) -> Expression {
    node(ExpressionKind::IntegerLiteral {
        value,
        value_type: limit_type(),
    })
}

fn field(base: Expression, name: &str) -> Expression {
    node(ExpressionKind::FieldAccess {
        base: Box::new(base),
        field: symbol(name),
    })
}

fn record(name: &str, fields: Vec<(&str, Expression)>) -> Expression {
    node(ExpressionKind::RecordLiteral {
        record: symbol(name),
        fields: fields
            .into_iter()
            .map(|(field, value)| RecordLiteralField::new(symbol(field), value))
            .collect(),
    })
}

fn none(option: ValueType) -> Expression {
    node(ExpressionKind::OptionNone { value_type: option })
}

fn some(option: ValueType, value: Expression) -> Expression {
    node(ExpressionKind::OptionSome {
        value_type: option,
        value: Box::new(value),
    })
}

fn present(option: Expression) -> Expression {
    node(ExpressionKind::IsPresent {
        option: Box::new(option),
    })
}

fn unwrap(option: Expression) -> Expression {
    node(ExpressionKind::Unwrap {
        option: Box::new(option),
    })
}

fn implies(antecedent: Expression, consequent: Expression) -> Expression {
    boolean(BooleanOperator::Implication, antecedent, consequent)
}

fn enum_type(name: &str, variants: &[&str]) -> TypeDeclaration {
    let variants = variants
        .iter()
        .map(|variant| EnumVariantDeclaration::new(symbol(variant), span()))
        .collect();
    TypeDeclaration::Enum {
        declaration: EnumDeclaration::new(symbol(name), span(), variants).expect("enum"),
    }
}

fn record_type(name: &str, fields: &[(&str, ValueType)]) -> TypeDeclaration {
    let fields = fields
        .iter()
        .map(|(field, value_type)| {
            RecordFieldDeclaration::new(symbol(field), value_type.clone(), span())
        })
        .collect();
    TypeDeclaration::Record {
        declaration: RecordDeclaration::new(symbol(name), span(), fields).expect("record"),
    }
}

fn record_ref(name: &str) -> ValueType {
    ValueType::Record { name: symbol(name) }
}

fn enum_ref(name: &str) -> ValueType {
    ValueType::Enum { name: symbol(name) }
}

fn integer() -> ValueType {
    ValueType::integer(limit_type())
}

fn input(name: &str, value_type: ValueType) -> ValueDeclaration {
    ValueDeclaration::new(
        symbol(name),
        ValueDeclarationKind::Input,
        value_type,
        span(),
    )
}

const ROLES: &[&str] = &["Admin", "Auditor", "Guest", "User"];

fn environment(
    label: &str,
    types: Vec<TypeDeclaration>,
    values: Vec<ValueDeclaration>,
) -> Result<DeclarationEnvironment, String> {
    let owner = RequirementRef::new(
        PackageId::new("agent-ix/record-test").expect("package"),
        RequirementId::new(format!("REQ-{label}")).expect("requirement"),
        RequirementRevision::new(1).expect("revision"),
    );
    DeclarationEnvironment::new(owner, types, values, vec![])
        .map_err(|diagnostics| format!("{diagnostics:?}"))
}

/// One requirement whose clauses are `expressions`, lowered against one shared environment.
fn try_statements(
    label: &str,
    types: Vec<TypeDeclaration>,
    values: Vec<ValueDeclaration>,
    expressions: Vec<Expression>,
) -> Result<Vec<StatementInput>, String> {
    let package_id = PackageId::new("agent-ix/record-test").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-{label}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let environment = environment(label, types, values)?;
    let mut clauses = Vec::new();
    for (index, expression) in expressions.iter().enumerate() {
        let checked = environment
            .check_expression(expression, &ValueType::Boolean, &point(), true)
            .map_err(|diagnostics| format!("{diagnostics:?}"))?;
        clauses.push(
            Clause::new(
                ClauseId::new(format!("C-{label}-{index}")).expect("clause"),
                ClauseKind::Assertion,
                Some(point()),
                span(),
                checked,
            )
            .expect("clause"),
        );
    }
    let requirement =
        Requirement::<TypedExpression>::new(&package_id, requirement_id, revision, span(), clauses)
            .expect("requirement");
    let package = ContractPackage::new(
        package_id,
        SchemaVersion::V1_1,
        span().source().clone(),
        vec![requirement],
    )
    .expect("package");
    let requirement = &package.requirements()[0];
    requirement
        .clauses()
        .iter()
        .map(|clause| {
            StatementInput::from_clause(&package, requirement, clause, environment.clone())
        })
        .collect::<Result<Vec<_>, Vec<LoweringError>>>()
        .map_err(|errors| format!("{errors:?}"))
}

fn statements(
    label: &str,
    types: Vec<TypeDeclaration>,
    values: Vec<ValueDeclaration>,
    expressions: Vec<Expression>,
) -> Vec<StatementInput> {
    try_statements(label, types, values, expressions).expect("statements")
}

/// The debug text of the diagnostics `check_expression` refuses an expression with.
fn check_refusal(
    types: Vec<TypeDeclaration>,
    values: Vec<ValueDeclaration>,
    expression: &Expression,
) -> String {
    let environment = environment("check", types, values).expect("env");
    let diagnostics = environment
        .check_expression(expression, &ValueType::Boolean, &point(), true)
        .expect_err("refused");
    format!("{diagnostics:?}")
}

fn lower(statements: Vec<StatementInput>) -> QueryBundle {
    let request = AnalysisRequest::consistency(vec![], statements, vec![]).expect("request");
    lower_analysis_request(&request).expect("lower")
}

fn lower_error(statements: Vec<StatementInput>) -> Vec<LoweringError> {
    lower_boolean_statements(&statements, &[]).expect_err("refused")
}

fn account_types() -> Vec<TypeDeclaration> {
    vec![
        enum_type("Role", ROLES),
        record_type(
            "Account",
            &[
                ("role", enum_ref("Role")),
                ("active", ValueType::Boolean),
                ("level", integer()),
            ],
        ),
    ]
}

fn account_values() -> Vec<ValueDeclaration> {
    vec![input("account", record_ref("Account"))]
}

/// A record-field constraint over a Boolean, an enum and an integer field.
fn record_fixture(label: &str, further: Expression) -> Vec<StatementInput> {
    statements(
        label,
        account_types(),
        account_values(),
        vec![
            compare(
                ComparisonOperator::Equal,
                field(var("account"), "role"),
                variant("Role", "Admin"),
            ),
            field(var("account"), "active"),
            further,
        ],
    )
}

fn record_sat_fixture() -> Vec<StatementInput> {
    record_fixture(
        "rsat",
        compare(
            ComparisonOperator::LessEqual,
            field(var("account"), "level"),
            number(10),
        ),
    )
}

fn record_unsat_fixture() -> Vec<StatementInput> {
    record_fixture(
        "runsat",
        compare(
            ComparisonOperator::Greater,
            field(var("account"), "level"),
            number(100),
        ),
    )
}

fn option_of_integer() -> ValueType {
    ValueType::option(integer())
}

/// `IsPresent(limit)` in the antecedent, `Unwrap(limit)` in the consequent, plus one assertion.
fn option_fixture(label: &str, further: Expression) -> Vec<StatementInput> {
    statements(
        label,
        vec![],
        vec![input("limit", option_of_integer())],
        vec![
            implies(
                present(var("limit")),
                compare(
                    ComparisonOperator::LessEqual,
                    unwrap(var("limit")),
                    number(10),
                ),
            ),
            further,
        ],
    )
}

fn option_sat_fixture() -> Vec<StatementInput> {
    option_fixture("osat", present(var("limit")))
}

fn option_unsat_fixture() -> Vec<StatementInput> {
    option_fixture(
        "ounsat",
        boolean(
            BooleanOperator::ShortCircuitAnd,
            present(var("limit")),
            compare(
                ComparisonOperator::Greater,
                unwrap(var("limit")),
                number(10),
            ),
        ),
    )
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quire-record-{label}-{}-{counter}",
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
    let directory = TempDirectory::new("response");
    let path = directory.path().join("solver");
    let body = format!(
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ] || [ \"$1\" = \"--version\" ]; then printf 'fake 1.0\\n'; exit 0; fi\n/bin/cat >/dev/null; printf '%s' '{response}'\n"
    );
    fs::write(&path, body).expect("write solver");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions).expect("executable");
    let config = SolverConfig::new(engine, &path, AdapterLimits::default()).expect("config");
    execute_solver(query, &config, &CancellationToken::default())
}

/// Both fake engines answer `response`; the run and report are checked for agreement.
fn decide(query: &QueryBundle, response: &str) -> quire_analyze::DifferentialRun {
    let z3 = response_record(query, response, SolverEngine::Z3);
    let cvc5 = response_record(query, response, SolverEngine::Cvc5);
    compare_solver_records(query, &z3, &cvc5)
}

/// FR-020-AC-6: the contract lists the record and option constructs as exact and no construct is
/// in both lists.
#[trace("TC-020", "FR-020-AC-6")]
#[test]
fn capability_contract_lists_record_and_option_constructs_as_exact() {
    for exact in [
        "record",
        "record_equality",
        "field_access",
        "option",
        "option_equality",
        "option_presence",
        "option_access",
    ] {
        assert!(CAPABILITY_CONTRACT.exact_constructs.contains(&exact));
        assert!(!CAPABILITY_CONTRACT.unsupported_constructs.contains(&exact));
    }
    for unsupported in ["collection", "collection_access", "text"] {
        assert!(CAPABILITY_CONTRACT
            .unsupported_constructs
            .contains(&unsupported));
    }
    for unsupported in CAPABILITY_CONTRACT.unsupported_constructs {
        assert!(!CAPABILITY_CONTRACT.exact_constructs.contains(unsupported));
    }
}

/// FR-020-AC-3, FR-020-AC-9 and NFR-020-AC-2: the record fixture lowers to a golden datatype
/// query under `QF_UFDTLIA`, byte-identical on repetition.
#[trace("TC-020", "FR-020-AC-3", "FR-020-AC-9", "NFR-020-AC-2")]
#[test]
fn record_fixture_lowers_to_the_golden_query() {
    let query = lower(record_sat_fixture());
    assert_eq!(query.profile(), "quire.smtlib2/v2");
    assert_eq!(query.logic(), "QF_UFDTLIA");
    assert_eq!(query.query(), include_str!("golden/record-sat-v2.smt2"));
    assert_eq!(query.query(), lower(record_sat_fixture()).query());
    assert_eq!(query.record_sorts().len(), 1);
    let sort = &query.record_sorts()[0];
    assert_eq!(sort.name, "Account");
    let fields: Vec<_> = sort
        .fields
        .iter()
        .map(|field| field.field.as_str())
        .collect();
    assert_eq!(fields, ["active", "level", "role"]);
    assert_eq!(query.enum_sorts().len(), 1);
    assert_eq!(sort.fields[2].sort, query.enum_sorts()[0].sort);
    assert_eq!(sort.fields[1].sort, "Int");
    assert_eq!(sort.fields[0].sort, "Bool");
    assert!(query.option_sorts().is_empty());
}

/// FR-020-AC-3, FR-020-AC-4 and FR-020-AC-9: the option fixture lowers to a golden datatype
/// query, with `Unwrap` as the `some` selector.
#[trace("TC-020", "FR-020-AC-3", "FR-020-AC-4", "FR-020-AC-9")]
#[test]
fn option_fixture_lowers_to_the_golden_query() {
    let query = lower(option_sat_fixture());
    assert_eq!(query.logic(), "QF_UFDTLIA");
    assert_eq!(query.query(), include_str!("golden/option-sat-v2.smt2"));
    assert_eq!(query.query(), lower(option_sat_fixture()).query());
    assert_eq!(query.option_sorts().len(), 1);
    let option = &query.option_sorts()[0];
    assert_eq!(option.payload_sort, "Int");
    let variable = &query.variables()[0].symbol;
    assert!(query
        .query()
        .contains(&format!("((_ is {}) {variable})", option.some)));
    assert!(query
        .query()
        .contains(&format!("({} {variable})", option.selector)));
}

/// FR-020-AC-3: the record fixtures are decided by controlled responses from both engines, and
/// the report names the datatype logic.
#[trace("TC-020", "FR-020-AC-3", "FR-020-AC-9")]
#[test]
fn record_fixtures_are_decided_and_reported_under_the_datatype_logic() {
    let query = lower(record_unsat_fixture());
    let run = decide(&query, "unsat\n");
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Refuted));
    let report = render_differential_report(&query, &run).expect("report");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
    assert_eq!(value["encodingProfile"], "quire.smtlib2/v2");
    assert_eq!(value["logic"], "QF_UFDTLIA");
    validate_report_document(&report).expect("document valid");
    validate_differential_report(&report, &query, &run).expect("valid");
}

/// FR-020-AC-3: a satisfiable record model is decided sat but stays unverified, naming the record
/// variable, until datatype witness replay exists.
#[trace("TC-020", "FR-020-AC-3", "FR-020-AC-10")]
#[test]
fn satisfiable_record_model_is_decided_but_unverified() {
    let query = lower(record_sat_fixture());
    let sort = &query.record_sorts()[0];
    let variable = &query.variables()[0].symbol;
    let model = format!(
        "sat\n(model (define-fun {variable} () {} ({} true 5 {})))\n",
        sort.sort,
        sort.constructor,
        query.enum_sorts()[0].constructors[0].symbol
    );
    let run = decide(&query, &model);
    assert_eq!(run.disposition(), DifferentialDisposition::Inconclusive);
    assert!(!run.is_conclusive());
    let conclusion = classify_analysis(&query, &response_record(&query, &model, SolverEngine::Z3));
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    assert!(conclusion.verified_model().is_none());
    assert_eq!(conclusion.explanation(), ExplanationState::Incomplete);
    let failure = conclusion.replay_failure().expect("replay failure");
    assert_eq!(failure.reason(), ReplayFailureReason::SortNotReplayable);
    assert_eq!(failure.subject(), Some(variable.as_str()));
}

/// FR-020-AC-3: the option fixtures are decided, and a satisfiable option model stays unverified.
#[trace("TC-020", "FR-020-AC-3", "FR-020-AC-10")]
#[test]
fn option_fixtures_are_decided_and_models_stay_unverified() {
    let query = lower(option_unsat_fixture());
    let run = decide(&query, "unsat\n");
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Refuted));
    let report = render_differential_report(&query, &run).expect("report");
    validate_differential_report(&report, &query, &run).expect("valid");

    let query = lower(option_sat_fixture());
    let option = &query.option_sorts()[0];
    let variable = &query.variables()[0].symbol;
    let model = format!(
        "sat\n(model (define-fun {variable} () {} ({} 3)))\n",
        option.sort, option.some
    );
    let conclusion = classify_analysis(&query, &response_record(&query, &model, SolverEngine::Z3));
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    assert!(conclusion.verified_model().is_none());
    let failure = conclusion.replay_failure().expect("replay failure");
    assert_eq!(failure.reason(), ReplayFailureReason::SortNotReplayable);
    assert_eq!(failure.subject(), Some(variable.as_str()));
    // An empty model gets the same reason, naming the same first datatype-sorted variable.
    let empty = classify_analysis(&query, &response_record(&query, "sat\n", SolverEngine::Z3));
    assert!(empty.verified_model().is_none());
    assert_eq!(
        empty.replay_failure().expect("failure").reason(),
        ReplayFailureReason::SortNotReplayable
    );
}

/// FR-020-AC-3: a query holding only record or option literals has no datatype variable and is
/// still never verified; the reason names the first declared datatype sort.
#[trace("TC-020", "FR-020-AC-10")]
#[test]
fn literal_only_record_and_option_queries_are_never_verified() {
    let option = option_of_integer();
    let literal_only = statements(
        "literals",
        account_types(),
        vec![input("flag", ValueType::Boolean)],
        vec![
            boolean(
                BooleanOperator::TotalOr,
                var("flag"),
                present(some(option, number(3))),
            ),
            compare(
                ComparisonOperator::Equal,
                field(
                    record(
                        "Account",
                        vec![
                            ("role", variant("Role", "Guest")),
                            ("active", var("flag")),
                            ("level", number(4)),
                        ],
                    ),
                    "active",
                ),
                var("flag"),
            ),
        ],
    );
    let query = lower(literal_only);
    assert_eq!(query.logic(), "QF_UFDTLIA");
    assert_eq!(
        query.enum_sorts().len() + query.record_sorts().len() + query.option_sorts().len(),
        3
    );
    let flag = &query.variables()[0].symbol;
    let model = format!("sat\n(model (define-fun {flag} () Bool true))\n");
    let conclusion = classify_analysis(&query, &response_record(&query, &model, SolverEngine::Z3));
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    assert!(conclusion.verified_model().is_none());
    let failure = conclusion.replay_failure().expect("replay failure");
    assert_eq!(failure.reason(), ReplayFailureReason::SortNotReplayable);
    let first = [
        query.enum_sorts()[0].sort.clone(),
        query.record_sorts()[0].sort.clone(),
        query.option_sorts()[0].sort.clone(),
    ]
    .into_iter()
    .min()
    .expect("a sort");
    assert_eq!(failure.subject(), Some(first.as_str()));
}

/// FR-020-AC-3: integer parts of a datatype variable are range-constrained in the query, so a
/// solver cannot choose a value outside the declared type.
#[trace("TC-020", "FR-020-AC-3")]
#[test]
fn integer_parts_of_datatype_variables_are_range_constrained() {
    let record = lower(record_sat_fixture());
    let account = &record.record_sorts()[0];
    let level = &account.fields[1].selector;
    let variable = &record.variables()[0].symbol;
    assert!(record.query().contains(&format!(
        "(assert (and (<= 0 ({level} {variable})) (<= ({level} {variable}) 100)))"
    )));
    let option = lower(option_sat_fixture());
    let sort = &option.option_sorts()[0];
    let variable = &option.variables()[0].symbol;
    let value = format!("({} {variable})", sort.selector);
    assert!(option.query().contains(&format!(
        "(assert (=> ((_ is {}) {variable}) (and (<= 0 {value}) (<= {value} 100))))",
        sort.some
    )));
}

/// FR-020-AC-1 (record and option equality): equality and inequality lower to `=` and
/// `distinct`, and literals lower to the constructor applications in field order.
#[trace("TC-020", "FR-020-AC-3")]
#[test]
fn record_and_option_equality_lower_to_equals_and_distinct() {
    let option = option_of_integer();
    let query = lower_boolean_statements(
        &statements(
            "eq",
            account_types(),
            vec![
                input("account", record_ref("Account")),
                input("limit", option.clone()),
            ],
            vec![
                compare(
                    ComparisonOperator::Equal,
                    var("account"),
                    record(
                        "Account",
                        vec![
                            ("level", number(7)),
                            ("role", variant("Role", "Admin")),
                            ("active", var_true()),
                        ],
                    ),
                ),
                compare(
                    ComparisonOperator::NotEqual,
                    var("limit"),
                    some(option.clone(), number(2)),
                ),
                compare(ComparisonOperator::Equal, var("limit"), none(option)),
            ],
        ),
        &[],
    )
    .expect("lower");
    let account = &query.record_sorts()[0];
    let option = &query.option_sorts()[0];
    let admin = &query.enum_sorts()[0].constructors[0].symbol;
    let text = query.query();
    assert!(text.contains(&format!("({} true 7 {admin})", account.constructor)));
    assert!(text.contains("(= v_s_"));
    assert!(text.contains("(distinct v_s_"));
    assert!(text.contains(&format!("({} 2)", option.some)));
    assert!(text.contains(&format!(" {})", option.none)));
}

fn var_true() -> Expression {
    node(ExpressionKind::BooleanLiteral { value: true })
}

/// FR-020-AC-3: two options over the same payload sort share one datatype, and an option over a
/// record is declared after the record.
#[trace("TC-020", "FR-020-AC-3", "FR-020-AC-7")]
#[test]
fn options_over_one_payload_share_a_datatype_declared_after_its_payload() {
    let shared = lower_boolean_statements(
        &statements(
            "share",
            vec![],
            vec![
                input("a", ValueType::option(ValueType::Boolean)),
                input("b", ValueType::option(ValueType::Boolean)),
            ],
            vec![boolean(
                BooleanOperator::TotalOr,
                present(var("a")),
                present(var("b")),
            )],
        ),
        &[],
    )
    .expect("lower");
    assert_eq!(shared.option_sorts().len(), 1);
    assert_eq!(shared.option_sorts()[0].payload_sort, "Bool");

    let nested = lower_boolean_statements(
        &statements(
            "order",
            account_types(),
            vec![input(
                "maybe",
                ValueType::option(ValueType::option(record_ref("Account"))),
            )],
            vec![present(var("maybe"))],
        ),
        &[],
    )
    .expect("lower");
    let text = nested.query();
    let position = |sort: &str| {
        text.find(&format!("(declare-datatypes (({sort} 0))"))
            .expect("declared")
    };
    let record_sort = &nested.record_sorts()[0].sort;
    let enum_sort = &nested.enum_sorts()[0].sort;
    let inner = &nested.option_sorts()[0];
    let outer = &nested.option_sorts()[1];
    let (inner, outer) = if inner.payload_sort == *record_sort {
        (inner, outer)
    } else {
        (outer, inner)
    };
    assert!(position(enum_sort) < position(record_sort));
    assert!(position(record_sort) < position(&inner.sort));
    assert!(position(&inner.sort) < position(&outer.sort));
    assert_eq!(outer.payload_sort, inner.sort);
}

/// FR-020-AC-2: only types reachable from a value or expression of a selected statement are
/// declared.
#[trace("TC-020", "FR-020-AC-2")]
#[test]
fn unused_record_declarations_do_not_change_the_query() {
    let used = statements(
        "unused",
        vec![record_type("Flags", &[("on", ValueType::Boolean)])],
        vec![input("flags", record_ref("Flags"))],
        vec![field(var("flags"), "on")],
    );
    let with_extra = statements(
        "unused",
        vec![
            record_type("Flags", &[("on", ValueType::Boolean)]),
            record_type("Other", &[("off", ValueType::Boolean)]),
            enum_type("Level", &["High", "Low"]),
        ],
        vec![
            input("flags", record_ref("Flags")),
            input("other", record_ref("Other")),
        ],
        vec![field(var("flags"), "on")],
    );
    assert_eq!(
        lower_boolean_statements(&used, &[]).expect("lower").query(),
        lower_boolean_statements(&with_extra, &[])
            .expect("lower")
            .query()
    );
}

/// FR-020-AC-4: an `Unwrap` with no presence proof is refused by `check_expression`, and the
/// proofs the spec lists admit it and lower it to the selector.
#[trace("TC-020", "FR-020-AC-4")]
#[test]
fn unwrap_needs_a_presence_proof() {
    let option = option_of_integer();
    let values = || vec![input("limit", option.clone())];
    let bare = compare(
        ComparisonOperator::LessEqual,
        unwrap(var("limit")),
        number(10),
    );
    // Total `and` and `or`, a negated antecedent and a bare use establish no presence fact.
    let unproven = [
        bare.clone(),
        boolean(
            BooleanOperator::TotalAnd,
            present(var("limit")),
            bare.clone(),
        ),
        boolean(
            BooleanOperator::TotalOr,
            present(var("limit")),
            bare.clone(),
        ),
        implies(
            node(ExpressionKind::BooleanNot {
                operand: Box::new(present(var("limit"))),
            }),
            bare.clone(),
        ),
    ];
    for expression in &unproven {
        let refusal = check_refusal(vec![], values(), expression);
        assert!(refusal.contains("OptionPresence"), "{refusal}");
        assert!(try_statements("noproof", vec![], values(), vec![expression.clone()]).is_err());
    }
    let proven = [
        implies(present(var("limit")), bare.clone()),
        boolean(
            BooleanOperator::ShortCircuitAnd,
            present(var("limit")),
            bare.clone(),
        ),
        boolean(
            BooleanOperator::ShortCircuitOr,
            node(ExpressionKind::BooleanNot {
                operand: Box::new(present(var("limit"))),
            }),
            bare.clone(),
        ),
        compare(
            ComparisonOperator::LessEqual,
            unwrap(some(option.clone(), number(4))),
            number(10),
        ),
    ];
    for expression in proven {
        let query = lower_boolean_statements(
            &statements("proof", vec![], values(), vec![expression]),
            &[],
        )
        .expect("lowers under the proof");
        let selector = &query.option_sorts()[0].selector;
        assert!(query.query().contains(&format!("({selector} ")));
    }
}

/// FR-020-AC-4: the absent option has no observable value. `none` lowers to its constructor,
/// presence of `none` to the `some` tester, and the selector is never constrained on `none`.
#[trace("TC-020", "FR-020-AC-4")]
#[test]
fn absent_option_selector_is_unconstrained() {
    let option = option_of_integer();
    let query = lower_boolean_statements(
        &statements(
            "absent",
            vec![],
            vec![input("limit", option.clone())],
            vec![
                compare(
                    ComparisonOperator::Equal,
                    var("limit"),
                    none(option.clone()),
                ),
                node(ExpressionKind::BooleanNot {
                    operand: Box::new(present(var("limit"))),
                }),
            ],
        ),
        &[],
    )
    .expect("lower");
    let sort = &query.option_sorts()[0];
    let variable = &query.variables()[0].symbol;
    let text = query.query();
    assert!(text.contains(&format!("(= {variable} {})", sort.none)));
    assert!(text.contains(&format!("(not ((_ is {}) {variable}))", sort.some)));
    // The selector appears in the datatype declaration and in the guarded range constraint only.
    let uses = text.matches(&sort.selector).count();
    assert_eq!(uses, 3, "{text}");
    assert!(text.contains(&format!(
        "(declare-datatypes (({} 0)) ((({}) ({} ({} Int)))))",
        sort.sort, sort.none, sort.some, sort.selector
    )));
}

/// FR-020-AC-5: a record or option that nests a collection or text, at any depth, is
/// `UnsupportedType` naming the declaration and the nested field path.
#[trace("TC-020", "FR-020-AC-5")]
#[test]
fn nested_collections_and_text_are_unsupported_types() {
    let collection = ValueType::collection(CollectionType::new(ValueType::Boolean, 4).expect("c"));
    let on = || field(var("v"), "on");
    let cases: Vec<(Vec<TypeDeclaration>, ValueType, Expression, &str)> = vec![
        (
            vec![record_type(
                "Profile",
                &[("on", ValueType::Boolean), ("name", ValueType::Text)],
            )],
            record_ref("Profile"),
            on(),
            "type.Profile.name",
        ),
        (
            vec![record_type(
                "Bag",
                &[("on", ValueType::Boolean), ("items", collection.clone())],
            )],
            record_ref("Bag"),
            on(),
            "type.Bag.items",
        ),
        (
            vec![
                record_type("Mid", &[("items", collection.clone())]),
                record_type("Outer", &[("inner", ValueType::option(record_ref("Mid")))]),
            ],
            record_ref("Outer"),
            present(field(var("v"), "inner")),
            "type.Outer.inner.some.items",
        ),
        (
            vec![],
            ValueType::option(ValueType::Text),
            present(var("v")),
            "type.option.some",
        ),
        (
            vec![],
            ValueType::option(collection),
            present(var("v")),
            "type.option.some",
        ),
    ];
    for (types, value_type, expression, path) in cases {
        let errors = lower_error(statements(
            "nest",
            types,
            vec![input("v", value_type)],
            vec![expression],
        ));
        assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedType);
        assert_eq!(errors[0].path(), path, "{:?}", errors[0]);
    }
}

/// FR-020-AC-5: a record that contains itself fails at environment build with `RecursiveType`,
/// directly and through an option.
#[trace("TC-020", "FR-020-AC-5")]
#[test]
fn self_containing_records_fail_at_environment_build() {
    let direct = environment(
        "cycle",
        vec![record_type("Node", &[("next", record_ref("Node"))])],
        vec![],
    )
    .expect_err("refused");
    assert!(direct.contains("RecursiveType"), "{direct}");
    let through_option = environment(
        "cycle",
        vec![
            record_type("A", &[("b", ValueType::option(record_ref("B")))]),
            record_type("B", &[("a", record_ref("A"))]),
        ],
        vec![],
    )
    .expect_err("refused");
    assert!(through_option.contains("RecursiveType"), "{through_option}");
}

/// FR-020-AC-5: an ordering comparison of records or options, and a comparison across sorts, is
/// refused by `check_expression` as ill-typed.
#[trace("TC-020", "FR-020-AC-5")]
#[test]
fn ordering_and_mixed_sort_comparisons_are_refused_at_statement_build() {
    let types = || {
        let mut types = account_types();
        types.push(record_type("Flags", &[("on", ValueType::Boolean)]));
        types
    };
    let values = || {
        vec![
            input("account", record_ref("Account")),
            input("flags", record_ref("Flags")),
            input("limit", option_of_integer()),
            input("flag", ValueType::Boolean),
        ]
    };
    let cases = [
        compare(ComparisonOperator::Less, var("account"), var("account")),
        compare(ComparisonOperator::Less, var("limit"), var("limit")),
        compare(ComparisonOperator::Equal, var("account"), var("flags")),
        compare(ComparisonOperator::Equal, var("account"), var("limit")),
        compare(ComparisonOperator::Equal, var("flag"), var("limit")),
    ];
    for expression in &cases {
        let refusal = check_refusal(types(), values(), expression);
        assert!(refusal.contains("IllTypedExpression"), "{refusal}");
    }
}

/// FR-020-AC-7 and NFR-020-AC-2: permuting type, value, field and statement order preserves the
/// query and the maps, and two records with the same fields stay distinct sorts.
#[trace("TC-020", "FR-020-AC-7", "NFR-020-AC-2")]
#[test]
fn permutation_preserves_bytes_and_equal_fields_stay_distinct() {
    let build = |reversed: bool| {
        let mut left = vec![("a", ValueType::Boolean), ("b", integer())];
        let mut right = left.clone();
        let expressions = vec![
            field(var("one"), "a"),
            field(var("two"), "a"),
            present(var("maybe")),
        ];
        let mut values = vec![
            input("one", record_ref("One")),
            input("two", record_ref("Two")),
            input("maybe", ValueType::option(record_ref("One"))),
        ];
        let mut types = vec![record_type("One", &left), record_type("Two", &right)];
        if reversed {
            left.reverse();
            right.reverse();
            types = vec![record_type("Two", &right), record_type("One", &left)];
            values.reverse();
        }
        let mut built = statements("perm", types, values, expressions);
        if reversed {
            built.reverse();
        }
        lower(built)
    };
    let forward = build(false);
    let backward = build(true);
    assert_eq!(forward.query(), backward.query());
    assert_eq!(forward.record_sorts(), backward.record_sorts());
    assert_eq!(forward.option_sorts(), backward.option_sorts());
    let sorts = forward.record_sorts();
    assert_eq!(sorts.len(), 2);
    assert_ne!(sorts[0].sort, sorts[1].sort);
    assert_ne!(sorts[0].constructor, sorts[1].constructor);
    assert_ne!(sorts[0].fields[0].selector, sorts[1].fields[0].selector);
}

/// NFR-020-AC-1: 64 fields per record are accepted and 65 are refused as a resource limit.
#[trace("TC-020", "NFR-020-AC-1")]
#[test]
fn record_field_bound_is_exact() {
    let at = |count: usize| {
        let names: Vec<String> = (0..count).map(|index| format!("f{index}")).collect();
        let fields: Vec<(&str, ValueType)> = names
            .iter()
            .map(|name| (name.as_str(), ValueType::Boolean))
            .collect();
        statements(
            "fields",
            vec![record_type("Wide", &fields)],
            vec![input("wide", record_ref("Wide"))],
            vec![field(var("wide"), "f0")],
        )
    };
    let query = lower_boolean_statements(&at(64), &[]).expect("64 accepted");
    assert_eq!(query.record_sorts()[0].fields.len(), 64);
    let errors = lower_error(at(65));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);
}

/// NFR-020-AC-1: a chain of 8 datatypes is accepted and 9 are refused, counting records, options
/// and enums alike.
#[trace("TC-020", "NFR-020-AC-1")]
#[test]
fn datatype_depth_bound_is_exact() {
    // R1 { next: R2 }, ... with the last record holding a Boolean: depth is the record count.
    let records = |count: usize| {
        let types: Vec<TypeDeclaration> = (1..=count)
            .map(|index| {
                if index == count {
                    record_type(&format!("R{index}"), &[("leaf", ValueType::Boolean)])
                } else {
                    record_type(
                        &format!("R{index}"),
                        &[("next", record_ref(&format!("R{}", index + 1)))],
                    )
                }
            })
            .collect();
        let mut access = var("root");
        for _ in 1..count {
            access = field(access, "next");
        }
        statements(
            "depth",
            types,
            vec![input("root", record_ref("R1"))],
            vec![field(access, "leaf")],
        )
    };
    lower_boolean_statements(&records(8), &[]).expect("depth 8 accepted");
    let errors = lower_error(records(9));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);

    // Option over option: a payload of Boolean makes the outermost option depth `levels`.
    let options = |levels: usize| {
        let mut value_type = ValueType::Boolean;
        for _ in 0..levels {
            value_type = ValueType::option(value_type);
        }
        statements(
            "optdepth",
            vec![],
            vec![input("deep", value_type)],
            vec![present(var("deep"))],
        )
    };
    lower_boolean_statements(&options(8), &[]).expect("depth 8 accepted");
    let errors = lower_error(options(9));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);

    // The count mixes kinds: record(1) -> option(2) -> enum(3) is depth 3, and an enum under
    // seven option layers inside a record is depth 9.
    let mut value_type = enum_ref("Role");
    for _ in 0..6 {
        value_type = ValueType::option(value_type);
    }
    let mixed = |holder: ValueType| {
        statements(
            "mixed",
            vec![
                enum_type("Role", ROLES),
                record_type("Holder", &[("value", holder)]),
            ],
            vec![input("holder", record_ref("Holder"))],
            vec![present(field(var("holder"), "value"))],
        )
    };
    lower_boolean_statements(&mixed(value_type.clone()), &[]).expect("depth 8 accepted");
    let errors = lower_error(mixed(ValueType::option(value_type)));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);
}

/// NFR-020-AC-1: 256 datatypes per query are accepted and 257 are refused as a resource limit,
/// counting each record and each distinct option payload once.
#[trace("TC-020", "NFR-020-AC-1")]
#[test]
fn datatype_count_bound_is_exact() {
    let at = |count: usize| {
        let types = (0..count)
            .map(|index| record_type(&format!("R{index}"), &[("on", ValueType::Boolean)]))
            .collect();
        let values = (0..count)
            .map(|index| input(&format!("r{index}"), record_ref(&format!("R{index}"))))
            .collect();
        let expressions = (0..count)
            .map(|index| field(var(&format!("r{index}")), "on"))
            .collect();
        statements("count", types, values, expressions)
    };
    let query = lower_boolean_statements(&at(256), &[]).expect("256 accepted");
    assert_eq!(query.record_sorts().len(), 256);
    let errors = lower_error(at(257));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);

    // 255 records plus the one option over Boolean make 256; a second distinct payload makes 257.
    let mixed = |extra_option: bool| {
        let types = (0..255)
            .map(|index| record_type(&format!("R{index}"), &[("on", ValueType::Boolean)]))
            .collect();
        let mut values: Vec<ValueDeclaration> = (0..255)
            .map(|index| input(&format!("r{index}"), record_ref(&format!("R{index}"))))
            .collect();
        let mut expressions: Vec<Expression> = (0..255)
            .map(|index| field(var(&format!("r{index}")), "on"))
            .collect();
        values.push(input("a", ValueType::option(ValueType::Boolean)));
        values.push(input("b", ValueType::option(ValueType::Boolean)));
        expressions.push(present(var("a")));
        expressions.push(present(var("b")));
        if extra_option {
            values.push(input("c", ValueType::option(integer())));
            expressions.push(present(var("c")));
        }
        statements("mixedcount", types, values, expressions)
    };
    lower_boolean_statements(&mixed(false), &[]).expect("256 accepted");
    let errors = lower_error(mixed(true));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);
}

/// FR-020-AC-3: real solvers decide the record and option fixtures; run only where installed.
#[trace("TC-020", "FR-020-AC-3")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn real_z3_cvc5_decide_record_and_option_fixtures() {
    let z3_path = PathBuf::from(std::env::var_os("QUIRE_Z3").expect("QUIRE_Z3 is required"));
    let cvc5_path = PathBuf::from(std::env::var_os("QUIRE_CVC5").expect("QUIRE_CVC5 is required"));
    let z3 = SolverConfig::new(SolverEngine::Z3, z3_path, AdapterLimits::default()).expect("z3");
    let cvc5 =
        SolverConfig::new(SolverEngine::Cvc5, cvc5_path, AdapterLimits::default()).expect("cvc5");
    let token = CancellationToken::default();
    for (fixture, outcome) in [
        (record_sat_fixture(), SolverOutcome::Sat),
        (record_unsat_fixture(), SolverOutcome::Unsat),
        (option_sat_fixture(), SolverOutcome::Sat),
        (option_unsat_fixture(), SolverOutcome::Unsat),
    ] {
        let query = lower(fixture);
        for config in [&z3, &cvc5] {
            assert_eq!(execute_solver(&query, config, &token).outcome(), outcome);
        }
    }
}
