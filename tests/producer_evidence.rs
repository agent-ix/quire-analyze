//! FR-009 library pieces for `producer-execute`: requests, the result line and the SolverRecord.

#![cfg(target_os = "linux")]

mod fake_script;

use ix_trace_rs::trace;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use quire_analyze::{
    compare_solver_records, execute_solver, lower_boolean_statements, parse_result_line,
    query_request, solver_record_from_evidence, version_request, version_text, AdapterLimits,
    CancellationToken, DifferentialDisposition, EvidenceErrorKind, ProducerExecutionResult,
    QueryBundle, SolverConfig, SolverEngine, SolverOutcome, SolverRecord,
};
use quire_contract_model::{
    AnchorName, Clause, ClauseId, ClauseKind, ContractPackage, DeclarationEnvironment,
    ExecutionPoint, Expression, ExpressionKind, PackageId, Requirement, RequirementId,
    RequirementRef, RequirementRevision, SchemaVersion, SourceDocumentId, SourceIdentity,
    SourceLocation, SourceRevision, SourceSpan, TypedExpression, ValueType,
};
use serde_json::{json, Value};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quire-analyze-{label}-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create temporary directory");
        Self(path)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn span() -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("evidence-test").expect("source id"),
        SourceRevision::new(1).expect("source revision"),
    );
    SourceSpan::new(
        SourceLocation::new(source.clone(), 1, 1, 0).expect("start"),
        SourceLocation::new(source, 1, 2, 1).expect("end"),
    )
    .expect("span")
}

fn statement(index: usize) -> quire_analyze::StatementInput {
    let package_id = PackageId::new("agent-ix/evidence-test").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-evidence-{index}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let environment =
        DeclarationEnvironment::new(owner, vec![], vec![], vec![]).expect("environment");
    let point = ExecutionPoint::Pre {
        operation: AnchorName::new("solve").expect("anchor").into(),
    };
    let expression = Expression::new(ExpressionKind::BooleanLiteral { value: true }, span());
    let checked = environment
        .check_expression(&expression, &ValueType::Boolean, &point, true)
        .expect("typed expression");
    let clause = Clause::new(
        ClauseId::new(format!("C-evidence-{index}")).expect("clause"),
        ClauseKind::Assertion,
        Some(point),
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
    let source = span().source().clone();
    let package = ContractPackage::new(package_id, SchemaVersion::V1_1, source, vec![requirement])
        .expect("package");
    quire_analyze::StatementInput::from_clause(
        &package,
        &package.requirements()[0],
        &package.requirements()[0].clauses()[0],
        environment,
    )
    .expect("statement")
}

fn query() -> QueryBundle {
    lower_boolean_statements(&[statement(0)], &[]).expect("query")
}

fn config(engine: SolverEngine, executable: &str) -> SolverConfig {
    SolverConfig::new(engine, executable, AdapterLimits::default()).expect("solver config")
}

fn result_line(state: Value, process: Option<(Option<Value>, &str, &str)>) -> Vec<u8> {
    let process = process.map_or(Value::Null, |(status, stdout, stderr)| {
        json!({
            "terminalStatus": status,
            "stdout": {"bytes": stdout.as_bytes(), "truncated": false},
            "stderr": {"bytes": stderr.as_bytes(), "truncated": false},
        })
    });
    let mut line = serde_json::to_vec(&json!({
        "protocol": "engineering-assurance.producer-execution-result/v1",
        "producer": {"name": "z3", "version": "Z3 4.13", "executable": "/opt/z3"},
        "cancellationEvent": null,
        "timing": {"preflightNanos": 1, "executionNanos": 7_000_000, "totalNanos": 9},
        "observedHost": null,
        "process": process,
        "artifacts": [],
        "state": state,
    }))
    .expect("serialize");
    line.push(b'\n');
    line
}

fn parse(state: Value, process: Option<(Option<Value>, &str, &str)>) -> ProducerExecutionResult {
    parse_result_line(&result_line(state, process)).expect("result document")
}

fn completed(code: i64, stdout: &str, stderr: &str) -> ProducerExecutionResult {
    parse(
        json!({"kind": "completed", "observation": {"exitCode": code}}),
        Some((
            Some(json!({"kind": "exit_code", "value": code})),
            stdout,
            stderr,
        )),
    )
}

fn version() -> ProducerExecutionResult {
    completed(0, "Z3 version 4.13.0\n", "")
}

fn record(result: &ProducerExecutionResult) -> SolverRecord {
    let config = config(SolverEngine::Z3, "/opt/z3");
    solver_record_from_evidence(&query(), &config, &version(), Some(result))
}

fn installed_engineering_assurance() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo/bin"));
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .chain(home)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join("engineering-assurance"))
        .find(|candidate| candidate.is_file())
}

fn published_schema(binary: &Path, which: &str) -> Value {
    let output = Command::new(binary)
        .args(["producer-execute", "--schema", which])
        .output()
        .expect("run engineering-assurance");
    assert!(output.status.success(), "--schema {which} failed");
    serde_json::from_slice(&output.stdout).expect("schema json")
}

/// Both built requests validate against `producer-execute --schema request`.
#[test]
fn requests_validate_against_the_published_request_schema() {
    let Some(binary) = installed_engineering_assurance() else {
        eprintln!("skipped: engineering-assurance is not installed");
        return;
    };
    let schema = published_schema(&binary, "request");
    let validator = jsonschema::JSONSchema::compile(&schema).expect("compile request schema");
    let config = config(SolverEngine::Z3, "/opt/z3");
    let root = Path::new("/out/.capability-1");
    let requests = [
        version_request(&config, root).expect("version request"),
        query_request(&config, root, "Z3 version 4.13.0", Path::new("query.smt2"))
            .expect("query request"),
    ];
    for request in requests {
        let document: Value = serde_json::from_slice(&request.to_json_bytes()).expect("json");
        if let Err(errors) = validator.validate(&document) {
            let messages: Vec<String> = errors.map(|error| error.to_string()).collect();
            panic!("{document}: {messages:?}");
        };
    }
}

/// The result line fixtures validate against `producer-execute --schema result`.
#[test]
fn result_fixtures_validate_against_the_published_result_schema() {
    let Some(binary) = installed_engineering_assurance() else {
        eprintln!("skipped: engineering-assurance is not installed");
        return;
    };
    let schema = published_schema(&binary, "result");
    let validator = jsonschema::JSONSchema::compile(&schema).expect("compile result schema");
    let line = result_line(
        json!({"kind": "completed", "observation": {"exitCode": 0}}),
        Some((Some(json!({"kind": "exit_code", "value": 0})), "sat\n", "")),
    );
    let document: Value = serde_json::from_slice(&line).expect("json");
    assert!(validator.is_valid(&document));
    let mut extra = document;
    extra["state"]["extra"] = json!(1);
    assert!(!validator.is_valid(&extra));
}

/// FR-009-AC-15 and AC-16: the version and query requests.
#[trace("FR-009-AC-15", "FR-009-AC-16")]
#[test]
fn version_and_query_requests_follow_fr_009() {
    let config = config(SolverEngine::Cvc5, "/opt/cvc5");
    let root = Path::new("/out/.capability-1");
    let version: Value = serde_json::from_slice(
        &version_request(&config, root)
            .expect("version request")
            .to_json_bytes(),
    )
    .expect("json");
    assert_eq!(version["producer"]["version"], "unprobed");
    assert_eq!(version["producer"]["name"], "cvc5");
    assert_eq!(version["producer"]["executable"], "/opt/cvc5");
    assert_eq!(version["stdin"], json!({"kind": "null"}));
    assert_eq!(version["inputs"], json!([]));
    assert_eq!(
        version["arguments"],
        json!([{"kind": "literal", "value": "--version"}])
    );

    let query: Value = serde_json::from_slice(
        &query_request(&config, root, "cvc5 1.2", Path::new("query.smt2"))
            .expect("query request")
            .to_json_bytes(),
    )
    .expect("json");
    assert_eq!(query["producer"]["version"], "cvc5 1.2");
    assert_eq!(
        query["stdin"],
        json!({"kind": "input_artifact", "role": "query"})
    );
    assert_eq!(
        query["inputs"],
        json!([{"role": "query", "path": "query.smt2"}])
    );
    assert_eq!(
        query["arguments"],
        json!([
            {"kind": "literal", "value": "--lang=smt2"},
            {"kind": "literal", "value": "--no-incremental"}
        ])
    );
    for request in [&version, &query] {
        assert_eq!(
            request["protocol"],
            "engineering-assurance.producer-execution-request/v1"
        );
        assert_eq!(request["procedure"], "direct");
        assert_eq!(request["environment"], json!({}));
        assert_eq!(request["outputs"], json!([]));
        assert_eq!(request["capabilityRoot"], "/out/.capability-1");
        assert_eq!(
            request["caller"],
            json!({"kind": "quire-analyze.run-package", "version": "1"})
        );
        assert_eq!(request["cancellation"], json!({"kind": "disabled"}));
        assert_eq!(
            request["containment"],
            json!({"profile": "process-group-v1", "contract": {"kind": "quire.solver-process", "version": "v1"}})
        );
        assert_eq!(request["response"]["exitCodes"], json!({"kind": "any"}));
        let budget = &request["budget"];
        assert_eq!(budget["timeoutMillis"], 5_000);
        assert_eq!(budget["maxStdoutBytes"], 8_388_608);
        assert_eq!(budget["maxStderrBytes"], 1_048_576);
        assert_eq!(budget["maxInputBytes"], 16_777_216);
        assert_eq!(budget["maxOutputArtifacts"], 0);
        assert_eq!(budget["maxOutputBytes"], 0);
        assert_eq!(budget["maxDescendants"], 16);
        assert_eq!(budget["maxConcurrency"], 1);
    }
    assert_eq!(
        query_request(&config, root, "", Path::new("/q"))
            .expect_err("empty version")
            .kind(),
        EvidenceErrorKind::Request
    );
    assert!(version_request(&config, Path::new("relative")).is_err());
    for bad in ["/abs/query", "../query", ""] {
        assert!(
            query_request(&config, root, "v", Path::new(bad)).is_err(),
            "{bad}"
        );
    }
}

/// The result line is exactly one newline-terminated, closed, valid result document.
#[test]
fn result_line_must_be_exactly_one_result_document() {
    let good = result_line(json!({"kind": "timed_out"}), None);
    assert!(parse_result_line(&good).is_ok());
    let two = [good.clone(), good.clone()].concat();
    assert_eq!(
        parse_result_line(&two).expect_err("two lines").kind(),
        EvidenceErrorKind::ResultLine
    );
    assert_eq!(
        parse_result_line(b"").expect_err("none").kind(),
        EvidenceErrorKind::ResultLine
    );
    assert_eq!(
        parse_result_line(b"{\"protocol\":\"x\"}\n")
            .expect_err("foreign")
            .kind(),
        EvidenceErrorKind::ResultDocument
    );
    let wrong = String::from_utf8(good.clone())
        .expect("utf8")
        .replace("result/v1", "result/v2");
    assert!(parse_result_line(wrong.as_bytes()).is_err());
    let extra = String::from_utf8(good.clone())
        .expect("utf8")
        .replacen('{', "{\"extra\":1,", 1);
    assert!(parse_result_line(extra.as_bytes()).is_err());
    let unterminated = &good[..good.len() - 1];
    assert_eq!(
        parse_result_line(unterminated)
            .expect_err("unterminated")
            .kind(),
        EvidenceErrorKind::ResultLine
    );
    assert_eq!(
        parse_result_line(b"\n").expect_err("empty line").kind(),
        EvidenceErrorKind::ResultLine
    );
    let no_process = result_line(
        json!({"kind": "completed", "observation": {"exitCode": 0}}),
        None,
    );
    assert!(parse_result_line(&no_process).is_err());
}

/// One fixture per non-completed row of the result-state table.
#[test]
fn every_result_state_maps_to_its_table_outcome_and_disposition() {
    let process = |status: Option<Value>| Some((status, "sat\n", ""));
    let rows = [
        (
            json!({"kind": "unavailable"}),
            None,
            SolverOutcome::MissingExecutable,
        ),
        (json!({"kind": "timed_out"}), None, SolverOutcome::TimedOut),
        (
            json!({"kind": "failed", "reason": {"kind": "exit_code", "value": 3}}),
            process(Some(json!({"kind": "exit_code", "value": 3}))),
            SolverOutcome::NonzeroExit,
        ),
        (
            json!({"kind": "failed", "reason": {"kind": "signal", "value": 9}}),
            process(Some(json!({"kind": "signal", "value": 9}))),
            SolverOutcome::Signaled,
        ),
        (
            json!({"kind": "failed", "reason": {"kind": "stdout_too_large"}}),
            process(None),
            SolverOutcome::StdoutLimit,
        ),
        (
            json!({"kind": "failed", "reason": {"kind": "stderr_too_large"}}),
            process(None),
            SolverOutcome::StderrLimit,
        ),
        (
            json!({"kind": "failed", "reason": {"kind": "observation"}}),
            None,
            SolverOutcome::IoError,
        ),
        (
            json!({"kind": "failed", "reason": {"kind": "output_artifact"}}),
            None,
            SolverOutcome::IoError,
        ),
        (
            json!({"kind": "refused", "reason": "executable"}),
            None,
            SolverOutcome::SpawnError,
        ),
        (
            json!({"kind": "malformed_response"}),
            None,
            SolverOutcome::MalformedOutput,
        ),
        (
            json!({"kind": "containment_failure"}),
            None,
            SolverOutcome::CleanupFailed,
        ),
        (json!({"kind": "cancelled"}), None, SolverOutcome::Cancelled),
    ];
    let query = query();
    let peer = record(&completed(0, "sat\n", ""));
    assert_eq!(peer.outcome(), SolverOutcome::Sat);
    for (state, process, expected) in rows {
        let state_text = state.to_string();
        let record = record(&parse(state, process));
        assert_eq!(record.outcome(), expected, "{state_text}");
        assert!(!record.is_conclusive_candidate(), "{state_text}");
        let disposition = compare_solver_records(&query, &record, &peer).disposition();
        let wanted = if expected == SolverOutcome::MissingExecutable {
            DifferentialDisposition::Unavailable
        } else {
            DifferentialDisposition::Inconclusive
        };
        assert_eq!(disposition, wanted, "{state_text}");
    }
}

/// Every closed object of the result schema refuses an unknown field, and every required key
/// (including nullable ones) must be present.
#[test]
fn result_decoder_is_closed_and_requires_every_schema_key() {
    let process = Some((Some(json!({"kind": "exit_code", "value": 0})), "sat\n", ""));
    let refused = |document: Value| {
        let mut line = serde_json::to_vec(&document).expect("serialize");
        line.push(b'\n');
        assert!(parse_result_line(&line).is_err(), "{document}");
    };
    let base = |state: Value, process: Option<(Option<Value>, &str, &str)>| -> Value {
        let line = result_line(state, process);
        serde_json::from_slice(&line).expect("json")
    };
    let completed_state = json!({"kind": "completed", "observation": {"exitCode": 0}});

    for state in [
        json!({"kind": "timed_out", "extra": 1}),
        json!({"kind": "unavailable", "extra": 1}),
        json!({"kind": "refused", "reason": "input", "extra": 1}),
        json!({"kind": "failed", "reason": {"kind": "exit_code", "value": 1, "extra": 1}}),
        json!({"kind": "failed", "reason": {"kind": "stdout_too_large", "extra": 1}}),
        json!({"kind": "completed", "observation": {"exitCode": 0, "extra": 1}}),
        json!({"kind": "completed", "observation": {"exitCode": 0}, "extra": 1}),
    ] {
        refused(base(state, process.clone()));
    }

    let mut document = base(completed_state.clone(), process.clone());
    document["process"]
        .as_object_mut()
        .expect("process")
        .remove("terminalStatus");
    refused(document);

    for key in ["process", "cancellationEvent", "observedHost", "artifacts"] {
        let mut document = base(completed_state.clone(), process.clone());
        document.as_object_mut().expect("object").remove(key);
        refused(document);
    }

    let mut document = base(completed_state, process);
    document["timing"]
        .as_object_mut()
        .expect("timing")
        .remove("executionNanos");
    refused(document);
}

/// FR-009-AC-16: a bad version run yields the table outcome, empty version text identity-error.
#[trace("FR-009-AC-16")]
#[test]
fn bad_version_run_yields_the_table_outcome() {
    let config = config(SolverEngine::Z3, "/opt/z3");
    assert_eq!(
        version_text(&config, &version()).as_deref(),
        Ok("Z3 version 4.13.0")
    );
    let query = query();
    for (bad, expected) in [
        (
            parse(json!({"kind": "unavailable"}), None),
            SolverOutcome::MissingExecutable,
        ),
        (
            parse(json!({"kind": "timed_out"}), None),
            SolverOutcome::TimedOut,
        ),
        (completed(0, "  \n", ""), SolverOutcome::IdentityError),
        (completed(0, "", ""), SolverOutcome::IdentityError),
        (completed(2, "Z3", ""), SolverOutcome::IdentityError),
    ] {
        assert_eq!(version_text(&config, &bad), Err(expected));
        let skipped = solver_record_from_evidence(&query, &config, &bad, None);
        assert_eq!(skipped.outcome(), expected);
        assert!(skipped.identity().is_none());
        let wanted = if expected == SolverOutcome::MissingExecutable {
            DifferentialDisposition::Unavailable
        } else {
            DifferentialDisposition::Inconclusive
        };
        let peer = record(&completed(0, "sat\n", ""));
        assert_eq!(
            compare_solver_records(&query, &skipped, &peer).disposition(),
            wanted
        );
    }
}

/// A completed result classifies as a direct run of the same bytes does.
#[test]
fn completed_result_classifies_as_a_direct_run() {
    let query = query();
    let directory = TempDirectory::new("equivalence");
    for (stdout, stderr, code) in [
        ("sat\n", "", 0),
        ("unsat\n", "", 0),
        ("unknown\n", "", 0),
        ("sat\n", "", 7),
        ("sat\n", "warning", 0),
        ("not-a-status\n", "", 0),
        ("(error \"injected\")\n", "", 0),
        ("sat\nunsat\n", "", 0),
        ("unsat\n", "", 1),
    ] {
        let path = directory.0.join("solver");
        let body = format!(
            "#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then printf 'Z3 version 4.13.0\\n'; exit 0; fi\n\
             printf '%s' '{stdout}'; printf '%s' '{stderr}' >&2; exit {code}\n"
        );
        fake_script::install(&path, &body);
        let direct = execute_solver(
            &query,
            &config(SolverEngine::Z3, path.to_str().expect("utf8")),
            &CancellationToken::default(),
        );
        let from_evidence = record(&completed(code, stdout, stderr));
        assert_eq!(
            from_evidence.outcome(),
            direct.outcome(),
            "{stdout:?} {stderr:?} {code}"
        );
        assert_eq!(from_evidence.model(), direct.model());
        assert_eq!(from_evidence.stdout(), direct.stdout());
        assert_eq!(from_evidence.exit(), direct.exit());
    }
}
