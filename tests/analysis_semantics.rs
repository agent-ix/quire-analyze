//! Requirement-tagged tests for Boolean analysis conclusions (issue #4).

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use quire_analyze::{
    classify_analysis, execute_solver, lower_analysis_request, AdapterLimits, AnalysisKind,
    AnalysisRequest, AnalysisRequestErrorCode, AnalysisStatus, AssertionPolarity, BindingGroup,
    BindingMember, CancellationToken, ExplanationState, ModelPurpose, QueryBundle, SolverConfig,
    SolverDigest, SolverEngine, SolverOutcome, SolverPin, StatementInput, StatementRole,
};
use quire_contract_ir::{
    AnchorName, BooleanOperator, Clause, ClauseId, ClauseKind, ContractPackage,
    DeclarationEnvironment, ExecutionPoint, Expression, ExpressionKind, PackageId, Requirement,
    RequirementId, RequirementRef, RequirementRevision, SchemaVersion, SourceDocumentId,
    SourceIdentity, SourceLocation, SourceRevision, SourceSpan, StateObservation, SymbolName,
    TypedExpression, ValueDeclaration, ValueDeclarationKind, ValueType,
};
use sha2::{Digest as _, Sha256};

const VERSION: &str = "fake-analysis-solver 1.0";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quire-analysis-{label}-{}-{counter}",
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

fn span(offset: u64) -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("analysis-test").expect("source"),
        SourceRevision::new(1).expect("revision"),
    );
    SourceSpan::new(
        SourceLocation::new(source.clone(), 1, offset as u32 + 1, offset).expect("start"),
        SourceLocation::new(source, 1, offset as u32 + 2, offset + 1).expect("end"),
    )
    .expect("span")
}

fn point() -> ExecutionPoint {
    ExecutionPoint::Pre {
        operation: AnchorName::new("analyze").expect("anchor"),
    }
}

fn value(name: &str) -> Expression {
    Expression::new(
        ExpressionKind::ValueReference {
            name: SymbolName::new(name).expect("symbol"),
            observation: StateObservation::Current,
        },
        span(0),
    )
}

fn literal(value: bool) -> Expression {
    Expression::new(ExpressionKind::BooleanLiteral { value }, span(0))
}

fn not(expression: Expression) -> Expression {
    Expression::new(
        ExpressionKind::BooleanNot {
            operand: Box::new(expression),
        },
        span(0),
    )
}

fn and(left: Expression, right: Expression) -> Expression {
    Expression::new(
        ExpressionKind::Boolean {
            operator: BooleanOperator::TotalAnd,
            left: Box::new(left),
            right: Box::new(right),
        },
        span(0),
    )
}

fn statement(label: &str, expression: Expression, variables: &[&str]) -> StatementInput {
    statement_with_environment(label, expression, variables).0
}

fn statement_with_environment(
    label: &str,
    expression: Expression,
    variables: &[&str],
) -> (StatementInput, DeclarationEnvironment) {
    let package_id = PackageId::new("agent-ix/analysis-test").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-{label}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let declarations = variables
        .iter()
        .map(|name| {
            ValueDeclaration::new(
                SymbolName::new(*name).expect("symbol"),
                ValueDeclarationKind::Input,
                ValueType::Boolean,
                span(0),
            )
        })
        .collect();
    let environment =
        DeclarationEnvironment::new(owner, vec![], declarations, vec![]).expect("environment");
    let checked = environment
        .check_expression(&expression, &ValueType::Boolean, &point(), true)
        .expect("typed expression");
    let clause = Clause::new(
        ClauseId::new(format!("C-{label}")).expect("clause"),
        ClauseKind::Assertion,
        Some(point()),
        span(label.len() as u64),
        checked,
    )
    .expect("clause");
    let requirement = Requirement::<TypedExpression>::new(
        &package_id,
        requirement_id,
        revision,
        span(0),
        vec![clause],
    )
    .expect("requirement");
    let package = ContractPackage::new(
        package_id,
        SchemaVersion::V1_0,
        span(0).source().clone(),
        vec![requirement],
    )
    .expect("package");
    let statement = StatementInput::from_clause(
        &package,
        &package.requirements()[0],
        &package.requirements()[0].clauses()[0],
        environment.clone(),
    )
    .expect("statement");
    (statement, environment)
}

fn digest(path: &Path) -> SolverDigest {
    SolverDigest::from_bytes(Sha256::digest(fs::read(path).expect("script")).into())
}

fn write_solver(directory: &Path, response: &str, slow: bool) -> PathBuf {
    let path = directory.join("solver");
    let action = if slow {
        "/bin/sleep 30".to_owned()
    } else {
        format!("/bin/cat >/dev/null; printf '%s' '{response}'")
    };
    let body = format!(
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then printf '{VERSION}\\n'; exit 0; fi\n{action}\n"
    );
    fs::write(&path, body).expect("write solver");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions).expect("executable");
    path
}

fn execute_response(query: &QueryBundle, response: &str) -> quire_analyze::SolverRecord {
    let directory = TempDirectory::new("response");
    let executable = write_solver(directory.path(), response, false);
    let config = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        SolverPin::new(VERSION, digest(&executable)),
        AdapterLimits::default(),
    )
    .expect("config");
    execute_solver(query, &config, &CancellationToken::default())
}

fn model_response(query: &QueryBundle, value: bool) -> String {
    let definitions = query
        .variables()
        .iter()
        .map(|variable| {
            format!(
                "(define-fun {} () Bool {})",
                variable.symbol,
                if value { "true" } else { "false" }
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("sat\n(model {definitions})\n")
}

fn satisfiable(formula: impl Fn(bool) -> bool) -> bool {
    [false, true].into_iter().any(formula)
}

fn evaluate_and(left: bool, right: bool) -> bool {
    left && right
}

fn evaluate_not(value: bool) -> bool {
    !value
}

/// FR-001-AC-1/4: constructors make every role and polarity explicit and reject ambiguity.
/// Trace: TC-001, TC-003, FR-001-AC-1, FR-001-AC-4
#[test]
fn dedicated_requests_are_role_exact_deterministic_and_fail_closed() {
    let assumption = statement("assumption", literal(true), &[]);
    let selected = statement("selected", literal(true), &[]);
    let selected_second = statement("selected-second", literal(false), &[]);
    let consistency = AnalysisRequest::consistency(
        vec![assumption.clone()],
        vec![selected.clone(), selected_second.clone()],
        vec![],
    )
    .expect("consistency");
    let repeated = AnalysisRequest::consistency(
        vec![assumption.clone()],
        vec![selected_second, selected.clone()],
        vec![],
    )
    .expect("consistency");
    let first = lower_analysis_request(&consistency).expect("lower");
    let second = lower_analysis_request(&repeated).expect("lower");
    assert_eq!(first.query(), second.query());
    assert_eq!(
        first.analysis_request_digest(),
        second.analysis_request_digest()
    );
    assert_eq!(first.analysis_kind(), Some(AnalysisKind::Consistency));
    assert!(first.assertions().iter().any(|map| {
        map.role == StatementRole::Assumption && map.polarity == AssertionPolarity::Positive
    }));
    assert!(first.assertions().iter().any(|map| {
        map.role == StatementRole::Selected && map.polarity == AssertionPolarity::Positive
    }));

    let implication = AnalysisRequest::implication(
        vec![],
        vec![statement("antecedent", literal(true), &[])],
        statement("consequent", literal(false), &[]),
        vec![],
    )
    .expect("implication");
    let query = lower_analysis_request(&implication).expect("lower");
    assert!(query.assertions().iter().any(|map| {
        map.role == StatementRole::Consequent && map.polarity == AssertionPolarity::Negated
    }));
    assert!(query.query().contains("(not false)"));

    let role_first = statement("role-first", literal(true), &[]);
    let role_second = statement("role-second", literal(true), &[]);
    let as_consistency = lower_analysis_request(
        &AnalysisRequest::consistency(
            vec![],
            vec![role_first.clone(), role_second.clone()],
            vec![],
        )
        .expect("consistency roles"),
    )
    .expect("lower consistency roles");
    let as_contradiction = lower_analysis_request(
        &AnalysisRequest::contradiction(vec![], vec![role_first], vec![role_second], vec![])
            .expect("contradiction roles"),
    )
    .expect("lower contradiction roles");
    assert_ne!(
        as_consistency.analysis_request_digest(),
        as_contradiction.analysis_request_digest()
    );
    assert_ne!(
        as_consistency.query_digest(),
        as_contradiction.query_digest()
    );

    assert_eq!(
        AnalysisRequest::consistency(vec![], vec![], vec![]).expect_err("empty selected")[0].code(),
        AnalysisRequestErrorCode::EmptyGroup
    );
    let empty_contradiction =
        AnalysisRequest::contradiction(vec![], vec![], vec![], vec![]).expect_err("empty sides");
    assert_eq!(empty_contradiction.len(), 2);
    assert!(empty_contradiction
        .iter()
        .all(|error| error.code() == AnalysisRequestErrorCode::EmptyGroup));
    assert_eq!(
        AnalysisRequest::redundancy(vec![], vec![], selected.clone(), vec![])
            .expect_err("empty peers")[0]
            .code(),
        AnalysisRequestErrorCode::EmptyGroup
    );
    assert_eq!(
        AnalysisRequest::consistency(vec![selected.clone()], vec![selected], vec![])
            .expect_err("duplicate role")[0]
            .code(),
        AnalysisRequestErrorCode::DuplicateStatement
    );
}

struct TruthCase {
    kind: AnalysisKind,
    request: AnalysisRequest,
    independently_sat: bool,
    expected: AnalysisStatus,
}

/// FR-001-AC-1 and FR-004-AC-1/5: all ten classifications match independent finite evaluation.
/// Trace: TC-001, TC-005, FR-001-AC-1, FR-004-AC-1, FR-004-AC-5
#[test]
fn all_ten_truth_table_cells_match_independent_finite_models() {
    let cases = vec![
        TruthCase {
            kind: AnalysisKind::Consistency,
            request: AnalysisRequest::consistency(
                vec![],
                vec![statement("consistency-sat", value("x"), &["x"])],
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|x| x),
            expected: AnalysisStatus::Satisfied,
        },
        TruthCase {
            kind: AnalysisKind::Consistency,
            request: AnalysisRequest::consistency(
                vec![],
                vec![statement(
                    "consistency-unsat",
                    and(value("x"), not(value("x"))),
                    &["x"],
                )],
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|x| evaluate_and(x, evaluate_not(x))),
            expected: AnalysisStatus::Refuted,
        },
        TruthCase {
            kind: AnalysisKind::Contradiction,
            request: AnalysisRequest::contradiction(
                vec![],
                vec![statement("contradiction-unsat-left", literal(true), &[])],
                vec![statement("contradiction-unsat-right", literal(false), &[])],
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|_| evaluate_and(true, false)),
            expected: AnalysisStatus::Satisfied,
        },
        TruthCase {
            kind: AnalysisKind::Contradiction,
            request: AnalysisRequest::contradiction(
                vec![],
                vec![statement("contradiction-sat-left", literal(true), &[])],
                vec![statement("contradiction-sat-right", literal(true), &[])],
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|_| true),
            expected: AnalysisStatus::Refuted,
        },
        TruthCase {
            kind: AnalysisKind::Implication,
            request: AnalysisRequest::implication(
                vec![],
                vec![statement("implication-unsat-left", literal(true), &[])],
                statement("implication-unsat-right", literal(true), &[]),
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|_| evaluate_and(true, evaluate_not(true))),
            expected: AnalysisStatus::Satisfied,
        },
        TruthCase {
            kind: AnalysisKind::Implication,
            request: AnalysisRequest::implication(
                vec![],
                vec![statement("implication-sat-left", literal(true), &[])],
                statement("implication-sat-right", literal(false), &[]),
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|_| true),
            expected: AnalysisStatus::Refuted,
        },
        TruthCase {
            kind: AnalysisKind::Redundancy,
            request: AnalysisRequest::redundancy(
                vec![],
                vec![statement("redundancy-unsat-peer", literal(true), &[])],
                statement("redundancy-unsat-candidate", literal(true), &[]),
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|_| evaluate_and(true, evaluate_not(true))),
            expected: AnalysisStatus::Satisfied,
        },
        TruthCase {
            kind: AnalysisKind::Redundancy,
            request: AnalysisRequest::redundancy(
                vec![],
                vec![statement("redundancy-sat-peer", literal(true), &[])],
                statement("redundancy-sat-candidate", literal(false), &[]),
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|_| true),
            expected: AnalysisStatus::Refuted,
        },
        TruthCase {
            kind: AnalysisKind::DeadAntecedent,
            request: AnalysisRequest::dead_antecedent(
                vec![],
                statement("dead-unsat", and(value("x"), not(value("x"))), &["x"]),
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|x| evaluate_and(x, evaluate_not(x))),
            expected: AnalysisStatus::Satisfied,
        },
        TruthCase {
            kind: AnalysisKind::DeadAntecedent,
            request: AnalysisRequest::dead_antecedent(
                vec![],
                statement("dead-sat", value("x"), &["x"]),
                vec![],
            )
            .expect("request"),
            independently_sat: satisfiable(|x| x),
            expected: AnalysisStatus::Refuted,
        },
    ];

    assert_eq!(cases.len(), 10);
    for case in cases {
        let query = lower_analysis_request(&case.request).expect("lower");
        assert_eq!(query.analysis_kind(), Some(case.kind));
        let response = if case.independently_sat {
            model_response(&query, true)
        } else {
            "unsat\n".to_owned()
        };
        let record = execute_response(&query, &response);
        assert_eq!(
            record.outcome(),
            if case.independently_sat {
                SolverOutcome::Sat
            } else {
                SolverOutcome::Unsat
            }
        );
        let conclusion = classify_analysis(&query, &record);
        assert_eq!(conclusion.status(), case.expected, "kind {:?}", case.kind);
        assert!(conclusion.is_conclusive());
        if case.independently_sat {
            assert_eq!(conclusion.explanation(), ExplanationState::Verified);
        } else {
            assert_eq!(conclusion.explanation(), ExplanationState::NotApplicable);
        }
    }
}

/// FR-004-AC-2/3/4: only complete replaying models become source-mapped verified evidence.
/// Trace: TC-004, TC-006, TC-007, FR-004-AC-2, FR-004-AC-3, FR-004-AC-4
#[test]
fn model_decode_mapping_and_replay_are_fail_closed() {
    let request = AnalysisRequest::consistency(
        vec![],
        vec![statement("mapped-model", value("ready"), &["ready"])],
        vec![],
    )
    .expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let symbol = query.variables()[0].symbol.clone();
    let valid = execute_response(&query, &model_response(&query, true));
    let conclusion = classify_analysis(&query, &valid);
    assert_eq!(conclusion.status(), AnalysisStatus::Satisfied);
    assert_eq!(conclusion.explanation(), ExplanationState::Verified);
    let model = conclusion.verified_model().expect("verified model");
    assert_eq!(model.purpose(), ModelPurpose::Shared);
    assert_eq!(model.values().len(), 1);
    assert_eq!(model.values()[0].symbol(), symbol);
    assert!(model.values()[0].value());
    assert_eq!(model.values()[0].origins().len(), 1);
    assert!(model.values()[0].origins()[0].contains("REQ-mapped-model"));
    assert_eq!(model.replayed_assertions().len(), 1);
    assert_eq!(conclusion.assertions()[0].role, StatementRole::Selected);
    assert_eq!(conclusion.query_digest(), query.query_digest());
    assert_eq!(conclusion.request_digest(), query.analysis_request_digest());
    assert_eq!(conclusion.binding_set_digest(), query.binding_set_digest());
    assert_eq!(
        conclusion.analysis_model_profile(),
        "quire.analysis-model/v1"
    );
    assert_eq!(conclusion.encoding_profile(), "quire.smtlib2/v1");
    assert_eq!(conclusion.logic(), "QF_UF");
    assert_eq!(
        conclusion.solver().query_digest(),
        query.query_digest().to_string()
    );

    for response in [
        "sat\n(model)\n".to_owned(),
        format!(
            "sat\n(model (define-fun {symbol} () Bool true) (define-fun {symbol} () Bool false))\n"
        ),
        "sat\n(model (define-fun extra () Bool true))\n".to_owned(),
        format!("sat\n(model (define-fun {symbol} () Bool false))\n"),
    ] {
        let record = execute_response(&query, &response);
        let conclusion = classify_analysis(&query, &record);
        assert_eq!(
            conclusion.status(),
            AnalysisStatus::Satisfied,
            "adapter outcome {:?} for {response:?}",
            record.outcome()
        );
        assert_eq!(conclusion.explanation(), ExplanationState::Incomplete);
        assert!(conclusion.verified_model().is_none());
        assert!(conclusion.diagnostic().is_some());
    }

    let (first, first_environment) =
        statement_with_environment("bound-first", value("ready"), &["ready"]);
    let (second, second_environment) =
        statement_with_environment("bound-second", value("ready"), &["ready"]);
    let ready = SymbolName::new("ready").expect("symbol");
    let binding = BindingGroup::new(
        "shared-ready",
        vec![
            BindingMember::from_declaration(
                &first_environment,
                &ready,
                StateObservation::Current,
                &point(),
            )
            .expect("first member"),
            BindingMember::from_declaration(
                &second_environment,
                &ready,
                StateObservation::Current,
                &point(),
            )
            .expect("second member"),
        ],
    )
    .expect("binding");
    let bound_request =
        AnalysisRequest::consistency(vec![], vec![first, second], vec![binding]).expect("request");
    let bound_query = lower_analysis_request(&bound_request).expect("lower");
    assert_eq!(bound_query.variables().len(), 1);
    let bound_record = execute_response(&bound_query, &model_response(&bound_query, true));
    let bound = classify_analysis(&bound_query, &bound_record);
    let value = &bound
        .verified_model()
        .expect("verified bound model")
        .values()[0];
    assert_eq!(value.origins().len(), 2);
    assert_eq!(value.binding_group(), Some("shared-ready"));
}

/// FR-004-AC-4/5: unknown, cancellation, and tool failures never become success.
/// Trace: TC-005, TC-007, FR-004-AC-4, FR-004-AC-5
#[test]
fn nonconclusive_adapter_states_remain_nonconclusive() {
    let request =
        AnalysisRequest::dead_antecedent(vec![], statement("status", literal(false), &[]), vec![])
            .expect("request");
    let query = lower_analysis_request(&request).expect("lower");

    let unknown = classify_analysis(&query, &execute_response(&query, "unknown\n"));
    assert_eq!(unknown.status(), AnalysisStatus::Unknown);
    assert!(!unknown.is_conclusive());

    let malformed = classify_analysis(&query, &execute_response(&query, "malformed\n"));
    assert_eq!(malformed.status(), AnalysisStatus::ToolError);
    assert!(!malformed.is_conclusive());

    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let directory = TempDirectory::new("cancelled");
    let executable = write_solver(directory.path(), "", true);
    let config = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        SolverPin::new(VERSION, digest(&executable)),
        AdapterLimits::default(),
    )
    .expect("config");
    let cancelled_record = execute_solver(&query, &config, &cancellation);
    let cancelled = classify_analysis(&query, &cancelled_record);
    assert_eq!(cancelled.status(), AnalysisStatus::Timeout);
    assert_eq!(cancelled.solver().outcome(), SolverOutcome::Cancelled);
    assert!(!cancelled.is_conclusive());

    let other_request = AnalysisRequest::consistency(
        vec![],
        vec![statement("other-query", literal(true), &[])],
        vec![],
    )
    .expect("request");
    let other_query = lower_analysis_request(&other_request).expect("lower");
    let mismatched_record = execute_response(&other_query, "unsat\n");
    let mismatched = classify_analysis(&query, &mismatched_record);
    assert_eq!(mismatched.status(), AnalysisStatus::ToolError);
    assert!(mismatched
        .diagnostic()
        .expect("diagnostic")
        .contains("identity"));
}
