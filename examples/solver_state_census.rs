//! Native producer: exercise every solver outcome and report what survives.
//!
//! This is the domain runner for issue #25. It is the repository's own tool, it
//! executes the real bounded adapter against real child processes, and it emits
//! a declared structured result. Nothing downstream re-runs it: Quire exports
//! and Quoin transcribes, and neither one may execute this program.
//!
//! What it is measuring is the thing this repository is not allowed to lose. A
//! solver that could not decide is not a solver that decided "no", and there are
//! many distinct ways for it to fail to decide. Each row below runs one of them
//! through `execute_solver`, classifies it through `classify_analysis`, and checks
//! the `SolverOutcome` AND the `AnalysisStatus` against what that row declared.
//! The report states how many of the enum's outcomes were reached and names the
//! ones that were not, because a count without a denominator is not a
//! measurement. A row whose outcome differs from the
//! one it was built to provoke is reported as a mismatch and makes the whole
//! census fail — it is not quietly relabelled to whatever happened.
//!
//! Usage:
//!
//! ```text
//! cargo run --quiet --example solver_state_census -- --json
//! ```

#![cfg(target_os = "linux")]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
};

use quire_analyze::{
    classify_analysis, compare_solver_records, execute_solver, lower_analysis_request,
    render_differential_report, validate_differential_report, AdapterLimits, AnalysisRequest,
    AnalysisStatus, CancellationToken, DifferentialDisposition, QueryBundle, SolverConfig,
    SolverDigest, SolverEngine, SolverOutcome, SolverPin, SolverRecord,
};
use quire_contract_ir::{
    AnchorName, Clause, ClauseId, ClauseKind, ContractPackage, DeclarationEnvironment,
    ExecutionPoint, Expression, ExpressionKind, PackageId, Requirement, RequirementId,
    RequirementRef, RequirementRevision, SchemaVersion, SourceDocumentId, SourceIdentity,
    SourceLocation, SourceRevision, SourceSpan, TypedExpression, ValueType,
};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

const CENSUS_SCHEMA: &str = "quire-analyze.solver-state-census/v1";
const VERSION: &str = "fake-solver 1.0";

/// One census row: a provoked condition and the outcome it must produce.
///
/// `expected` is stated up front rather than read back from the run. A census
/// that records whatever it observed would agree with a broken adapter as
/// readily as with a correct one.
struct Row {
    /// The fake solver behaviour to install.
    mode: &'static str,
    expected: SolverOutcome,
    /// The analysis status this outcome must produce for a consistency query.
    ///
    /// Stated per row, not read back from the run. Recording whatever status
    /// turned up would agree with a classifier that called every timed-out,
    /// signaled or version-mismatched solver `satisfied` — which is exactly the
    /// collapse this census exists to detect.
    expected_status: AnalysisStatus,
    /// Why this row exists, in the vocabulary a reviewer reads.
    note: &'static str,
}

fn rows() -> Vec<Row> {
    vec![
        Row {
            mode: "sat",
            expected_status: AnalysisStatus::Satisfied,
            expected: SolverOutcome::Sat,
            note: "conclusive: the solver decided satisfiable",
        },
        Row {
            mode: "unsat",
            expected_status: AnalysisStatus::Refuted,
            expected: SolverOutcome::Unsat,
            note: "conclusive: the solver decided unsatisfiable",
        },
        Row {
            mode: "unknown",
            expected_status: AnalysisStatus::Unknown,
            expected: SolverOutcome::Unknown,
            note: "non-conclusive: the solver ran and declined to decide",
        },
        Row {
            mode: "malformed",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::MalformedOutput,
            note: "non-conclusive: the response is not a recognised SMT-LIB status",
        },
        Row {
            mode: "solver_error",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::SolverError,
            note: "non-conclusive: the solver reported its own error",
        },
        Row {
            mode: "contradictory",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::ContradictoryOutput,
            note: "non-conclusive: two statuses in one response decide nothing",
        },
        Row {
            mode: "nonzero_diagnostic",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::NonzeroExit,
            note: "non-conclusive: the process failed without a status",
        },
        Row {
            mode: "signaled",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::Signaled,
            note: "non-conclusive: the process was killed by a signal",
        },
        Row {
            mode: "diagnostic",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::DiagnosticOutput,
            note: "non-conclusive: a status accompanied by stderr is not trusted",
        },
        Row {
            mode: "large_stdout",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::StdoutLimit,
            note: "non-conclusive: the response exceeded the capture bound",
        },
        Row {
            mode: "large_stderr",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::StderrLimit,
            note: "non-conclusive: the diagnostic stream exceeded its bound",
        },
        Row {
            mode: "slow",
            expected_status: AnalysisStatus::Timeout,
            expected: SolverOutcome::TimedOut,
            note: "non-conclusive: the wall-time bound elapsed",
        },
        Row {
            mode: "version_nonzero",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::IdentityError,
            note: "unavailable: the identity probe could not establish the engine",
        },
        Row {
            mode: "version_mutates",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::ExecutableChanged,
            note: "unavailable: the executable changed under the adapter",
        },
        Row {
            mode: "version_other",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::VersionMismatch,
            note: "unavailable: the engine is not the pinned version",
        },
        Row {
            mode: "missing",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::MissingExecutable,
            note: "unavailable: the engine is not installed at all",
        },
        Row {
            mode: "digest_other",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::ExecutableDigestMismatch,
            note: "unavailable: the executable is not the pinned bytes",
        },
        Row {
            mode: "cancelled",
            expected_status: AnalysisStatus::Timeout,
            expected: SolverOutcome::Cancelled,
            note: "non-conclusive: the caller withdrew the request",
        },
        Row {
            mode: "oversized_stdin",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::StdinLimit,
            note: "non-conclusive: the query exceeded the input bound and was never sent",
        },
        Row {
            mode: "large_model",
            expected_status: AnalysisStatus::ToolError,
            expected: SolverOutcome::ModelLimit,
            note: "non-conclusive: the model exceeded the capture bound",
        },
    ]
}

/// Every `SolverOutcome` this build knows about.
///
/// Written as an exhaustive `match` so that adding a variant fails the build
/// here rather than silently shrinking the denominator. A hand-maintained array
/// would let a twenty-fifth outcome go uncounted and unmentioned.
fn all_outcomes() -> Vec<SolverOutcome> {
    use SolverOutcome::*;
    let every = [
        Sat,
        Unsat,
        Unknown,
        TimedOut,
        Cancelled,
        MissingExecutable,
        SpawnError,
        IdentityError,
        ExecutableDigestMismatch,
        ExecutableChanged,
        VersionMismatch,
        SolverError,
        MalformedOutput,
        ContradictoryOutput,
        NonzeroExit,
        Signaled,
        StdinLimit,
        StdoutLimit,
        StderrLimit,
        DiagnosticOutput,
        ModelLimit,
        CleanupFailed,
        UnsupportedPlatform,
        IoError,
    ];
    for outcome in every {
        // Exhaustiveness guard: a new variant makes this match non-exhaustive.
        match outcome {
            Sat
            | Unsat
            | Unknown
            | TimedOut
            | Cancelled
            | MissingExecutable
            | SpawnError
            | IdentityError
            | ExecutableDigestMismatch
            | ExecutableChanged
            | VersionMismatch
            | SolverError
            | MalformedOutput
            | ContradictoryOutput
            | NonzeroExit
            | Signaled
            | StdinLimit
            | StdoutLimit
            | StderrLimit
            | DiagnosticOutput
            | ModelLimit
            | CleanupFailed
            | UnsupportedPlatform
            | IoError => {}
        }
    }
    every.to_vec()
}

fn main() -> ExitCode {
    let emit_json = std::env::args().any(|argument| argument == "--json");
    let workspace = TempDirectory::new("census");
    let query = query();

    let mut records: BTreeMap<String, SolverRecord> = BTreeMap::new();
    let mut entries = Vec::new();
    let mut mismatches = Vec::new();

    for row in rows() {
        let mode = row.mode;
        let record = execute_mode(workspace.path(), mode, &query);
        let observed = record.outcome();
        if observed != row.expected {
            mismatches.push(format!(
                "{mode}: expected {} and observed {}",
                row.expected.as_str(),
                observed.as_str()
            ));
        }
        let conclusion = classify_analysis(&query, &record);
        // The status is checked, not merely recorded. An unchecked column is a
        // column a reader trusts and nothing verifies.
        if conclusion.status() != row.expected_status {
            mismatches.push(format!(
                "{mode}: expected analysis status {} and observed {}",
                row.expected_status.as_str(),
                conclusion.status().as_str()
            ));
        }
        entries.push(json!({
            "mode": mode,
            "expectedOutcome": row.expected.as_str(),
            "observedOutcome": observed.as_str(),
            "expectedAnalysisStatus": row.expected_status.as_str(),
            "analysisStatus": conclusion.status().as_str(),
            "matched": observed == row.expected && conclusion.status() == row.expected_status,
            "conclusive": conclusion.is_conclusive(),
            "note": row.note,
        }));
        records.insert(mode.to_owned(), record);
    }

    // Two engines whose records are already in hand, compared without running
    // anything again. `compare_solver_records` is the domain's own comparison;
    // this program does not reimplement agreement.
    let mut dispositions = Vec::new();
    for (label, left, right, expected) in [
        (
            "agreement",
            "unsat",
            "unsat",
            DifferentialDisposition::Agreement,
        ),
        (
            "disagreement",
            "sat",
            "unsat",
            DifferentialDisposition::Disagreement,
        ),
        (
            "unavailable",
            "unsat",
            "version_other",
            DifferentialDisposition::Unavailable,
        ),
        (
            "inconclusive",
            "unsat",
            "unknown",
            DifferentialDisposition::Inconclusive,
        ),
    ] {
        let Some(z3) = records.get(left) else {
            mismatches.push(format!("{label}: no retained record for {left}"));
            continue;
        };
        if !records.contains_key(right) {
            mismatches.push(format!("{label}: no retained record for {right}"));
            continue;
        }
        // `compare_solver_records` reads the engine identity off the record, and
        // every retained census record was produced as Z3. The right-hand side is
        // re-run as cvc5 so the pair is two engines rather than one record
        // relabelled as the other, which would make agreement trivially true.
        let cvc5 = execute_mode_engine(workspace.path(), right, &query, SolverEngine::Cvc5);
        let run = compare_solver_records(&query, z3, &cvc5);
        let observed = run.disposition();
        if observed != expected {
            mismatches.push(format!(
                "{label}: expected disposition {} and observed {}",
                expected.as_str(),
                observed.as_str()
            ));
        }
        dispositions.push(json!({
            "case": label,
            "z3Mode": left,
            "cvc5Mode": right,
            "expectedDisposition": expected.as_str(),
            "observedDisposition": observed.as_str(),
            "matched": observed == expected,
            "agreedStatus": run.agreed_status().map(AnalysisStatus::as_str),
        }));
    }

    // Retained-output identity, measured on the domain's own report format.
    // `render_differential_report` produces the authoritative bytes and
    // `validate_differential_report` re-derives them; a report that validates
    // only because validation is lenient is caught by the two tamper probes,
    // which must both be refused. A probe that is *accepted* is a failure.
    let report_identity = match records.get("unsat") {
        Some(z3) => {
            let cvc5 = execute_mode_engine(workspace.path(), "unsat", &query, SolverEngine::Cvc5);
            let run = compare_solver_records(&query, z3, &cvc5);
            match render_differential_report(&query, &run) {
                Ok(bytes) => {
                    let accepted = validate_differential_report(&bytes, &query, &run).is_ok();
                    if !accepted {
                        mismatches
                            .push("report identity: authoritative bytes failed validation".into());
                    }
                    // Positive control first, then the negatives. A refusal never
                    // seen to accept anything is indistinguishable from a
                    // validator that refuses everything.
                    let mut probes = Vec::new();
                    for (label, mutated) in tamper_probes(&bytes) {
                        let refused = validate_differential_report(&mutated, &query, &run).is_err();
                        if !refused {
                            mismatches.push(format!("report identity: {label} was accepted"));
                        }
                        probes.push(json!({"probe": label, "refused": refused}));
                    }
                    json!({
                        "bytes": bytes.len(),
                        "sha256": format!("{:x}", Sha256::digest(&bytes)),
                        "disposition": run.disposition().as_str(),
                        "authoritativeBytesAccepted": accepted,
                        "tamperProbes": probes,
                    })
                }
                Err(error) => {
                    mismatches.push(format!("report identity: render failed: {error}"));
                    Value::Null
                }
            }
        }
        None => {
            mismatches.push("report identity: no retained unsat record".into());
            Value::Null
        }
    };

    // The distinctness claim, measured rather than asserted: the observed
    // outcomes must be as numerous as the rows that produced them. If two
    // provoked conditions collapsed onto one outcome, this shrinks.
    let distinct_outcomes: BTreeSet<&str> = entries
        .iter()
        .filter(|entry| entry["matched"] == Value::Bool(true))
        .filter_map(|entry| entry["observedOutcome"].as_str())
        .collect();
    let distinct_statuses: BTreeSet<&str> = entries
        .iter()
        .filter(|entry| entry["matched"] == Value::Bool(true))
        .filter_map(|entry| entry["analysisStatus"].as_str())
        .collect();

    // A count with no denominator is not a measurement. The outcomes this census
    // does NOT provoke are named, so the gap is legible instead of implied by a
    // bare `15`.
    let every = all_outcomes();
    let covered: BTreeSet<&str> = distinct_outcomes.iter().copied().collect();
    let unexercised: Vec<&str> = every
        .iter()
        .map(|outcome| outcome.as_str())
        .filter(|name| !covered.contains(name))
        .collect();

    let matched = entries
        .iter()
        .filter(|entry| entry["matched"] == Value::Bool(true))
        .count()
        + dispositions
            .iter()
            .filter(|entry| entry["matched"] == Value::Bool(true))
            .count();
    let total = entries.len() + dispositions.len();

    // `outcome` is the producer's own verdict, and it is what the assurance
    // adapter reads. It is derived from the mismatches, not asserted.
    let outcome = if mismatches.is_empty() {
        "passed"
    } else {
        "failed"
    };

    let document = json!({
        "schema": CENSUS_SCHEMA,
        "producer": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
        "outcome": outcome,
        "matched": matched,
        "total": total,
        "distinctOutcomes": distinct_outcomes.len(),
        "totalOutcomes": every.len(),
        "unexercisedOutcomes": unexercised,
        "distinctAnalysisStatuses": distinct_statuses.len(),
        "mismatches": mismatches,
        "solverStates": entries,
        "differentialDispositions": dispositions,
        "reportIdentity": report_identity,
    });

    if emit_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&document).expect("serializable census")
        );
    } else {
        println!("solver state census: {matched}/{total} matched, outcome {outcome}");
        for mismatch in &mismatches {
            println!("  mismatch: {mismatch}");
        }
    }

    if mismatches.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Named single edits to authoritative report bytes, each of which must be refused.
///
/// Every probe is one change to real bytes rather than a hand-written blob, so a
/// refusal cannot be earned by the document being obviously malformed. The names
/// say which integrity claim each one attacks.
fn tamper_probes(bytes: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
    let mut probes = Vec::new();
    let Ok(original) = serde_json::from_slice::<Value>(bytes) else {
        return probes;
    };

    // Semantic field: the disposition is rewritten to contradict the engine
    // records underneath it. The replacement is chosen to differ from whatever
    // the report actually says — writing back the value already there produces
    // byte-identical output, which a validator is right to accept and which
    // would make this probe prove nothing.
    let mut semantic = original.clone();
    let actual = semantic["differential"]["disposition"]
        .as_str()
        .unwrap_or("");
    semantic["differential"]["disposition"] = if actual == "agreement" {
        json!("disagreement")
    } else {
        json!("agreement")
    };
    assert_ne!(
        semantic["differential"]["disposition"], original["differential"]["disposition"],
        "the disposition probe must change the disposition"
    );
    probes.push((
        "tampered-semantic-field-disposition",
        serde_json::to_vec(&semantic).expect("serializable probe"),
    ));

    // Semantic field: an engine's normalized status is flipped away from the
    // outcome that produced it.
    let mut status = original.clone();
    status["engines"][0]["status"] = json!("satisfied");
    probes.push((
        "tampered-semantic-field-status",
        serde_json::to_vec(&status).expect("serializable probe"),
    ));

    // Retained bytes: the recorded raw stdout is altered while its digest is
    // left alone, which is the shape of an edited transcript.
    let mut retained = original.clone();
    retained["engines"][0]["stdoutHex"] = json!("6465616462656566");
    probes.push((
        "tampered-retained-bytes-stdout",
        serde_json::to_vec(&retained).expect("serializable probe"),
    ));

    // Identity: the self-declared report digest no longer covers the payload.
    let mut digest = original.clone();
    digest["reportDigest"] = json!("0".repeat(64));
    probes.push((
        "tampered-report-digest",
        serde_json::to_vec(&digest).expect("serializable probe"),
    ));

    // Truncation: a prefix of the authoritative bytes, which is malformed rather
    // than merely wrong, and must not be read as an empty or absent report.
    probes.push(("malformed-truncated", bytes[..bytes.len() / 2].to_vec()));

    probes
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("quire-analyze-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temporary directory");
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

fn script_body(mode: &str) -> String {
    let version_action = match mode {
        "version_nonzero" => "exit 9".to_owned(),
        "version_mutates" => format!("printf '{VERSION}\\n'; printf '# changed\\n' >> \"$0\""),
        // Reports a version the pin does not name. The executable is intact and
        // the process is healthy; the engine is simply not the one that was
        // pinned, which is unavailability and not a failed decision.
        "version_other" => "printf 'fake-solver 9.9\\n'".to_owned(),
        _ => format!("printf '{VERSION}\\n'"),
    };
    let action = match mode {
        "sat" => "printf 'sat\\n'".to_owned(),
        "unsat" => "printf 'unsat\\n'".to_owned(),
        "unknown" => "printf 'unknown\\n'".to_owned(),
        "malformed" => "printf 'not-a-status\\n'".to_owned(),
        "solver_error" => "printf '(error \"injected\")\\n'".to_owned(),
        "contradictory" => "printf 'sat\\nunsat\\n'".to_owned(),
        "nonzero_diagnostic" => "printf 'failure' >&2; exit 7".to_owned(),
        "signaled" => "kill -TERM $$".to_owned(),
        "diagnostic" => "printf 'warning' >&2; printf 'sat\\n'".to_owned(),
        "large_stdout" => "i=0; while [ $i -lt 65 ]; do printf x; i=$((i + 1)); done".to_owned(),
        "large_stderr" => {
            "i=0; while [ $i -lt 65 ]; do printf x >&2; i=$((i + 1)); done; printf 'sat\\n'"
                .to_owned()
        }
        "slow" => "/bin/sleep 30".to_owned(),
        "version_nonzero" | "version_mutates" | "version_other" | "digest_other" | "missing"
        | "cancelled" | "oversized_stdin" => "printf 'sat\\n'".to_owned(),
        "large_model" => "printf 'sat\\n(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)\\n'".to_owned(),
        other => panic!("unknown fake mode {other}"),
    };
    format!(
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ] || [ \"$1\" = \"--version\" ]; then\n  {version_action}\n  exit $?\nfi\n{action}\n"
    )
}

fn make_script(directory: &Path, name: &str, mode: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, script_body(mode)).expect("write fake solver");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions).expect("make fake solver executable");
    path
}

fn digest(path: &Path) -> SolverDigest {
    SolverDigest::from_bytes(Sha256::digest(fs::read(path).expect("read script")).into())
}

fn execute_mode(directory: &Path, mode: &str, query: &QueryBundle) -> SolverRecord {
    execute_mode_engine(directory, mode, query, SolverEngine::Z3)
}

fn execute_mode_engine(
    directory: &Path,
    mode: &str,
    query: &QueryBundle,
    engine: SolverEngine,
) -> SolverRecord {
    let executable = make_script(directory, &format!("{}-{mode}", engine.as_str()), mode);
    let mut limits = AdapterLimits {
        wall_time_ms: 250,
        stdout_bytes: 64,
        stderr_bytes: 64,
        model_bytes: 64,
        ..AdapterLimits::default()
    };

    // Four conditions are provoked by configuration rather than by the child.
    // They are the ones a fake solver cannot stage: an engine that is not there,
    // bytes that are not the pinned ones, a caller that withdrew, and a query
    // too large to send.
    let mut pin = SolverPin::new(VERSION, digest(&executable));
    let mut path = executable.clone();
    let cancellation = CancellationToken::default();
    match mode {
        "missing" => path = directory.join("no-such-solver"),
        "digest_other" => pin = SolverPin::new(VERSION, SolverDigest::from_bytes([0x11; 32])),
        "cancelled" => cancellation.cancel(),
        "oversized_stdin" => limits.stdin_bytes = 1,
        "large_model" => limits.model_bytes = 8,
        _ => {}
    }

    let config = SolverConfig::new(engine, &path, pin, limits).expect("solver config");
    execute_solver(query, &config, &cancellation)
}

fn span() -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("census").expect("source id"),
        SourceRevision::new(1).expect("source revision"),
    );
    SourceSpan::new(
        SourceLocation::new(source.clone(), 1, 1, 0).expect("start"),
        SourceLocation::new(source, 1, 2, 1).expect("end"),
    )
    .expect("span")
}

fn statement(index: usize) -> quire_analyze::StatementInput {
    let package_id = PackageId::new("agent-ix/census").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-census-{index}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let environment =
        DeclarationEnvironment::new(owner, vec![], vec![], vec![]).expect("environment");
    let point = ExecutionPoint::Pre {
        operation: AnchorName::new("solve").expect("anchor"),
    };
    let expression = Expression::new(ExpressionKind::BooleanLiteral { value: true }, span());
    let checked = environment
        .check_expression(&expression, &ValueType::Boolean, &point, true)
        .expect("typed expression");
    let clause = Clause::new(
        ClauseId::new(format!("C-census-{index}")).expect("clause"),
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
    let package = ContractPackage::new(package_id, SchemaVersion::V1_0, source, vec![requirement])
        .expect("package");
    quire_analyze::StatementInput::from_clause(
        &package,
        &package.requirements()[0],
        &package.requirements()[0].clauses()[0],
        environment,
    )
    .expect("statement")
}

/// A real consistency request, not a bare lowering.
///
/// `classify_analysis` refuses to classify a bundle with no analysis kind, and
/// answers `tool-error` for every outcome. A census built on a bare lowering
/// would therefore report one analysis status for all twenty-two solver states
/// and look like proof that they collapse.
fn query() -> QueryBundle {
    let request = AnalysisRequest::consistency(vec![], vec![statement(0)], vec![])
        .expect("consistency request");
    lower_analysis_request(&request).expect("lowered analysis request")
}
