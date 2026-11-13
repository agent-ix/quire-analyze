//! FR-009 and NFR-003 `run-package` over a stub `producer-execute` (TC-015).
//!
//! Every contract package is built at test time (`common`); nothing is committed. The stub is a
//! `/bin/sh` script that records each request it receives and answers with scripted result lines.

#![cfg(target_os = "linux")]

mod common;

use std::{
    ffi::OsString,
    fs,
    os::unix::{
        ffi::OsStringExt,
        fs::{symlink, PermissionsExt},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use common::{Document, TempDir, AND, NOT};
use ix_trace_rs::trace;
use quire_analyze::{
    lower_analysis_request, translate_analysis_claims, validate_report_document,
    DifferentialDisposition,
};
use quire_contract_model::{
    CheckedPackageReadLimits, ContractPackageReadResult, ContractPackageWireV1,
};
use serde_json::{json, Value};

const BINARY: &str = env!("CARGO_BIN_EXE_quire-analyze");
const VERSION_Z3: &str = "Z3 version 4.13.0 - 64 bit\n";
const VERSION_CVC5: &str = "This is cvc5 version 1.1.2\n";
const RESULT_PROTOCOL: &str = "engineering-assurance.producer-execution-result/v1";

/// A stub that logs the request, the mode of the capability root and the staged query, and
/// answers from `data/{engine}.{kind}` where kind is `version` or `query`.
fn stub_script(data: &Path) -> String {
    let data = data.display();
    format!(
        r#"#!/bin/sh
D='{data}'
req=
while IFS= read -r l || [ -n "$l" ]; do req="$req$l"; done
printf '%s\n' "$req" >> "$D/requests.log"
case "$req" in *'"stdin":{{"kind":"null"}}'*) kind=version;; *) kind=query;; esac
case "$req" in *'"producer":{{"name":"z3"'*) eng=z3;; *) eng=cvc5;; esac
root=${{req#*'"capabilityRoot":"'}}
root=${{root%%'"'*}}
stat -c %a "$root" >> "$D/modes.log"
if [ "$kind" = query ]; then cp "$root/query.smt2" "$D/$eng.staged"; fi
f="$D/$eng.$kind"
if [ -f "$f.signal" ]; then kill -KILL $$; fi
if [ -f "$f" ]; then
  while IFS= read -r l || [ -n "$l" ]; do printf '%s\n' "$l"; done < "$f"
fi
if [ -f "$f.exit" ]; then read -r c < "$f.exit"; exit "$c"; fi
exit 0
"#
    )
}

fn process(exit: i64, stdout: &str, stderr: &str) -> Value {
    json!({
        "terminalStatus": {"kind": "exit_code", "value": exit},
        "stdout": {"bytes": stdout.as_bytes(), "truncated": false},
        "stderr": {"bytes": stderr.as_bytes(), "truncated": false},
    })
}

fn line(state: &Value, process: Option<Value>) -> String {
    let document = json!({
        "protocol": RESULT_PROTOCOL,
        "producer": {"name": "z3", "version": "stub", "executable": "/stub"},
        "cancellationEvent": null,
        "timing": {"preflightNanos": 1, "executionNanos": 3_000_000, "totalNanos": 4_000_000},
        "observedHost": null,
        "process": process,
        "artifacts": [],
        "state": state,
    });
    serde_json::to_string(&document).expect("result line")
}

fn completed(exit: i64, stdout: &str, stderr: &str) -> String {
    line(
        &json!({"kind": "completed", "observation": {"exitCode": exit}}),
        Some(process(exit, stdout, stderr)),
    )
}

fn state_only(state: Value) -> String {
    line(&state, None)
}

/// A package, and the model response that satisfies its query.
struct Built {
    bytes: Vec<u8>,
    model: String,
}

fn model_for(bytes: &[u8]) -> String {
    let package = match ContractPackageWireV1::read(
        bytes,
        CheckedPackageReadLimits::bounded(),
        &Default::default(),
    ) {
        ContractPackageReadResult::Admitted(package) => *package,
        other => panic!("fixture refused: {other:?}"),
    };
    let translation = translate_analysis_claims(&package).expect("translation");
    let query = lower_analysis_request(translation.request()).expect("query");
    let definitions = query
        .variables()
        .iter()
        .map(|variable| format!("(define-fun {} () Bool true)", variable.symbol))
        .collect::<Vec<_>>()
        .join(" ");
    format!("sat\n(model {definitions})\n")
}

/// Two parameters, each asserted: every assignment of true satisfies them.
fn consistent() -> Built {
    let mut document = Document::new();
    let p = document.parameter();
    let q = document.parameter();
    let first = document.claim(&p);
    let second = document.claim(&q);
    let bytes = document.bytes(&[&first, &second]);
    Built {
        model: model_for(&bytes),
        bytes,
    }
}

/// A parameter and its negation, both asserted.
fn contradictory() -> Built {
    let mut document = Document::new();
    let p = document.parameter();
    let not_p = document.op(NOT, &[&p]);
    let first = document.claim(&p);
    let second = document.claim(&not_p);
    let bytes = document.bytes(&[&first, &second]);
    Built {
        model: model_for(&bytes),
        bytes,
    }
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

struct Fixture {
    dir: TempDir,
    package: PathBuf,
}

impl Fixture {
    fn new(package: &[u8]) -> Self {
        let dir = TempDir::new("run-package");
        for name in ["pkg", "out", "stub", "stub/data", "solvers"] {
            fs::create_dir(dir.path().join(name)).expect("directory");
        }
        let package_path = dir.path().join("pkg/package.json");
        fs::write(&package_path, package).expect("package");
        let producer = dir.path().join("stub/producer");
        fs::write(&producer, stub_script(&dir.path().join("stub/data"))).expect("stub");
        fs::set_permissions(&producer, fs::Permissions::from_mode(0o700)).expect("executable");
        let fixture = Self {
            dir,
            package: package_path,
        };
        fixture.answer_all(&consistent_answers());
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn producer(&self) -> PathBuf {
        self.path("stub/producer")
    }

    fn z3(&self) -> PathBuf {
        self.path("solvers/z3")
    }

    fn cvc5(&self) -> PathBuf {
        self.path("solvers/cvc5")
    }

    fn output(&self) -> PathBuf {
        self.path("out/report.json")
    }

    fn answer(&self, engine: &str, kind: &str, body: &str) {
        fs::write(self.path(&format!("stub/data/{engine}.{kind}")), body).expect("answer");
    }

    fn answer_all(&self, answers: &[(&str, &str, String)]) {
        for (engine, kind, body) in answers {
            self.answer(engine, kind, body);
        }
    }

    fn exit_after(&self, engine: &str, kind: &str, code: i32) {
        fs::write(
            self.path(&format!("stub/data/{engine}.{kind}.exit")),
            code.to_string(),
        )
        .expect("exit");
    }

    fn signal(&self, engine: &str, kind: &str) {
        fs::write(self.path(&format!("stub/data/{engine}.{kind}.signal")), "").expect("signal");
    }

    /// Both engines answer their version and, for the queries, `z3` and `cvc5`.
    fn both(&self, z3: &str, cvc5: &str) {
        self.answer_all(&answers(z3, cvc5));
    }

    fn arguments(&self) -> Vec<OsString> {
        let pairs: [(&str, &Path); 5] = [
            ("--package", &self.package),
            ("--output", &self.output()),
            ("--z3", &self.z3()),
            ("--cvc5", &self.cvc5()),
            ("--producer-execute", &self.producer()),
        ];
        let mut arguments = vec![OsString::from("run-package")];
        for (name, value) in pairs {
            arguments.push(name.into());
            arguments.push(value.into());
        }
        arguments
    }

    fn command(&self, arguments: &[OsString]) -> Command {
        let mut command = Command::new(BINARY);
        command.args(arguments).stdin(Stdio::null());
        command
    }

    fn run(&self) -> Run {
        self.run_with(&self.arguments())
    }

    fn run_with(&self, arguments: &[OsString]) -> Run {
        finish(self.command(arguments))
    }

    /// Replace the value of `option` in the standard arguments.
    fn with(&self, option: &str, value: impl Into<OsString>) -> Vec<OsString> {
        let mut arguments = self.arguments();
        let position = arguments
            .iter()
            .position(|argument| argument == option)
            .expect("option");
        arguments[position + 1] = value.into();
        arguments
    }

    fn without(&self, option: &str) -> Vec<OsString> {
        let mut arguments = self.arguments();
        let position = arguments
            .iter()
            .position(|argument| argument == option)
            .expect("option");
        arguments.drain(position..=position + 1);
        arguments
    }

    fn requests(&self) -> Vec<Value> {
        fs::read_to_string(self.path("stub/data/requests.log"))
            .unwrap_or_default()
            .lines()
            .map(|text| serde_json::from_str(text).expect("logged request"))
            .collect()
    }

    fn report(&self) -> Value {
        serde_json::from_slice(&fs::read(self.output()).expect("report")).expect("report json")
    }

    fn assert_no_report(&self) {
        assert!(!self.output().exists(), "a report was written");
        self.assert_no_leftovers();
    }

    /// The only entries of the output directory are the published report.
    fn assert_no_leftovers(&self) {
        let names = directory_names(&self.path("out"));
        assert!(
            names.iter().all(|name| name == "report.json"),
            "unexpected entries {names:?}"
        );
    }
}

fn directory_names(path: &Path) -> Vec<String> {
    let mut names = fs::read_dir(path)
        .expect("directory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn finish(mut command: Command) -> Run {
    let output = command.output().expect("run quire-analyze");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn answers<'a>(z3: &str, cvc5: &str) -> Vec<(&'a str, &'a str, String)> {
    vec![
        ("z3", "version", completed(0, VERSION_Z3, "")),
        ("z3", "query", completed(0, z3, "")),
        ("cvc5", "version", completed(0, VERSION_CVC5, "")),
        ("cvc5", "query", completed(0, cvc5, "")),
    ]
}

fn consistent_answers<'a>() -> Vec<(&'a str, &'a str, String)> {
    answers("unknown\n", "unknown\n")
}

fn outcomes(report: &Value) -> (String, String) {
    let outcome = |index: usize| {
        report["engines"][index]["outcome"]
            .as_str()
            .expect("outcome")
            .to_owned()
    };
    (outcome(0), outcome(1))
}

#[trace("FR-009-AC-1", "FR-009-AC-16", "FR-009-AC-2")]
#[test]
fn two_requests_per_engine_version_then_query_and_a_satisfied_report() {
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    fixture.both(&built.model, &built.model);
    let run = fixture.run();
    assert_eq!(run.code, Some(0), "{}", run.stderr);

    let requests = fixture.requests();
    assert_eq!(requests.len(), 4);
    let sequence = requests
        .iter()
        .map(|request| {
            (
                request["producer"]["name"].as_str().expect("engine"),
                request["stdin"]["kind"].as_str().expect("stdin"),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sequence,
        [
            ("z3", "null"),
            ("z3", "input_artifact"),
            ("cvc5", "null"),
            ("cvc5", "input_artifact")
        ]
    );
    for (index, request) in requests.iter().enumerate() {
        let (path, version, arguments, text) = if index < 2 {
            (fixture.z3(), VERSION_Z3, ["-version"].as_slice(), "-in")
        } else {
            (
                fixture.cvc5(),
                VERSION_CVC5,
                ["--version"].as_slice(),
                "--lang=smt2",
            )
        };
        let is_query = index % 2 == 1;
        assert_eq!(
            request["protocol"],
            "engineering-assurance.producer-execution-request/v1"
        );
        assert_eq!(request["procedure"], "direct");
        assert_eq!(
            request["producer"]["executable"],
            path.to_str().expect("utf8")
        );
        assert_eq!(
            request["caller"],
            json!({"kind": "quire-analyze.run-package", "version": "1"})
        );
        assert_eq!(request["environment"], json!({}));
        assert_eq!(request["outputs"], json!([]));
        assert_eq!(request["containment"]["profile"], "process-group-v1");
        assert_eq!(request["cancellation"], json!({"kind": "disabled"}));
        assert_eq!(
            request["response"],
            json!({
                "protocol": {"kind": "engineering-assurance.process-evidence", "version": "1"},
                "adapter": {"kind": "engineering-assurance.cli-process-evidence-adapter", "version": "1"},
                "exitCodes": {"kind": "any"},
            })
        );
        if is_query {
            assert_eq!(request["producer"]["version"], version.trim());
            assert_eq!(request["arguments"][0]["value"], text);
            assert_eq!(request["inputs"][0]["role"], "query");
            assert_eq!(request["stdin"]["role"], "query");
        } else {
            assert_eq!(request["producer"]["version"], "unprobed");
            assert_eq!(request["arguments"][0]["value"], arguments[0]);
            assert_eq!(request["inputs"], json!([]));
        }
        let budget = &request["budget"];
        assert!(budget["maxStdoutBytes"].as_u64().expect("stdout") <= 8_388_608);
        assert!(budget["maxStderrBytes"].as_u64().expect("stderr") <= 8_388_608);
        assert_eq!(budget["maxOutputArtifacts"], 0);
        assert_eq!(budget["maxOutputBytes"], 0);
        assert_eq!(budget["maxDescendants"], 16);
        assert_eq!(budget["maxConcurrency"], 1);
    }

    let bytes = fs::read(fixture.output()).expect("report");
    assert_eq!(
        validate_report_document(&bytes),
        Ok(DifferentialDisposition::Agreement)
    );
    assert_eq!(
        fixture.report()["differential"]["agreedStatus"],
        "satisfied"
    );
    fixture.assert_no_leftovers();
}

#[trace("FR-009-AC-3")]
#[test]
fn both_unsat_for_contradictory_clauses_is_refuted() {
    let built = contradictory();
    let fixture = Fixture::new(&built.bytes);
    fixture.both("unsat\n", "unsat\n");
    let run = fixture.run();
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(fixture.report()["differential"]["agreedStatus"], "refuted");
}

/// One row of the result-state table, applied to the Z3 query invocation.
struct Row {
    result: String,
    outcome: &'static str,
    disposition: &'static str,
}

fn rows() -> Vec<Row> {
    let row = |result: String, outcome, disposition| Row {
        result,
        outcome,
        disposition,
    };
    let failed = |reason: Value| state_only(json!({"kind": "failed", "reason": reason}));
    vec![
        row(
            state_only(json!({"kind": "unavailable"})),
            "missing-executable",
            "unavailable",
        ),
        row(
            state_only(json!({"kind": "timed_out"})),
            "timed-out",
            "inconclusive",
        ),
        row(
            failed(json!({"kind": "exit_code", "value": 3})),
            "nonzero-exit",
            "inconclusive",
        ),
        row(
            failed(json!({"kind": "signal", "value": 9})),
            "signaled",
            "inconclusive",
        ),
        row(
            failed(json!({"kind": "stdout_too_large"})),
            "stdout-limit",
            "inconclusive",
        ),
        row(
            failed(json!({"kind": "stderr_too_large"})),
            "stderr-limit",
            "inconclusive",
        ),
        row(
            failed(json!({"kind": "observation"})),
            "io-error",
            "inconclusive",
        ),
        row(
            failed(json!({"kind": "output_artifact"})),
            "io-error",
            "inconclusive",
        ),
        row(
            state_only(json!({"kind": "refused", "reason": "executable"})),
            "spawn-error",
            "inconclusive",
        ),
        row(
            state_only(json!({"kind": "malformed_response"})),
            "malformed-output",
            "inconclusive",
        ),
        row(
            state_only(json!({"kind": "containment_failure"})),
            "cleanup-failed",
            "inconclusive",
        ),
        row(
            state_only(json!({"kind": "cancelled"})),
            "cancelled",
            "inconclusive",
        ),
    ]
}

#[trace("FR-009-AC-4", "FR-009-AC-17")]
#[test]
fn every_non_completed_state_records_its_table_outcome_and_exits_three() {
    let built = consistent();
    for row in rows() {
        let fixture = Fixture::new(&built.bytes);
        fixture.both(&built.model, &built.model);
        fixture.answer("z3", "query", &row.result);
        let run = fixture.run();
        assert_eq!(run.code, Some(3), "{}: {}", row.outcome, run.stderr);
        let report = fixture.report();
        let bytes = fs::read(fixture.output()).expect("report");
        validate_report_document(&bytes).expect("valid report");
        assert_eq!(outcomes(&report).0, row.outcome);
        assert_eq!(report["differential"]["disposition"], row.disposition);
        assert_eq!(report["engines"].as_array().expect("engines").len(), 2);
        assert_eq!(outcomes(&report).1, "sat");
    }
}

#[trace("FR-009-AC-4", "FR-009-AC-17")]
#[test]
fn completed_results_are_classified_and_opposite_answers_exit_three() {
    let built = consistent();
    let cases = [
        ("unknown\n", "unknown"),
        ("(error \"boom\")\n", "solver-error"),
        ("garbage\n", "malformed-output"),
    ];
    for (stdout, outcome) in cases {
        let fixture = Fixture::new(&built.bytes);
        fixture.both(stdout, &built.model);
        let run = fixture.run();
        assert_eq!(run.code, Some(3), "{}", run.stderr);
        assert_eq!(outcomes(&fixture.report()).0, outcome);
    }
    let fixture = Fixture::new(&built.bytes);
    fixture.both(&built.model, "unsat\n");
    assert_eq!(fixture.run().code, Some(3));
    assert_eq!(
        fixture.report()["differential"]["disposition"],
        "disagreement"
    );
    assert_eq!(outcomes(&fixture.report()), ("sat".into(), "unsat".into()));
}

#[trace("FR-009-AC-16")]
#[test]
fn a_bad_or_empty_version_skips_the_query_and_exits_three() {
    let built = consistent();
    let bad_versions = [
        (state_only(json!({"kind": "timed_out"})), "timed-out"),
        (
            state_only(json!({"kind": "unavailable"})),
            "missing-executable",
        ),
        (completed(0, "  \n", ""), "identity-error"),
    ];
    for (version, outcome) in bad_versions {
        let fixture = Fixture::new(&built.bytes);
        fixture.both(&built.model, &built.model);
        fixture.answer("cvc5", "version", &version);
        let run = fixture.run();
        assert_eq!(run.code, Some(3), "{}", run.stderr);
        // z3 version, z3 query, cvc5 version only.
        assert_eq!(fixture.requests().len(), 3);
        assert_eq!(outcomes(&fixture.report()).1, outcome);
        assert!(!fixture.path("stub/data/cvc5.staged").exists());
    }
}

#[trace("FR-009-AC-5")]
#[test]
fn an_unusable_producer_execute_exits_four_without_a_report() {
    let built = consistent();
    let line_ok = completed(0, "unknown\n", "");
    let scenarios: Vec<Scenario> = vec![
        (
            "cannot start",
            Box::new(|fixture| fs::remove_file(fixture.producer()).expect("remove")),
        ),
        (
            "exit status 1",
            Box::new(|fixture| fixture.exit_after("z3", "version", 1)),
        ),
        (
            "exit status 2",
            Box::new(|fixture| fixture.exit_after("z3", "version", 2)),
        ),
        (
            "exit status 7 on the query",
            Box::new(|fixture| fixture.exit_after("cvc5", "query", 7)),
        ),
        (
            "signal",
            Box::new(|fixture| fixture.signal("z3", "version")),
        ),
        (
            "no line",
            Box::new(|fixture| fixture.answer("z3", "version", "")),
        ),
        (
            "two lines",
            Box::new({
                let two = format!("{line_ok}\n{line_ok}");
                move |fixture| fixture.answer("z3", "version", &two)
            }),
        ),
        (
            "garbage",
            Box::new(|fixture| fixture.answer("z3", "version", "not json")),
        ),
        (
            "another document",
            Box::new(|fixture| fixture.answer("z3", "version", "{\"protocol\":\"other\"}")),
        ),
    ];
    for (label, apply) in scenarios {
        let fixture = Fixture::new(&built.bytes);
        apply(&fixture);
        if label == "cannot start" {
            // The directory is still needed to hold the output; the stub is gone.
            assert!(!fixture.producer().exists());
        }
        let run = fixture.run();
        assert_eq!(run.code, Some(4), "{label}: {}", run.stderr);
        assert!(
            run.stderr.contains("producer-execute"),
            "{label}: {}",
            run.stderr
        );
        fixture.assert_no_report();
    }
}

/// A named change to a fixture.
type Scenario = (&'static str, Box<dyn Fn(&Fixture)>);

fn overlong() -> OsString {
    OsString::from(format!("/{}", "a".repeat(5000)))
}

fn non_utf8() -> OsString {
    OsString::from_vec(vec![b'/', b'a', 0xff])
}

#[trace("FR-009-AC-6")]
#[test]
fn refused_packages_exit_two_without_a_report() {
    let built = consistent();

    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    cases.push(("garbage", b"not a package".to_vec()));
    cases.push(("truncated", built.bytes[..built.bytes.len() / 2].to_vec()));

    let mut value: Value = serde_json::from_slice(&built.bytes).expect("json");
    value["package"]["source"]["lock"]["dependency_selections"] =
        json!([{"identity": "library/dependency"}]);
    cases.push(("dependency", serde_json::to_vec(&value).expect("bytes")));

    let mut document = Document::new();
    let p = document.parameter();
    cases.push(("no analysis claim", document.bytes(&[&p])));

    let mut document = Document::new();
    let range = document.range();
    let number = document.parameter_of(&range);
    let claim = document.claim(&number);
    cases.push(("non-Boolean claim", document.bytes(&[&claim])));

    let limit = 1_048_576;
    let mut padded = built.bytes.clone();
    padded.resize(limit + 1, b' ');
    cases.push(("one byte over the limit", padded));

    for (label, bytes) in cases {
        let fixture = Fixture::new(&bytes);
        let run = fixture.run();
        assert_eq!(run.code, Some(2), "{label}: {}", run.stderr);
        assert!(run.stderr.contains("--package"), "{label}: {}", run.stderr);
        assert!(fixture.requests().is_empty(), "{label}");
        fixture.assert_no_report();
    }

    let fixture = Fixture::new(&built.bytes);
    fs::remove_file(&fixture.package).expect("remove");
    let run = fixture.run();
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    fixture.assert_no_report();
}

#[trace("FR-009-AC-6")]
#[test]
fn a_document_at_the_limit_is_not_refused_for_size() {
    let built = consistent();
    let mut padded = built.bytes.clone();
    padded.resize(1_048_576, b' ');
    let fixture = Fixture::new(&padded);
    let run = fixture.run();
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(!run.stderr.contains("read limit"), "{}", run.stderr);
    padded.push(b' ');
    let fixture = Fixture::new(&padded);
    let run = fixture.run();
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(run.stderr.contains("read limit"), "{}", run.stderr);
}

#[trace("FR-009-AC-7")]
#[test]
fn bad_arguments_exit_two_with_the_usage() {
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    let mut cases: Vec<(&str, Vec<OsString>)> = Vec::new();
    for option in [
        "--package",
        "--output",
        "--z3",
        "--cvc5",
        "--producer-execute",
    ] {
        cases.push((option, fixture.without(option)));
        let mut repeated = fixture.arguments();
        repeated.push(option.into());
        repeated.push("/x".into());
        cases.push((option, repeated));
        let mut valueless = fixture.arguments();
        let position = valueless
            .iter()
            .position(|argument| argument == option)
            .expect("option");
        valueless.truncate(position + 1);
        cases.push((option, valueless));
    }
    let mut unknown = fixture.arguments();
    unknown.push("--unknown".into());
    unknown.push("x".into());
    cases.push(("unknown", unknown));
    cases.push(("none", vec!["run-package".into()]));
    for (label, arguments) in cases {
        let run = fixture.run_with(&arguments);
        assert_eq!(run.code, Some(2), "{label}: {}", run.stderr);
        assert!(run.stderr.contains("usage:"), "{label}: {}", run.stderr);
        assert!(run.stdout.is_empty());
        fixture.assert_no_report();
    }
}

#[trace("FR-009-AC-8")]
#[test]
fn bad_executable_paths_exit_two() {
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    let bad = [
        ("relative", OsString::from("relative/solver")),
        ("empty", OsString::new()),
        ("non-utf8", non_utf8()),
        ("over-long", overlong()),
    ];
    for option in ["--z3", "--cvc5", "--producer-execute"] {
        for (label, value) in &bad {
            let run = fixture.run_with(&fixture.with(option, value.clone()));
            assert_eq!(run.code, Some(2), "{option} {label}: {}", run.stderr);
            assert!(fixture.requests().is_empty());
            fixture.assert_no_report();
        }
    }
}

#[trace("FR-009-AC-9")]
#[test]
fn a_bad_destination_exits_four_and_keeps_existing_bytes() {
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    fixture.both(&built.model, &built.model);

    fs::write(fixture.output(), b"keep").expect("existing");
    let run = fixture.run();
    assert_eq!(run.code, Some(4), "{}", run.stderr);
    assert_eq!(fs::read(fixture.output()).expect("bytes"), b"keep");
    assert_eq!(directory_names(&fixture.path("out")), ["report.json"]);
    fs::remove_file(fixture.output()).expect("remove");

    let missing = fixture.path("missing/report.json");
    let run = fixture.run_with(&fixture.with("--output", missing.clone()));
    assert_eq!(run.code, Some(4), "{}", run.stderr);
    assert!(!missing.exists());

    let out = fixture.path("out");
    fs::set_permissions(&out, fs::Permissions::from_mode(0o500)).expect("read-only");
    if fs::write(out.join("probe"), b"").is_ok() {
        eprintln!("skipped the unwritable-parent case: directory permissions are not enforced");
    } else {
        let run = fixture.run();
        assert_eq!(run.code, Some(4), "{}", run.stderr);
        assert!(directory_names(&out).is_empty());
    }
    fs::set_permissions(&out, fs::Permissions::from_mode(0o700)).expect("restore");
}

#[trace("FR-009-AC-11", "FR-009-AC-12", "FR-009-AC-26")]
#[test]
fn identical_inputs_give_identical_bytes_and_leave_no_trace() {
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    fixture.both(&built.model, &built.model);
    let before = (
        directory_names(&fixture.path("pkg")),
        fs::read(&fixture.package).expect("package"),
        fs::metadata(&fixture.package)
            .expect("metadata")
            .modified()
            .expect("mtime"),
    );

    assert_eq!(fixture.run().code, Some(0));
    let first = fs::read(fixture.output()).expect("report");
    fs::remove_file(fixture.output()).expect("remove");
    assert_eq!(fixture.run().code, Some(0));
    assert_eq!(first, fs::read(fixture.output()).expect("report"));

    let after = (
        directory_names(&fixture.path("pkg")),
        fs::read(&fixture.package).expect("package"),
        fs::metadata(&fixture.package)
            .expect("metadata")
            .modified()
            .expect("mtime"),
    );
    assert_eq!(before, after);
    fixture.assert_no_leftovers();

    // Every invocation saw a capability root of mode 0700 holding the staged query.
    let modes = fs::read_to_string(fixture.path("stub/data/modes.log")).expect("modes");
    assert_eq!(modes.lines().count(), 8);
    assert!(modes.lines().all(|mode| mode == "700"), "{modes}");
    let staged = fs::read_to_string(fixture.path("stub/data/z3.staged")).expect("staged query");
    assert!(staged.contains("(check-sat)"), "{staged}");
}

#[trace("FR-009-AC-12")]
#[test]
fn the_capability_root_is_gone_after_every_exit_path() {
    let built = consistent();
    let satisfied = Fixture::new(&built.bytes);
    satisfied.both(&built.model, &built.model);
    assert_eq!(satisfied.run().code, Some(0));
    satisfied.assert_no_leftovers();

    let non_conclusive = Fixture::new(&built.bytes);
    assert_eq!(non_conclusive.run().code, Some(3));
    non_conclusive.assert_no_leftovers();

    let usage = Fixture::new(&built.bytes);
    assert_eq!(usage.run_with(&usage.without("--z3")).code, Some(2));
    usage.assert_no_report();

    let failed = Fixture::new(&built.bytes);
    failed.exit_after("cvc5", "version", 9);
    assert_eq!(failed.run().code, Some(4));
    failed.assert_no_report();
    assert_eq!(
        fs::read_to_string(failed.path("stub/data/modes.log"))
            .expect("modes")
            .lines()
            .count(),
        3
    );
}

#[trace("FR-009-AC-13")]
#[test]
fn help_lists_the_arguments_and_exit_classes() {
    let run = finish({
        let mut command = Command::new(BINARY);
        command.args(["run-package", "--help"]).stdin(Stdio::null());
        command
    });
    assert_eq!(run.code, Some(0));
    for text in [
        "--package",
        "--output",
        "--z3",
        "--cvc5",
        "--producer-execute",
        "  0  ",
        "  1  ",
        "  2  ",
        "  3  ",
        "  4  ",
    ] {
        assert!(run.stdout.contains(text), "{text}: {}", run.stdout);
    }
}

#[trace("FR-009-AC-18")]
#[test]
fn a_symlinked_component_exits_two() {
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    fixture.both(&built.model, &built.model);
    symlink(fixture.path("solvers"), fixture.path("solver-link")).expect("link");
    symlink(fixture.path("stub"), fixture.path("stub-link")).expect("link");
    symlink(fixture.path("out"), fixture.path("out-link")).expect("link");
    symlink(fixture.producer(), fixture.path("producer-link")).expect("link");

    let cases = [
        ("--z3", fixture.path("solver-link/z3")),
        ("--cvc5", fixture.path("solver-link/cvc5")),
        ("--producer-execute", fixture.path("stub-link/producer")),
        ("--producer-execute", fixture.path("producer-link")),
        ("--output", fixture.path("out-link/report.json")),
    ];
    for (option, value) in cases {
        let run = fixture.run_with(&fixture.with(option, value.clone()));
        assert_eq!(run.code, Some(2), "{option}: {}", run.stderr);
        assert!(
            run.stderr.contains("symbolic link"),
            "{option}: {}",
            run.stderr
        );
        assert!(fixture.requests().is_empty());
        fixture.assert_no_report();
    }
}

/// A package whose claim is a Boolean term the translation cannot lower, and the claim node.
fn untranslatable() -> (Vec<u8>, String) {
    let mut document = Document::new();
    let range = document.range();
    let a = document.parameter_of(&range);
    let b = document.parameter_of(&range);
    let less = document.application("quire.op.integer.lt", &[&a, &b], "scalar_type/boolean");
    let claim = document.claim(&less);
    (document.bytes(&[&claim]), less)
}

fn node_index(bytes: &[u8], label: &str) -> usize {
    let value: Value = serde_json::from_slice(bytes).expect("json");
    let key = json!({"package": "test/pkg", "path": [label], "ordinal": 0});
    value["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .position(|node| node["node_id"] == key)
        .expect("node")
}

#[trace("FR-009-AC-19", "FR-009-AC-23")]
#[test]
fn an_untranslatable_claim_exits_two_naming_the_node() {
    let (bytes, less) = untranslatable();
    let fixture = Fixture::new(&bytes);
    let run = fixture.run();
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    let node = format!("semantic_graph.nodes[{}]", node_index(&bytes, &less));
    assert!(run.stderr.contains(&node), "{node}: {}", run.stderr);
    assert!(fixture.requests().is_empty());
    fixture.assert_no_report();
}

#[trace("FR-009-AC-23")]
#[test]
fn a_shared_subterm_chain_exits_two_without_hanging() {
    let mut document = Document::new();
    let p = document.parameter();
    let mut top = document.op(AND, &[&p, &p]);
    for _ in 1..61 {
        top = document.op(AND, &[&top, &top]);
    }
    let claim = document.claim(&top);
    let bytes = document.bytes(&[&claim]);
    let fixture = Fixture::new(&bytes);
    let started = std::time::Instant::now();
    let run = fixture.run();
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(
        run.stderr.contains("semantic_graph.nodes["),
        "{}",
        run.stderr
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    fixture.assert_no_report();
}

#[trace("FR-009-AC-24")]
#[test]
fn a_claim_without_occurrences_exits_two_naming_the_node() {
    let mut document = Document::new();
    let p = document.parameter();
    let claim = document.claim(&p);
    let bytes = document.bytes(&[&claim]);
    let node = node_index(&bytes, &claim);
    let mut value: Value = serde_json::from_slice(&bytes).expect("json");
    value["semantic_graph"]["nodes"][node]["occurrences"] = json!([]);
    value["source_map"]["nodes"][node]["occurrences"] = json!([]);
    value["source_map"]["requested_items"][0]["occurrences"] = json!([]);
    let fixture = Fixture::new(&serde_json::to_vec(&value).expect("bytes"));
    let run = fixture.run();
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(
        run.stderr
            .contains(&format!("semantic_graph.nodes[{node}]")),
        "{}",
        run.stderr
    );
    fixture.assert_no_report();
}

#[trace("NFR-003-AC-2")]
#[test]
fn closed_stdin_and_a_minimal_environment_give_the_same_report() {
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    fixture.both(&built.model, &built.model);
    let mut command = fixture.command(&fixture.arguments());
    command.stdin(Stdio::piped());
    let mut child = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    drop(child.stdin.take());
    let normal_code = child.wait().expect("wait").code();
    let normal_report = fs::read(fixture.output()).expect("report");
    fs::remove_file(fixture.output()).expect("remove");

    let mut command = fixture.command(&fixture.arguments());
    command.env_clear();
    let hermetic = finish(command);
    assert_eq!(normal_code, hermetic.code);
    assert_eq!(normal_report, fs::read(fixture.output()).expect("report"));
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| fs::canonicalize(candidate).ok())
}

fn real_producer_execute() -> Option<PathBuf> {
    find_executable("engineering-assurance")
}

/// Every logged request validates against the schema `producer-execute --schema request` prints.
#[trace("FR-009-AC-15")]
#[test]
fn logged_requests_validate_against_the_published_request_schema() {
    let Some(executable) = real_producer_execute() else {
        eprintln!("skipped: no engineering-assurance executable on PATH");
        return;
    };
    let schema = Command::new(&executable)
        .args(["producer-execute", "--schema", "request"])
        .output()
        .expect("schema");
    assert!(schema.status.success());
    let schema: Value = serde_json::from_slice(&schema.stdout).expect("schema json");
    let compiled = jsonschema::JSONSchema::compile(&schema).expect("compile schema");

    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    fixture.both(&built.model, &built.model);
    assert_eq!(fixture.run().code, Some(0));
    let requests = fixture.requests();
    assert_eq!(requests.len(), 4);
    for request in &requests {
        if let Err(errors) = compiled.validate(request) {
            let errors = errors.map(|error| error.to_string()).collect::<Vec<_>>();
            panic!("request is invalid: {errors:?}");
        }
    }
}

fn write_solver(path: &Path, version: &str, query_body: &str) {
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in -version|--version) printf '{version}\\n'; exit 0;; esac\n{query_body}\n"
    );
    fs::write(path, script).expect("solver");
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("executable");
}

/// NFR-003-AC-1: a stalled and a flooding solver run through the real `producer-execute` give a
/// published report with typed non-conclusive outcomes and exit 3.
#[trace("NFR-003-AC-1")]
#[test]
fn a_stalled_and_a_flooding_solver_are_non_conclusive_through_the_real_executor() {
    let Some(executable) = real_producer_execute() else {
        eprintln!("skipped: no engineering-assurance executable on PATH");
        return;
    };
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    write_solver(&fixture.z3(), "Z3 version stub", "while :; do :; done");
    write_solver(
        &fixture.cvc5(),
        "cvc5 version stub",
        "while :; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\\n'; done",
    );
    let run = fixture.run_with(&fixture.with("--producer-execute", executable));
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    let report = fixture.report();
    let bytes = fs::read(fixture.output()).expect("report");
    validate_report_document(&bytes).expect("valid report");
    assert_eq!(
        outcomes(&report),
        ("timed-out".into(), "stdout-limit".into())
    );
    fixture.assert_no_leftovers();
}

/// FR-009-AC-14: the real executor with real Z3 and cvc5.
#[trace("FR-009-AC-14")]
#[test]
fn real_executor_and_real_solvers_give_a_conclusive_report() {
    let (Some(executable), Some(z3), Some(cvc5)) = (
        real_producer_execute(),
        find_executable("z3"),
        find_executable("cvc5"),
    ) else {
        eprintln!("skipped: engineering-assurance, z3 and cvc5 must all be on PATH");
        return;
    };
    let built = consistent();
    let fixture = Fixture::new(&built.bytes);
    let mut arguments = fixture.with("--producer-execute", executable);
    for (option, value) in [("--z3", z3), ("--cvc5", cvc5)] {
        let position = arguments.iter().position(|a| a == option).expect("option");
        arguments[position + 1] = value.into();
    }
    let run = fixture.run_with(&arguments);
    assert!(
        matches!(run.code, Some(0 | 1)),
        "{:?} {}",
        run.code,
        run.stderr
    );
    let bytes = fs::read(fixture.output()).expect("report");
    validate_report_document(&bytes).expect("valid report");
}
