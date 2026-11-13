//! Requirement-tagged tests for enum datatype lowering (FR-020 enum part, NFR-020, TC-020).

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
    validate_report_document, AdapterLimits, AnalysisRequest, AnalysisStatus, CancellationToken,
    DifferentialDisposition, ExplanationState, LoweringError, LoweringErrorCode, QueryBundle,
    ReplayFailureReason, SolverConfig, SolverEngine, SolverOutcome, StatementInput, WitnessValue,
    CAPABILITY_CONTRACT,
};
use quire_contract_model::{
    AnchorName, BooleanOperator, Clause, ClauseId, ClauseKind, ComparisonOperator, ContractPackage,
    DeclarationEnvironment, EnumDeclaration, EnumVariantDeclaration, ExecutionPoint, Expression,
    ExpressionKind, PackageId, Requirement, RequirementId, RequirementRef, RequirementRevision,
    SchemaVersion, SourceDocumentId, SourceIdentity, SourceLocation, SourceRevision, SourceSpan,
    StateObservation, SymbolName, TypeDeclaration, TypedExpression, ValueDeclaration,
    ValueDeclarationKind, ValueType,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn span() -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("enum-test").expect("source id"),
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

fn variant(enumeration: &str, name: &str) -> Expression {
    node(ExpressionKind::EnumLiteral {
        enumeration: symbol(enumeration),
        variant: symbol(name),
    })
}

fn compare(operator: ComparisonOperator, left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Compare {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn is(name: &str, enumeration: &str, value: &str) -> Expression {
    compare(
        ComparisonOperator::Equal,
        var(name),
        variant(enumeration, value),
    )
}

fn is_not(name: &str, enumeration: &str, value: &str) -> Expression {
    compare(
        ComparisonOperator::NotEqual,
        var(name),
        variant(enumeration, value),
    )
}

fn or(left: Expression, right: Expression) -> Expression {
    node(ExpressionKind::Boolean {
        operator: BooleanOperator::TotalOr,
        left: Box::new(left),
        right: Box::new(right),
    })
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

fn input(name: &str, value_type: ValueType) -> ValueDeclaration {
    ValueDeclaration::new(
        symbol(name),
        ValueDeclarationKind::Input,
        value_type,
        span(),
    )
}

fn enum_value(name: &str, enumeration: &str) -> ValueDeclaration {
    input(
        name,
        ValueType::Enum {
            name: symbol(enumeration),
        },
    )
}

const ROLES: &[&str] = &["Admin", "Auditor", "Guest", "User"];

/// One requirement whose clauses are `expressions`, lowered against one shared environment.
fn try_statements(
    label: &str,
    types: Vec<TypeDeclaration>,
    values: Vec<ValueDeclaration>,
    expressions: Vec<Expression>,
) -> Result<Vec<StatementInput>, String> {
    let package_id = PackageId::new("agent-ix/enum-test").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-{label}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let environment = DeclarationEnvironment::new(owner, types, values, vec![])
        .map_err(|diagnostics| format!("{diagnostics:?}"))?;
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

/// Builds a typed clause under `build` declarations, then supplies `run` declarations to statement
/// build, which re-checks the expression under them.
fn statement_build_errors(
    build: (Vec<TypeDeclaration>, Vec<ValueDeclaration>),
    run: (Vec<TypeDeclaration>, Vec<ValueDeclaration>),
    expression: Expression,
) -> Vec<LoweringError> {
    let package_id = PackageId::new("agent-ix/enum-test").expect("package");
    let requirement_id = RequirementId::new("REQ-swap").expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let build_environment =
        DeclarationEnvironment::new(owner.clone(), build.0, build.1, vec![]).expect("build env");
    let run_environment =
        DeclarationEnvironment::new(owner, run.0, run.1, vec![]).expect("run env");
    let checked = build_environment
        .check_expression(&expression, &ValueType::Boolean, &point(), true)
        .expect("well-typed under the build environment");
    let clause = Clause::new(
        ClauseId::new("C-swap").expect("clause"),
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
        run_environment,
    )
    .expect_err("statement build refuses")
}

/// The debug text of the diagnostics `check_expression` refuses an expression with.
fn check_refusal(
    types: Vec<TypeDeclaration>,
    values: Vec<ValueDeclaration>,
    expression: &Expression,
) -> String {
    let owner = RequirementRef::new(
        PackageId::new("agent-ix/enum-test").expect("package"),
        RequirementId::new("REQ-check").expect("requirement"),
        RequirementRevision::new(1).expect("revision"),
    );
    let environment = DeclarationEnvironment::new(owner, types, values, vec![]).expect("env");
    let diagnostics = environment
        .check_expression(expression, &ValueType::Boolean, &point(), true)
        .expect_err("refused");
    format!("{diagnostics:?}")
}

fn role_statements(label: &str, expressions: Vec<Expression>) -> Vec<StatementInput> {
    try_statements(
        label,
        vec![enum_type("Role", ROLES)],
        vec![enum_value("role", "Role")],
        expressions,
    )
    .expect("statements")
}

/// `role = Admin or role = Auditor` plus one further assertion.
fn membership(label: &str, further: Expression) -> Vec<StatementInput> {
    role_statements(
        label,
        vec![
            or(is("role", "Role", "Admin"), is("role", "Role", "Auditor")),
            further,
        ],
    )
}

fn sat_fixture() -> Vec<StatementInput> {
    membership("sat", is_not("role", "Role", "Admin"))
}

fn unsat_fixture() -> Vec<StatementInput> {
    membership("unsat", is("role", "Role", "Guest"))
}

fn lower(statements: Vec<StatementInput>) -> QueryBundle {
    let request = AnalysisRequest::consistency(vec![], statements, vec![]).expect("request");
    lower_analysis_request(&request).expect("lower")
}

fn lower_error(statements: Vec<StatementInput>) -> Vec<LoweringError> {
    lower_boolean_statements(&statements, &[]).expect_err("refused")
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quire-enum-{label}-{}-{counter}",
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
    fake_script::install(&path, &body);
    let config = SolverConfig::new(engine, &path, AdapterLimits::default()).expect("config");
    execute_solver(query, &config, &CancellationToken::default())
}

/// FR-020-AC-6: the contract lists the enum constructs as exact and no construct is in both lists.
#[trace("TC-020", "FR-020-AC-6")]
#[test]
fn capability_contract_lists_enum_constructs_as_exact() {
    for exact in ["enum_literal", "enum_equality", "enum_inequality"] {
        assert!(CAPABILITY_CONTRACT.exact_constructs.contains(&exact));
    }
    assert!(!CAPABILITY_CONTRACT.unsupported_constructs.contains(&"enum"));
    for unsupported in ["collection", "collection_access", "text"] {
        assert!(CAPABILITY_CONTRACT
            .unsupported_constructs
            .contains(&unsupported));
    }
    for unsupported in CAPABILITY_CONTRACT.unsupported_constructs {
        assert!(!CAPABILITY_CONTRACT.exact_constructs.contains(unsupported));
    }
}

/// FR-020-AC-1, FR-020-AC-9 and NFR-020-AC-2: membership lowers to a golden datatype query under
/// `QF_UFDTLIA`, and repeated lowering is byte-identical.
#[trace("TC-020", "FR-020-AC-1", "FR-020-AC-9", "NFR-020-AC-2")]
#[test]
fn membership_fixture_lowers_to_the_golden_query() {
    let query = lower(sat_fixture());
    assert_eq!(query.profile(), "quire.smtlib2/v2");
    assert_eq!(query.logic(), "QF_UFDTLIA");
    assert_eq!(query.query(), include_str!("golden/enum-sat-v2.smt2"));
    assert_eq!(query.query(), lower(sat_fixture()).query());
    assert_eq!(query.enum_sorts().len(), 1);
    let sort = &query.enum_sorts()[0];
    assert_eq!(sort.name, "Role");
    let variants: Vec<_> = sort
        .constructors
        .iter()
        .map(|constructor| constructor.variant.as_str())
        .collect();
    assert_eq!(variants, ["Admin", "Auditor", "Guest", "User"]);
}

/// FR-020-AC-9: a query without a datatype stays `QF_LIA`.
#[trace("TC-020", "FR-020-AC-9")]
#[test]
fn query_without_datatype_declares_qf_lia() {
    let statements = try_statements(
        "plain",
        vec![],
        vec![input("flag", ValueType::Boolean)],
        vec![var("flag")],
    )
    .expect("statements");
    let query = lower(statements);
    assert_eq!(query.logic(), "QF_LIA");
    assert!(query.query().contains("(set-logic QF_LIA)"));
    assert!(!query.query().contains("declare-datatypes"));
    assert!(query.enum_sorts().is_empty());
}

/// FR-020-AC-1 (controlled responses): the unsatisfiable fixture agrees on unsat and its report
/// names `QF_UFDTLIA`.
#[trace("TC-020", "FR-020-AC-1", "FR-020-AC-9")]
#[test]
fn unsatisfiable_membership_agrees_and_reports_the_datatype_logic() {
    let query = lower(unsat_fixture());
    let z3 = response_record(&query, "unsat\n", SolverEngine::Z3);
    let cvc5 = response_record(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Refuted));
    let report = render_differential_report(&query, &run).expect("report");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
    assert_eq!(value["encodingProfile"], "quire.smtlib2/v2");
    assert_eq!(value["logic"], "QF_UFDTLIA");
    validate_report_document(&report).expect("document valid");
    validate_differential_report(&report, &query, &run).expect("valid");
}

/// FR-020-AC-1 and FR-021-AC-1: a satisfiable enum model is decided sat and its model verifies
/// in both engines' spellings; a model that violates an assertion stays unverified.
#[trace("TC-020", "FR-020-AC-1", "FR-020-AC-10", "FR-021-AC-1", "FR-021-AC-4")]
#[test]
fn satisfiable_membership_model_is_replayed() {
    let query = lower(sat_fixture());
    let sort = &query.enum_sorts()[0];
    let auditor = &sort.constructors[1].symbol;
    let variable = &query.variables()[0].symbol;
    let model = format!(
        "sat\n(model (define-fun {variable} () {} {auditor}))\n",
        sort.sort
    );
    let z3 = response_record(&query, &model, SolverEngine::Z3);
    let cvc5_model = format!(
        "sat\n((define-fun {variable} () {} {auditor}))\n",
        sort.sort
    );
    let cvc5 = response_record(&query, &cvc5_model, SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Satisfied));
    let conclusion = classify_analysis(&query, &z3);
    assert_eq!(conclusion.explanation(), ExplanationState::Verified);
    let model = conclusion.verified_model().expect("verified model");
    assert_eq!(
        model.datatype_values()[0].value(),
        &WitnessValue::Enum {
            enumeration: "Role".to_owned(),
            variant: "Auditor".to_owned()
        }
    );

    // `role != Admin` is violated by Admin: the solver "satisfied" it, the replay does not.
    let admin = &sort.constructors[0].symbol;
    let bad = format!(
        "sat\n(model (define-fun {variable} () {} {admin}))\n",
        sort.sort
    );
    let record = response_record(&query, &bad, SolverEngine::Z3);
    let conclusion = classify_analysis(&query, &record);
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    assert!(conclusion.verified_model().is_none());
    assert_eq!(conclusion.explanation(), ExplanationState::Incomplete);
    let failure = conclusion.replay_failure().expect("replay failure");
    assert_eq!(failure.reason(), ReplayFailureReason::AssertionFalse);
    let run = compare_solver_records(&query, &record, &record);
    assert_eq!(run.disposition(), DifferentialDisposition::Inconclusive);
}

/// FR-020-AC-9: a report naming v1, `QF_UF`, or a logic the feature set does not require is
/// refused, for a datatype query and for a datatype-free query.
#[trace("TC-007", "TC-020", "FR-020-AC-9", "FR-005-AC-6")]
#[test]
fn reports_naming_a_logic_the_feature_set_does_not_require_are_refused() {
    let plain = try_statements(
        "plain",
        vec![],
        vec![input("flag", ValueType::Boolean)],
        vec![var("flag")],
    )
    .expect("statements");
    let cases = [
        (
            lower(unsat_fixture()),
            [
                ("encodingProfile", "quire.smtlib2/v1"),
                ("logic", "QF_UF"),
                ("logic", "QF_LIA"),
                ("logic", "UFDTLIA"),
            ],
        ),
        (
            lower(plain),
            [
                ("encodingProfile", "quire.smtlib2/v1"),
                ("logic", "QF_UF"),
                ("logic", "QF_UFDTLIA"),
                ("logic", "LIA"),
            ],
        ),
    ];
    for (query, stale) in cases {
        let z3 = response_record(&query, "unsat\n", SolverEngine::Z3);
        let cvc5 = response_record(&query, "unsat\n", SolverEngine::Cvc5);
        let run = compare_solver_records(&query, &z3, &cvc5);
        let report = render_differential_report(&query, &run).expect("report");
        validate_report_document(&report).expect("accepted");
        let value: serde_json::Value = serde_json::from_slice(&report).expect("json");
        for (field, wrong) in stale {
            let mut mutated = value.clone();
            mutated[field] = serde_json::Value::String(wrong.to_owned());
            let bytes = serde_json::to_vec(&mutated).expect("bytes");
            assert!(validate_report_document(&bytes).is_err(), "{field} {wrong}");
            assert!(validate_differential_report(&bytes, &query, &run).is_err());
        }
    }
}

/// FR-020-AC-8: a membership disjunction with a repeated literal is lowered as written.
#[trace("TC-020", "FR-020-AC-8")]
#[test]
fn repeated_membership_literal_is_lowered_as_written() {
    let statements = role_statements(
        "repeat",
        vec![or(is("role", "Role", "Admin"), is("role", "Role", "Admin"))],
    );
    let query = lower_boolean_statements(&statements, &[]).expect("lower");
    let admin = &query.enum_sorts()[0].constructors[0].symbol;
    let variable = &query.variables()[0].symbol;
    let equality = format!("(= {variable} {admin})");
    assert!(query
        .query()
        .contains(&format!("(or {equality} {equality})")));
}

/// FR-020-AC-1: enum inequality lowers to `distinct`.
#[trace("TC-020", "FR-020-AC-1")]
#[test]
fn enum_inequality_lowers_to_distinct() {
    let query = lower_boolean_statements(
        &role_statements("ne", vec![is_not("role", "Role", "Guest")]),
        &[],
    )
    .expect("lower");
    let guest = &query.enum_sorts()[0].constructors[2].symbol;
    assert!(query.query().contains("(distinct "));
    assert!(query.query().contains(guest));
}

/// FR-020-AC-2: an unresolved variant or enumeration is refused at statement build as
/// `InvalidStatement` before lowering, and an unresolved value type cannot form an environment.
#[trace("TC-020", "FR-020-AC-2")]
#[test]
fn unresolved_names_are_refused_before_lowering() {
    let with_root = vec![enum_type("Role", &["Admin", "Root"])];
    let without_root = vec![enum_type("Role", &["Admin"])];
    let errors = statement_build_errors(
        (with_root, vec![enum_value("role", "Role")]),
        (without_root, vec![enum_value("role", "Role")]),
        is("role", "Role", "Root"),
    );
    assert!(errors.iter().all(|error| {
        error.code() == LoweringErrorCode::InvalidStatement
            && error.path() == "statement.expression"
    }));
    let errors = statement_build_errors(
        (vec![enum_type("Missing", &["A"])], vec![]),
        (vec![], vec![]),
        compare(
            ComparisonOperator::Equal,
            variant("Missing", "A"),
            variant("Missing", "A"),
        ),
    );
    assert!(errors.iter().all(|error| {
        error.code() == LoweringErrorCode::InvalidStatement
            && error.path() == "statement.expression"
    }));
    let owner = RequirementRef::new(
        PackageId::new("agent-ix/enum-test").expect("package"),
        RequirementId::new("REQ-orphan").expect("requirement"),
        RequirementRevision::new(1).expect("revision"),
    );
    assert!(DeclarationEnvironment::new(
        owner,
        vec![enum_type("Role", ROLES)],
        vec![enum_value("role", "Missing")],
        vec![]
    )
    .is_err());
}

/// FR-020-AC-2: only referenced enums are declared, so an unused declaration changes nothing.
#[trace("TC-020", "FR-020-AC-2")]
#[test]
fn unused_declarations_do_not_change_the_query() {
    let used = role_statements("unused", vec![is("role", "Role", "Admin")]);
    let with_extra = try_statements(
        "unused",
        vec![
            enum_type("Role", ROLES),
            enum_type("Level", &["High", "Low"]),
        ],
        vec![enum_value("role", "Role"), enum_value("level", "Level")],
        vec![is("role", "Role", "Admin")],
    )
    .expect("statements");
    let plain = lower_boolean_statements(&used, &[]).expect("lower");
    let extra = lower_boolean_statements(&with_extra, &[]).expect("lower");
    assert_eq!(plain.query(), extra.query());
}

/// FR-020-AC-5: enum ordering and a mixed-sort comparison are refused as ill-typed by the
/// statement-build check, and an unsupported sort is refused by lowering.
#[trace("TC-020", "FR-020-AC-5")]
#[test]
fn ordering_and_mixed_sorts_are_refused_at_statement_build() {
    let types = || vec![enum_type("Role", ROLES)];
    let values = || {
        vec![
            enum_value("role", "Role"),
            input("flag", ValueType::Boolean),
        ]
    };
    let ordering = compare(
        ComparisonOperator::Less,
        var("role"),
        variant("Role", "Guest"),
    );
    let mixed = compare(
        ComparisonOperator::Equal,
        var("flag"),
        variant("Role", "Guest"),
    );
    for expression in [&ordering, &mixed] {
        assert!(check_refusal(types(), values(), expression).contains("IllTypedExpression"));
    }
    let statements = try_statements(
        "text",
        vec![],
        vec![input("note", ValueType::Text)],
        vec![compare(
            ComparisonOperator::Equal,
            var("note"),
            node(ExpressionKind::TextLiteral {
                value: "x".to_owned(),
            }),
        )],
    )
    .expect("statements");
    let errors = lower_error(statements);
    assert_eq!(errors[0].code(), LoweringErrorCode::UnsupportedConstruct);
}

/// FR-020-AC-7 and NFR-020-AC-2: permuting type, value, variant and statement order preserves the
/// query, and two enums with identical variant names are distinct sorts.
#[trace("TC-020", "FR-020-AC-7", "NFR-020-AC-2")]
#[test]
fn permutation_preserves_bytes_and_equal_variants_stay_distinct() {
    let build = |reversed: bool| {
        let (mut roles, mut levels) = (vec!["Admin", "User"], vec!["Admin", "User"]);
        let mut types = vec![enum_type("Role", &roles), enum_type("Level", &levels)];
        let mut values = vec![enum_value("role", "Role"), enum_value("level", "Level")];
        let expressions = vec![is("role", "Role", "Admin"), is("level", "Level", "User")];
        if reversed {
            roles.reverse();
            levels.reverse();
            types = vec![enum_type("Level", &levels), enum_type("Role", &roles)];
            values.reverse();
        }
        let mut statements =
            try_statements("perm", types, values, expressions).expect("statements");
        if reversed {
            statements.reverse();
        }
        lower(statements)
    };
    let forward = build(false);
    let backward = build(true);
    assert_eq!(forward.query(), backward.query());
    assert_eq!(forward.enum_sorts(), backward.enum_sorts());
    let sorts = forward.enum_sorts();
    assert_eq!(sorts.len(), 2);
    assert_ne!(sorts[0].sort, sorts[1].sort);
    assert_ne!(
        sorts[0].constructors[0].symbol,
        sorts[1].constructors[0].symbol
    );
}

/// NFR-020-AC-1: 256 variants are accepted and 257 are refused as a resource limit.
#[trace("TC-020", "NFR-020-AC-1")]
#[test]
fn variant_bound_is_exact() {
    let at = |count: usize| {
        let names: Vec<String> = (0..count).map(|index| format!("V{index}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        try_statements(
            "variants",
            vec![enum_type("Wide", &names)],
            vec![enum_value("wide", "Wide")],
            vec![is("wide", "Wide", "V0")],
        )
        .expect("statements")
    };
    let query = lower_boolean_statements(&at(256), &[]).expect("256 accepted");
    assert_eq!(query.enum_sorts()[0].constructors.len(), 256);
    let errors = lower_error(at(257));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);
}

/// NFR-020-AC-1: 256 datatypes per query are accepted and 257 are refused as a resource limit.
#[trace("TC-020", "NFR-020-AC-1")]
#[test]
fn datatype_count_bound_is_exact() {
    let at = |count: usize| {
        let types = (0..count)
            .map(|index| enum_type(&format!("E{index}"), &["A", "B"]))
            .collect();
        let values = (0..count)
            .map(|index| enum_value(&format!("e{index}"), &format!("E{index}")))
            .collect();
        let expressions = (0..count)
            .map(|index| is(&format!("e{index}"), &format!("E{index}"), "A"))
            .collect();
        try_statements("count", types, values, expressions).expect("statements")
    };
    let query = lower_boolean_statements(&at(256), &[]).expect("256 accepted");
    assert_eq!(query.enum_sorts().len(), 256);
    let errors = lower_error(at(257));
    assert_eq!(errors[0].code(), LoweringErrorCode::ResourceLimit);
}

/// FR-020-AC-1: real solvers decide both membership fixtures; run only where installed.
#[trace("TC-020", "FR-020-AC-1")]
#[test]
#[ignore = "requires official Z3 and cvc5 executables at QUIRE_Z3 and QUIRE_CVC5"]
fn real_z3_cvc5_decide_membership_fixtures() {
    let z3_path = PathBuf::from(std::env::var_os("QUIRE_Z3").expect("QUIRE_Z3 is required"));
    let cvc5_path = PathBuf::from(std::env::var_os("QUIRE_CVC5").expect("QUIRE_CVC5 is required"));
    let z3 = SolverConfig::new(SolverEngine::Z3, z3_path, AdapterLimits::default()).expect("z3");
    let cvc5 =
        SolverConfig::new(SolverEngine::Cvc5, cvc5_path, AdapterLimits::default()).expect("cvc5");
    let sat = lower(sat_fixture());
    let token = CancellationToken::default();
    for config in [&z3, &cvc5] {
        let record = execute_solver(&sat, config, &token);
        assert_eq!(record.outcome(), SolverOutcome::Sat);
    }
    let run = quire_analyze::execute_differential(&sat, &z3, &cvc5, &token).expect("run");
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    let unsat = lower(unsat_fixture());
    let run = quire_analyze::execute_differential(&unsat, &z3, &cvc5, &token).expect("run");
    assert_eq!(run.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(run.agreed_status(), Some(AnalysisStatus::Refuted));
}

/// FR-021-AC-1: a query that declares an enum but holds only enum literals replays its Boolean
/// variable and the literal comparison; a false model and a model without the variable do not
/// verify.
#[trace("TC-020", "FR-020-AC-10", "FR-021-AC-1", "FR-021-AC-5")]
#[test]
fn literal_only_enum_query_is_replayed() {
    let statements = try_statements(
        "literals",
        vec![enum_type("Role", ROLES)],
        vec![input("flag", ValueType::Boolean)],
        vec![or(
            var("flag"),
            compare(
                ComparisonOperator::Equal,
                variant("Role", "Admin"),
                variant("Role", "Guest"),
            ),
        )],
    )
    .expect("statements");
    let query = lower(statements);
    assert_eq!(query.logic(), "QF_UFDTLIA");
    let flag = &query.variables()[0].symbol;
    let verdict = |model: &str| {
        let record = response_record(&query, model, SolverEngine::Z3);
        classify_analysis(&query, &record)
    };
    let verified = verdict(&format!("sat\n(model (define-fun {flag} () Bool true))\n"));
    assert_eq!(verified.explanation(), ExplanationState::Verified);
    let falsified = verdict(&format!("sat\n(model (define-fun {flag} () Bool false))\n"));
    assert_eq!(
        falsified.replay_failure().expect("failure").reason(),
        ReplayFailureReason::AssertionFalse
    );
    let omitted = verdict("sat\n(model )\n");
    assert_eq!(
        omitted.replay_failure().expect("failure").reason(),
        ReplayFailureReason::SymbolMissing
    );
}
