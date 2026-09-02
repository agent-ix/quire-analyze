//! Requirement-tagged tests for Boolean analysis conclusions (issue #4).

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier,
    },
    thread,
};

use quire_analyze::{
    classify_analysis, compare_solver_records, execute_differential, execute_solver,
    lower_analysis_request, publish_report_new, render_differential_report,
    render_differential_summary, validate_differential_report, validate_report_document,
    AdapterLimits, AnalysisKind, AnalysisRequest, AnalysisRequestErrorCode, AnalysisStatus,
    AssertionPolarity, BindingGroup, BindingMember, CancellationToken, DifferentialDisposition,
    ExplanationState, ModelPurpose, QueryBundle, SolverConfig, SolverDigest, SolverEngine,
    SolverOutcome, SolverPin, StatementInput, StatementRole,
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

fn reseal_report(mut value: serde_json::Value) -> Vec<u8> {
    value
        .as_object_mut()
        .expect("report object")
        .remove("reportDigest");
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&value).expect("canonical payload"))
    );
    value
        .as_object_mut()
        .expect("report object")
        .insert("reportDigest".to_owned(), serde_json::Value::String(digest));
    serde_json::to_vec(&value).expect("sealed report")
}

fn write_solver(directory: &Path, response: &str, slow: bool) -> PathBuf {
    let path = directory.join("solver");
    let action = if slow {
        "/bin/sleep 30".to_owned()
    } else {
        format!("/bin/cat >/dev/null; printf '%s' '{response}'")
    };
    let body = format!(
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ] || [ \"$1\" = \"--version\" ]; then printf '{VERSION}\\n'; exit 0; fi\n{action}\n"
    );
    fs::write(&path, body).expect("write solver");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions).expect("executable");
    path
}

fn execute_response(query: &QueryBundle, response: &str) -> quire_analyze::SolverRecord {
    execute_engine_response(query, response, SolverEngine::Z3)
}

fn execute_engine_response(
    query: &QueryBundle,
    response: &str,
    engine: SolverEngine,
) -> quire_analyze::SolverRecord {
    let directory = TempDirectory::new("response");
    let executable = write_solver(directory.path(), response, false);
    let config = SolverConfig::new(
        engine,
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

/// FR-005-AC-3: agreement, disagreement, unavailability, and incomplete evidence stay distinct.
/// Trace: TC-006, FR-005-AC-3, StR-001-VC-2
#[test]
fn differential_disposition_requires_two_verified_conclusive_results() {
    let request = AnalysisRequest::consistency(
        vec![],
        vec![statement("differential", value("x"), &["x"])],
        vec![],
    )
    .expect("request");
    let query = lower_analysis_request(&request).expect("lower");

    let z3_unsat = execute_engine_response(&query, "unsat\n", SolverEngine::Z3);
    let cvc5_unsat = execute_engine_response(&query, "unsat\n", SolverEngine::Cvc5);
    let agreement = compare_solver_records(&query, &z3_unsat, &cvc5_unsat);
    assert_eq!(agreement.disposition(), DifferentialDisposition::Agreement);
    assert_eq!(agreement.agreed_status(), Some(AnalysisStatus::Refuted));
    assert!(agreement.is_conclusive());

    let z3_sat = execute_engine_response(&query, &model_response(&query, true), SolverEngine::Z3);
    let disagreement = compare_solver_records(&query, &z3_sat, &cvc5_unsat);
    assert_eq!(
        disagreement.disposition(),
        DifferentialDisposition::Disagreement
    );
    assert!(!disagreement.is_conclusive());
    assert_eq!(disagreement.z3().solver().outcome(), SolverOutcome::Sat);
    assert_eq!(disagreement.cvc5().solver().outcome(), SolverOutcome::Unsat);

    let z3_incomplete = execute_engine_response(&query, "sat\n", SolverEngine::Z3);
    let cvc5_incomplete = execute_engine_response(&query, "sat\n", SolverEngine::Cvc5);
    let inconclusive = compare_solver_records(&query, &z3_incomplete, &cvc5_incomplete);
    assert_eq!(
        inconclusive.disposition(),
        DifferentialDisposition::Inconclusive
    );

    let directory = TempDirectory::new("differential-unavailable");
    let executable = write_solver(directory.path(), "unsat\n", false);
    let z3_config = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        SolverPin::new(VERSION, digest(&executable)),
        AdapterLimits::default(),
    )
    .expect("z3 config");
    let missing = directory.path().join("missing-cvc5");
    let cvc5_config = SolverConfig::new(
        SolverEngine::Cvc5,
        &missing,
        SolverPin::new(VERSION, SolverDigest::from_bytes([0; 32])),
        AdapterLimits::default(),
    )
    .expect("cvc5 config");
    let unavailable = execute_differential(
        &query,
        &z3_config,
        &cvc5_config,
        &CancellationToken::default(),
    )
    .expect("differential");
    assert_eq!(
        unavailable.disposition(),
        DifferentialDisposition::Unavailable
    );
    assert_eq!(
        unavailable.cvc5().solver().outcome(),
        SolverOutcome::MissingExecutable
    );
    assert!(execute_differential(
        &query,
        &cvc5_config,
        &z3_config,
        &CancellationToken::default()
    )
    .is_err());
}

/// FR-005-AC-3: official pinned engines agree on independently selected sat and unsat cases.
/// Trace: TC-006, FR-005-AC-3, MP-001-M-06
#[test]
#[ignore = "requires the pinned official Z3 and cvc5 release assets"]
fn official_z3_cvc5_differential_corpus_agrees() {
    let z3_path = PathBuf::from(std::env::var_os("QUIRE_Z3").expect("QUIRE_Z3 is required"));
    let cvc5_path = PathBuf::from(std::env::var_os("QUIRE_CVC5").expect("QUIRE_CVC5 is required"));
    let z3_digest = digest(&z3_path);
    let cvc5_digest = digest(&cvc5_path);
    assert_eq!(
        z3_digest.to_string(),
        "54bae839dd54e262edac6f755fc99659ce2a121301faff20a3e3b94478dcead0"
    );
    assert_eq!(
        cvc5_digest.to_string(),
        "7562a8b0b835e3eaad5f1a7b4616cd762350cf567b6be03d7e8ee24fa5ced5ee"
    );
    let z3_version = String::from_utf8(
        Command::new(&z3_path)
            .arg("-version")
            .output()
            .expect("Z3 version probe")
            .stdout,
    )
    .expect("Z3 version UTF-8")
    .trim()
    .to_owned();
    let cvc5_version = String::from_utf8(
        Command::new(&cvc5_path)
            .arg("--version")
            .output()
            .expect("cvc5 version probe")
            .stdout,
    )
    .expect("cvc5 version UTF-8")
    .trim()
    .to_owned();
    assert_eq!(z3_version, "Z3 version 5.1.0 - 64 bit");
    assert!(cvc5_version.starts_with("cvc5 1.3.4 [git f3b21c4 on branch HEAD]\n"));

    let z3 = SolverConfig::new(
        SolverEngine::Z3,
        z3_path,
        SolverPin::new(z3_version, z3_digest),
        AdapterLimits::default(),
    )
    .expect("Z3 configuration");
    let cvc5 = SolverConfig::new(
        SolverEngine::Cvc5,
        cvc5_path,
        SolverPin::new(cvc5_version, cvc5_digest),
        AdapterLimits::default(),
    )
    .expect("cvc5 configuration");
    let corpus = [
        AnalysisRequest::consistency(
            vec![],
            vec![statement("real-sat", value("x"), &["x"])],
            vec![],
        )
        .expect("sat request"),
        AnalysisRequest::consistency(
            vec![],
            vec![statement(
                "real-unsat",
                and(value("x"), not(value("x"))),
                &["x"],
            )],
            vec![],
        )
        .expect("unsat request"),
    ];
    let expected = [AnalysisStatus::Satisfied, AnalysisStatus::Refuted];
    for (request, expected_status) in corpus.iter().zip(expected) {
        let query = lower_analysis_request(request).expect("real query");
        let run = execute_differential(&query, &z3, &cvc5, &CancellationToken::default())
            .expect("real differential run");
        assert_eq!(
            run.disposition(),
            DifferentialDisposition::Agreement,
            "real differential failure: {run:#?}"
        );
        assert_eq!(run.agreed_status(), Some(expected_status));
        let report = render_differential_report(&query, &run).expect("render real report");
        validate_differential_report(&report, &query, &run).expect("real report");
    }
}

/// FR-005-AC-1/2/4: one canonical renderer backs validation and no-replace atomic publication.
/// Trace: TC-007, TC-008, FR-005-AC-1, FR-005-AC-2, FR-005-AC-4
#[test]
fn report_bytes_are_canonical_reconstructed_and_published_without_overwrite() {
    let request =
        AnalysisRequest::dead_antecedent(vec![], statement("report", literal(false), &[]), vec![])
            .expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let z3 = execute_engine_response(&query, "unsat\n", SolverEngine::Z3);
    let cvc5 = execute_engine_response(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    let first = render_differential_report(&query, &run).expect("first report");
    let second = render_differential_report(&query, &run).expect("second report");
    assert_eq!(first, second);
    validate_differential_report(&first, &query, &run).expect("valid report");
    let value: serde_json::Value = serde_json::from_slice(&first).expect("JSON");
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/differential-report-v1.schema.json"
    ))
    .expect("schema JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .unwrap_or_else(|error| panic!("schema compile: {error}"));
    assert!(validator.is_valid(&value));
    assert_eq!(
        value["pgm01Envelope"]["status"],
        serde_json::Value::String("unavailable".to_owned())
    );
    assert_eq!(value["engines"].as_array().expect("engines").len(), 2);
    assert!(render_differential_summary(&run).contains("PGM-01 envelope: unavailable"));

    let mut mutated = value.clone();
    let mut omitted = mutated.clone();
    omitted
        .as_object_mut()
        .expect("object")
        .remove("queryDigest");
    assert!(!validator.is_valid(&omitted));
    mutated["differential"]["disposition"] = serde_json::Value::String("disagreement".to_owned());
    let mutated = reseal_report(mutated);
    assert!(validate_differential_report(&mutated, &query, &run).is_err());

    let mut unknown_field = value.clone();
    unknown_field["unexpected"] = serde_json::Value::Bool(true);
    assert!(validate_report_document(&reseal_report(unknown_field)).is_err());

    let mut raw_mismatch = value.clone();
    raw_mismatch["engines"][0]["stdoutHex"] = serde_json::Value::String("00".to_owned());
    assert!(validate_report_document(&reseal_report(raw_mismatch)).is_err());

    let mut query_mismatch = value.clone();
    query_mismatch["queryHex"] = serde_json::Value::String("00".to_owned());
    assert!(validate_report_document(&reseal_report(query_mismatch)).is_err());

    let mut swapped = value.clone();
    swapped["engines"]
        .as_array_mut()
        .expect("engines")
        .swap(0, 1);
    assert!(validate_report_document(&reseal_report(swapped)).is_err());

    let mut false_status = value.clone();
    false_status["engines"][0]["status"] = serde_json::Value::String("refuted".to_owned());
    assert!(validate_report_document(&reseal_report(false_status)).is_err());

    let mut noncanonical = first.clone();
    noncanonical.push(b'\n');
    assert!(validate_report_document(&noncanonical).is_err());

    let directory = TempDirectory::new("atomic-report");
    let destination = directory.path().join("report.json");
    publish_report_new(&destination, &first).expect("publish");
    assert_eq!(fs::read(&destination).expect("published"), first);
    let replacement = b"developer-owned";
    assert!(publish_report_new(&destination, replacement).is_err());
    assert_eq!(fs::read(&destination).expect("unchanged"), first);
    assert!(fs::read_dir(directory.path())
        .expect("directory")
        .all(|entry| !entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .starts_with(".quire-analyze-report-")));

    let race_directory = TempDirectory::new("atomic-report-race");
    let race_destination = race_directory.path().join("winner.json");
    let barrier = Arc::new(Barrier::new(8));
    let publishers = (0..8)
        .map(|publisher| {
            let barrier = Arc::clone(&barrier);
            let destination = race_destination.clone();
            thread::spawn(move || {
                let bytes = format!("publisher-{publisher}").into_bytes();
                barrier.wait();
                let result = publish_report_new(&destination, &bytes);
                (bytes, result)
            })
        })
        .collect::<Vec<_>>();
    let outcomes = publishers
        .into_iter()
        .map(|publisher| publisher.join().expect("publisher thread"))
        .collect::<Vec<_>>();
    let winners = outcomes
        .iter()
        .filter(|(_, result)| result.is_ok())
        .collect::<Vec<_>>();
    assert_eq!(winners.len(), 1);
    assert_eq!(
        fs::read(&race_destination).expect("race winner"),
        winners[0].0
    );
    assert!(fs::read_dir(race_directory.path())
        .expect("race directory")
        .all(|entry| !entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .starts_with(".quire-analyze-report-")));

    let cli_input = directory.path().join("cli-input.json");
    let cli_output = directory.path().join("cli-output.json");
    fs::write(&cli_input, &first).expect("CLI input");
    let cli = Command::new(env!("CARGO_BIN_EXE_quire-analyze"))
        .args([
            "publish-report",
            "--input",
            cli_input.to_str().expect("UTF-8 input path"),
            "--output",
            cli_output.to_str().expect("UTF-8 output path"),
        ])
        .output()
        .expect("CLI execution");
    assert_eq!(cli.status.code(), Some(0));
    assert!(cli.stdout.is_empty());
    assert!(String::from_utf8(cli.stderr)
        .expect("diagnostic UTF-8")
        .contains("published"));
    assert_eq!(fs::read(&cli_output).expect("CLI output"), first);

    let refused = Command::new(env!("CARGO_BIN_EXE_quire-analyze"))
        .args([
            "publish-report",
            "--input",
            cli_input.to_str().expect("UTF-8 input path"),
            "--output",
            cli_output.to_str().expect("UTF-8 output path"),
        ])
        .output()
        .expect("CLI refusal");
    assert_eq!(refused.status.code(), Some(4));
    assert_eq!(fs::read(&cli_output).expect("unchanged CLI output"), first);

    let refuted_request = AnalysisRequest::consistency(
        vec![],
        vec![statement("cli-refuted", literal(false), &[])],
        vec![],
    )
    .expect("refuted request");
    let refuted_query = lower_analysis_request(&refuted_request).expect("refuted query");
    let refuted_run = compare_solver_records(
        &refuted_query,
        &execute_engine_response(&refuted_query, "unsat\n", SolverEngine::Z3),
        &execute_engine_response(&refuted_query, "unsat\n", SolverEngine::Cvc5),
    );
    let refuted_input = directory.path().join("cli-refuted-input.json");
    let refuted_output = directory.path().join("cli-refuted-output.json");
    fs::write(
        &refuted_input,
        render_differential_report(&refuted_query, &refuted_run).expect("refuted report"),
    )
    .expect("refuted input");
    let refuted_cli = Command::new(env!("CARGO_BIN_EXE_quire-analyze"))
        .args([
            "publish-report",
            "--input",
            refuted_input.to_str().expect("UTF-8 input path"),
            "--output",
            refuted_output.to_str().expect("UTF-8 output path"),
        ])
        .output()
        .expect("refuted CLI execution");
    assert_eq!(refuted_cli.status.code(), Some(1));

    let invalid_input = directory.path().join("invalid.json");
    let invalid_output = directory.path().join("invalid-output.json");
    fs::write(&invalid_input, b"{}").expect("invalid input");
    let invalid_cli = Command::new(env!("CARGO_BIN_EXE_quire-analyze"))
        .args([
            "publish-report",
            "--input",
            invalid_input.to_str().expect("UTF-8 input path"),
            "--output",
            invalid_output.to_str().expect("UTF-8 output path"),
        ])
        .output()
        .expect("invalid CLI execution");
    assert_eq!(invalid_cli.status.code(), Some(2));
    assert!(!invalid_output.exists());
}

/// A report's conclusion must be re-derivable from the evidence it retains.
///
/// Regression for SR-001 H2. Validation proved `stdoutHex` matched
/// `stdoutSha256` and that `reportDigest` covered the payload — all
/// self-consistency, which an author editing both halves together satisfies. It
/// never asked the retained stdout whether it agreed with the claimed outcome,
/// nor re-derived `queryDigest` from the `requestDigest` and query bytes the
/// document already carries. A `refuted` report could be forged into a
/// `satisfied` one, resealed, and validated — and the CLI turned that into exit
/// zero.
///
/// Trace: TC-007, FR-005-AC-2, NFR-002-AC-1, NFR-002-AC-3
#[test]
fn a_forged_conclusion_is_refused_by_re_derivation_from_retained_evidence() {
    let request =
        AnalysisRequest::dead_antecedent(vec![], statement("forge", literal(false), &[]), vec![])
            .expect("request");
    let query = lower_analysis_request(&request).expect("lower");
    let z3 = execute_engine_response(&query, "unsat\n", SolverEngine::Z3);
    let cvc5 = execute_engine_response(&query, "unsat\n", SolverEngine::Cvc5);
    let run = compare_solver_records(&query, &z3, &cvc5);
    let authoritative = render_differential_report(&query, &run).expect("report");

    // Positive control. Every refusal below is meaningless without it.
    let disposition =
        validate_report_document(&authoritative).expect("the authoritative report validates");
    assert_eq!(disposition, DifferentialDisposition::Agreement);

    let value: serde_json::Value = serde_json::from_slice(&authoritative).expect("JSON");

    // The retained stdout says `unsat`. Claiming `sat` — with the paired status
    // and the differential rewritten so the document stays internally
    // consistent — must be refused, because stdout is the evidence.
    let mut forged = value.clone();
    for index in 0..2 {
        forged["engines"][index]["outcome"] = serde_json::Value::String("sat".to_owned());
        forged["engines"][index]["status"] = serde_json::Value::String("refuted".to_owned());
        forged["engines"][index]["explanation"] = serde_json::Value::String("verified".to_owned());
    }
    forged["differential"]["agreedStatus"] = serde_json::Value::String("refuted".to_owned());
    let forged = reseal_report(forged);
    assert!(
        validate_report_document(&forged).is_err(),
        "a report whose retained stdout says unsat validated as sat"
    );

    // The same attack in the other direction: leave the outcome alone and swap
    // the retained bytes for a `sat` transcript, fixing the paired digest so the
    // self-consistency check passes.
    let mut swapped_evidence = value.clone();
    let sat_bytes = b"sat\n";
    let sat_hex = sat_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    swapped_evidence["engines"][0]["stdoutHex"] = serde_json::Value::String(sat_hex);
    swapped_evidence["engines"][0]["stdoutSha256"] =
        serde_json::Value::String(format!("{:x}", Sha256::digest(sat_bytes)));
    assert!(
        validate_report_document(&reseal_report(swapped_evidence)).is_err(),
        "retained stdout was replaced wholesale, digest and all, and still validated"
    );

    // `queryDigest` must be re-derived from `requestDigest` and the query bytes,
    // both of which the document carries. Replacing the query with a different
    // program and its paired digest must not pass.
    let mut relabelled_query = value.clone();
    let other = b"(set-logic QF_UF)\n(check-sat)\n";
    relabelled_query["queryHex"] = serde_json::Value::String(
        other
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    );
    relabelled_query["querySha256"] =
        serde_json::Value::String(format!("{:x}", Sha256::digest(other)));
    assert!(
        validate_report_document(&reseal_report(relabelled_query)).is_err(),
        "the query bytes were replaced and queryDigest still validated against them"
    );

    // A report produced under a different contract-IR revision was produced
    // under different clause-digest semantics.
    let mut other_revision = value.clone();
    other_revision["contractIrRevision"] = serde_json::Value::String("0".repeat(40));
    assert!(
        validate_report_document(&reseal_report(other_revision)).is_err(),
        "a foreign contract-IR revision validated"
    );
}
