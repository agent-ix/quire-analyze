//! Requirement-tagged tests for enum, record and option witness replay (FR-021, NFR-020-AC-3,
//! TC-020). The solvers are fake adapters that print recorded model text in each engine's
//! spelling; no real solver is run.

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
    AdapterLimits, AnalysisConclusion, AnalysisRequest, AnalysisStatus, CancellationToken,
    DifferentialDisposition, ExplanationState, LoweringError, QueryBundle, ReplayFailureReason,
    SolverConfig, SolverEngine, SolverOutcome, StatementInput, WitnessValue,
};
use quire_contract_model::{
    AnchorName, BooleanOperator, Clause, ClauseId, ClauseKind, ComparisonOperator,
    ContractPackage, DeclarationEnvironment, EnumDeclaration, EnumVariantDeclaration,
    ExecutionPoint, Expression, ExpressionKind, IntegerDomain, IntegerType, OverflowPolicy,
    PackageId, RecordDeclaration, RecordFieldDeclaration, Requirement, RequirementId,
    RequirementRef, RequirementRevision, SchemaVersion, SourceDocumentId, SourceIdentity,
    SourceLocation, SourceRevision, SourceSpan, StateObservation, SymbolName, TypeDeclaration,
    TypedExpression, ValueDeclaration, ValueDeclarationKind, ValueType,
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

fn lower(statements: Vec<StatementInput>) -> QueryBundle {
    let request = AnalysisRequest::consistency(vec![], statements, vec![]).expect("request");
    lower_analysis_request(&request).expect("lower")
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
    fake_script::install(&path, &body);
    let config = SolverConfig::new(engine, &path, limits).expect("config");
    execute_solver(query, &config, &CancellationToken::default())
}

/// Both fake engines answer `response`; the run and report are checked for agreement.
fn decide(query: &QueryBundle, response: &str) -> quire_analyze::DifferentialRun {
    let z3 = response_record(query, response, SolverEngine::Z3);
    let cvc5 = response_record(query, response, SolverEngine::Cvc5);
    compare_solver_records(query, &z3, &cvc5)
}


fn not(operand: Expression) -> Expression {
    node(ExpressionKind::BooleanNot {
        operand: Box::new(operand),
    })
}

fn option_of_integer() -> ValueType {
    ValueType::option(integer())
}

/// An account with a Boolean, an optional integer limit and an enum role.
fn account_types() -> Vec<TypeDeclaration> {
    vec![
        enum_type("Role", ROLES),
        record_type(
            "Account",
            &[
                ("role", enum_ref("Role")),
                ("active", ValueType::Boolean),
                ("limit", option_of_integer()),
            ],
        ),
    ]
}

/// `role = Admin`, `active`, and `limit <= 10` when the limit is present.
fn account_query() -> QueryBundle {
    let limit = || field(var("account"), "limit");
    lower(statements(
        "account",
        account_types(),
        vec![input("account", record_ref("Account"))],
        vec![
            compare(
                ComparisonOperator::Equal,
                field(var("account"), "role"),
                variant("Role", "Admin"),
            ),
            field(var("account"), "active"),
            implies(
                present(limit()),
                compare(ComparisonOperator::LessEqual, unwrap(limit()), number(10)),
            ),
        ],
    ))
}

/// The encoded names of the account query, and the model texts built from them.
struct Account {
    query: QueryBundle,
    variable: String,
    sort: String,
    constructor: String,
    option_sort: String,
    none: String,
    some: String,
    role_sort: String,
    admin: String,
    auditor: String,
}

impl Account {
    fn new() -> Self {
        let query = account_query();
        let option = &query.option_sorts()[0];
        let record = &query.record_sorts()[0];
        let role = &query.enum_sorts()[0];
        Self {
            variable: query.variables()[0].symbol.clone(),
            sort: record.sort.clone(),
            constructor: record.constructor.clone(),
            option_sort: option.sort.clone(),
            none: option.none.clone(),
            some: option.some.clone(),
            role_sort: role.sort.clone(),
            admin: role.constructors[0].symbol.clone(),
            auditor: role.constructors[1].symbol.clone(),
            query,
        }
    }

    /// A Z3-spelled model binding the account variable to `value` under the sort `sort`.
    fn z3(&self, sort: &str, value: &str) -> String {
        format!("sat\n(model (define-fun {} () {sort} {value}))\n", self.variable)
    }

    /// A cvc5-spelled model.
    fn cvc5(&self, sort: &str, value: &str) -> String {
        format!("sat\n((define-fun {} () {sort} {value}))\n", self.variable)
    }

    fn record(&self, active: &str, limit: &str, role: &str) -> String {
        format!("({} {active} {limit} {role})", self.constructor)
    }

    fn conclude(&self, model: &str, engine: SolverEngine) -> AnalysisConclusion {
        classify_analysis(&self.query, &response_record(&self.query, model, engine))
    }

    fn reason(&self, model: &str) -> ReplayFailureReason {
        let conclusion = self.conclude(model, SolverEngine::Z3);
        assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
        assert!(conclusion.verified_model().is_none(), "{model}");
        assert_eq!(conclusion.explanation(), ExplanationState::Incomplete);
        conclusion.replay_failure().expect("replay failure").reason()
    }
}

/// FR-021-AC-1 and FR-021-AC-3: a record nesting an option and an enum, with the option present
/// and absent, parses from both engines' spellings into one typed value and replays.
#[trace("TC-020", "FR-021-AC-1", "FR-021-AC-3")]
#[test]
fn nested_record_replays_from_both_spellings() {
    let account = Account::new();
    let present = format!("({} 5)", account.some);
    let absent_plain = account.none.clone();
    let absent_qualified = format!("(as {} {})", account.none, account.option_sort);
    let absent_parametric = format!("(as {} (Option Int))", account.none);
    let cases = [
        (&present, "{active: true, limit: present(5), role: Admin}"),
        (&absent_plain, "{active: true, limit: absent, role: Admin}"),
        (&absent_qualified, "{active: true, limit: absent, role: Admin}"),
        (&absent_parametric, "{active: true, limit: absent, role: Admin}"),
    ];
    for (limit, expected) in cases {
        let value = account.record("true", limit, &account.admin);
        let z3 = account.conclude(&account.z3(&account.sort, &value), SolverEngine::Z3);
        let qualified = format!("(as {} {})", account.admin, account.role_sort);
        let cvc5_value = account.record("true", limit, &qualified);
        let cvc5 = account.conclude(&account.cvc5(&account.sort, &cvc5_value), SolverEngine::Cvc5);
        for conclusion in [&z3, &cvc5] {
            assert_eq!(conclusion.explanation(), ExplanationState::Verified, "{limit}");
            let model = conclusion.verified_model().expect("verified");
            assert_eq!(model.datatype_values()[0].value().to_string(), expected);
            assert_eq!(model.datatype_values()[0].origins(), account.query.variables()[0].origins);
        }
        assert_eq!(
            z3.verified_model().expect("z3").datatype_values(),
            cvc5.verified_model().expect("cvc5").datatype_values()
        );
    }
    let run = compare_solver_records(
        &account.query,
        &response_record(
            &account.query,
            &account.z3(&account.sort, &account.record("true", &account.none, &account.admin)),
            SolverEngine::Z3,
        ),
        &response_record(
            &account.query,
            &account.cvc5(&account.sort, &account.record("true", &account.none, &account.admin)),
            SolverEngine::Cvc5,
        ),
    );
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
}

/// FR-021-AC-3: a lone enum, a lone present option and a lone absent option each replay; the
/// published enum is its variant name.
#[trace("TC-020", "FR-021-AC-3")]
#[test]
fn enum_and_option_kinds_replay_alone() {
    let statements = statements(
        "kinds",
        vec![enum_type("Role", ROLES)],
        vec![input("role", enum_ref("Role")), input("limit", option_of_integer())],
        vec![
            compare(ComparisonOperator::Equal, var("role"), variant("Role", "Guest")),
            present(var("limit")),
        ],
    );
    let query = lower(statements);
    let guest = &query.enum_sorts()[0].constructors[2].symbol;
    let enum_sort = &query.enum_sorts()[0].sort;
    let option = &query.option_sorts()[0];
    let symbols: Vec<_> = query.variables().iter().map(|v| v.symbol.clone()).collect();
    let (role, limit) = if query.query().contains(&format!("(declare-fun {} () {enum_sort})", symbols[0])) {
        (&symbols[0], &symbols[1])
    } else {
        (&symbols[1], &symbols[0])
    };
    let model = format!(
        "sat\n(model (define-fun {role} () {enum_sort} {guest}) (define-fun {limit} () {} ({} 7)))\n",
        option.sort, option.some
    );
    let conclusion = classify_analysis(&query, &response_record(&query, &model, SolverEngine::Z3));
    let verified = conclusion.verified_model().expect("verified");
    let values: Vec<_> = verified.datatype_values().iter().map(|v| v.value().clone()).collect();
    assert!(values.contains(&WitnessValue::Enum {
        enumeration: "Role".to_owned(),
        variant: "Guest".to_owned()
    }));
    assert!(values.contains(&WitnessValue::Option(Some(Box::new(WitnessValue::Integer(7))))));
}

/// FR-021-AC-2: every kind of value that does not match its declared datatype fails replay.
#[trace("TC-020", "FR-021-AC-2")]
#[test]
fn values_that_do_not_match_the_declared_datatype_fail_replay() {
    let account = Account::new();
    let good = |limit: &str| account.record("true", limit, &account.admin);
    let some = format!("({} 5)", account.some);
    let cases: Vec<(String, ReplayFailureReason)> = vec![
        // An unknown constructor and an abstract element are values of no declared datatype.
        (
            account.z3(&account.sort, &account.record("true", &some, "bogus")),
            ReplayFailureReason::ValueMalformed,
        ),
        (
            account.z3(&account.sort, &account.record("true", &some, "@uc_Role_0")),
            ReplayFailureReason::ValueMalformed,
        ),
        // A constructor of another datatype in a field.
        (
            account.z3(&account.sort, &account.record("true", &some, &account.none)),
            ReplayFailureReason::SortMismatch,
        ),
        // Wrong arity: one field too few, one too many, and an argument on a nullary constructor.
        (
            account.z3(&account.sort, &format!("({} true {some})", account.constructor)),
            ReplayFailureReason::SortMismatch,
        ),
        (
            account.z3(
                &account.sort,
                &format!("({} true {some} {} {})", account.constructor, account.admin, account.admin),
            ),
            ReplayFailureReason::SortMismatch,
        ),
        (
            account.z3(&account.sort, &good(&format!("({} 5)", account.none))),
            ReplayFailureReason::SortMismatch,
        ),
        (
            account.z3(&account.sort, &good(&format!("({})", account.some))),
            ReplayFailureReason::SortMismatch,
        ),
        // The record bound under another sort, or qualified with another sort.
        (account.z3("Int", &good(&some)), ReplayFailureReason::SortMismatch),
        (
            account.z3(&account.sort, &account.record("true", &some, &format!("(as {} Int)", account.admin))),
            ReplayFailureReason::SortMismatch,
        ),
        // Scalars of the wrong sort inside the record.
        (
            account.z3(&account.sort, &account.record("5", &some, &account.admin)),
            ReplayFailureReason::SortMismatch,
        ),
        (
            account.z3(&account.sort, &account.record("true", "(m_x true)", &account.admin)),
            ReplayFailureReason::ValueMalformed,
        ),
        (
            account.z3(&account.sort, &account.record("true", &format!("({} true)", account.some), &account.admin)),
            ReplayFailureReason::SortMismatch,
        ),
        // Malformed text: a bad numeral, a nested non-constructor term and an unclosed term.
        (
            account.z3(&account.sort, &account.record("true", &format!("({} 05)", account.some), &account.admin)),
            ReplayFailureReason::ValueMalformed,
        ),
        (
            account.z3(&account.sort, &account.record("true", &some, "(())")),
            ReplayFailureReason::ValueMalformed,
        ),
        (
            account.z3(&account.sort, &format!("({} true {some} {}", account.constructor, account.admin)),
            ReplayFailureReason::ValueMalformed,
        ),
    ];
    for (model, expected) in &cases {
        assert_eq!(account.reason(model), *expected, "{model}");
        let cvc5 = model.replace("(model (define-fun", "((define-fun");
        assert_eq!(account.reason(&cvc5), *expected, "{cvc5}");
    }
}

/// FR-021-AC-4: a model the solver reported but one assertion rejects under the independent
/// evaluator is incomplete, and the differential run is not conclusive.
#[trace("TC-020", "FR-021-AC-4")]
#[test]
fn a_model_that_violates_an_assertion_is_rejected() {
    let account = Account::new();
    let some = |value: u32| format!("({} {value})", account.some);
    let wrong_role = account.z3(&account.sort, &account.record("true", &some(5), &account.auditor));
    let inactive = account.z3(&account.sort, &account.record("false", &some(5), &account.admin));
    let over_limit = account.z3(&account.sort, &account.record("true", &some(11), &account.admin));
    for model in [&wrong_role, &inactive, &over_limit] {
        assert_eq!(account.reason(model), ReplayFailureReason::AssertionFalse, "{model}");
    }
    let record = response_record(&account.query, &wrong_role, SolverEngine::Z3);
    let run = compare_solver_records(&account.query, &record, &record);
    assert_eq!(run.disposition(), DifferentialDisposition::Inconclusive);
    // An integer part outside its declared range is out of domain even when every named
    // assertion holds.
    let out_of_range = account.z3(&account.sort, &account.record("true", &format!("({} 101)", account.some), &account.admin));
    assert_eq!(account.reason(&out_of_range), ReplayFailureReason::ValueOutOfDomain);
    let negative = account.z3(&account.sort, &account.record("true", &format!("({} (- 1))", account.some), &account.admin));
    assert_eq!(account.reason(&negative), ReplayFailureReason::ValueOutOfDomain);
}

/// FR-021-AC-5: a model that omits the datatype symbol, defines an unknown symbol, or defines the
/// symbol twice is not verified.
#[trace("TC-020", "FR-021-AC-5")]
#[test]
fn omitted_unknown_and_repeated_symbols_are_not_verified() {
    let account = Account::new();
    let value = account.record("true", &account.none, &account.admin);
    let define = |symbol: &str| format!("(define-fun {symbol} () {} {value})", account.sort);
    let own = define(&account.variable);
    let wrap = |bodies: &[&str]| format!("sat\n(model {})\n", bodies.join(" "));
    assert_eq!(account.reason(&wrap(&[])), ReplayFailureReason::SymbolMissing);
    assert_eq!(
        account.reason(&wrap(&[&own, &define("not_in_the_query")])),
        ReplayFailureReason::SymbolUnknown
    );
    assert_eq!(account.reason(&wrap(&[&own, &own])), ReplayFailureReason::SymbolRepeated);
    assert!(account.conclude(&wrap(&[&own]), SolverEngine::Z3).verified_model().is_some());
}

/// A query with one option variable and one assertion of the given shape.
fn guarded_query(label: &str, assertion: impl Fn(&dyn Fn() -> Expression) -> Expression) -> QueryBundle {
    let limit = || var("limit");
    lower(statements(
        label,
        vec![],
        vec![input("limit", option_of_integer())],
        vec![assertion(&limit)],
    ))
}

/// FR-021-AC-7: an implication consequent, a short-circuit `and` right operand and a
/// short-circuit `or` right operand are not evaluated when their guard decides the result, so an
/// `Unwrap` of an absent option under them replays; the solver's value for the selector is never
/// read.
#[trace("TC-020", "FR-021-AC-7")]
#[test]
fn guarded_unwrap_of_an_absent_option_replays_lazily() {
    let at_most_ten = |limit: &dyn Fn() -> Expression| {
        compare(ComparisonOperator::LessEqual, unwrap(limit()), number(10))
    };
    let implication = guarded_query("imp", |limit| {
        implies(present(limit()), at_most_ten(limit))
    });
    let and_right = guarded_query("and", |limit| {
        not(boolean(
            BooleanOperator::ShortCircuitAnd,
            present(limit()),
            at_most_ten(limit),
        ))
    });
    let or_right = guarded_query("or", |limit| {
        boolean(
            BooleanOperator::ShortCircuitOr,
            not(present(limit())),
            at_most_ten(limit),
        )
    });
    for (query, absent_ok, some_three, some_eleven) in [
        (&implication, true, true, false),
        (&and_right, true, false, true),
        (&or_right, true, true, false),
    ] {
        let option = &query.option_sorts()[0];
        let variable = &query.variables()[0].symbol;
        let model = |value: String| {
            format!("sat\n(model (define-fun {variable} () {} {value}))\n", option.sort)
        };
        for (value, verifies) in [
            (option.none.clone(), absent_ok),
            (format!("({} 3)", option.some), some_three),
            (format!("({} 11)", option.some), some_eleven),
        ] {
            let record = response_record(query, &model(value.clone()), SolverEngine::Z3);
            let conclusion = classify_analysis(query, &record);
            assert_eq!(conclusion.verified_model().is_some(), verifies, "{value}");
        }
    }
}

/// FR-021-AC-6 and NFR-020-AC-3: a model nested far beyond the datatype depth bound is rejected
/// without unbounded recursion, and a model beyond the model ceiling is a limit breach that is
/// never conclusive.
#[trace("TC-020", "FR-021-AC-6", "NFR-020-AC-3")]
#[test]
fn over_deep_and_over_large_models_are_rejected() {
    let account = Account::new();
    let depth = 40_000;
    let mut nested = String::new();
    for _ in 0..depth {
        nested.push_str(&format!("({} ", account.some));
    }
    nested.push('5');
    nested.push_str(&")".repeat(depth));
    let model = account.z3(&account.sort, &account.record("true", &nested, &account.admin));
    assert_eq!(account.reason(&model), ReplayFailureReason::SortMismatch);

    let value = account.record("true", &account.none, &account.admin);
    let model = account.z3(&account.sort, &value);
    let limits = AdapterLimits { model_bytes: 16, ..AdapterLimits::default() };
    let record = response_record_with(&account.query, &model, SolverEngine::Z3, limits);
    assert_eq!(record.outcome(), SolverOutcome::ModelLimit);
    let conclusion = classify_analysis(&account.query, &record);
    assert!(!conclusion.is_conclusive());
    assert!(conclusion.verified_model().is_none());
}
