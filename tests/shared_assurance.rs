//! Requirement-tagged tests for the shared assurance intake path (issue #25).
//!
//! These tests read what `make assurance-inputs` produced. They do not produce
//! it: a test that can create its own inputs can create a green run out of
//! nothing. An absent input is an assertion failure naming that target, never a
//! skip — a skipped check and a passing one are indistinguishable in a summary,
//! and only one of them is true.

#![cfg(target_os = "linux")]

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use serde_json::Value;

const DECLARATION: &str = include_str!("../assurance/change-assurance.json");
const PINS: &str = include_str!("../assurance/pins.json");
const MAKEFILE: &str = include_str!("../Makefile");
const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");
const FR_006: &str = include_str!("../spec/functional/FR-006-shared-assurance-intake.md");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn head_revision() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root())
        .output()
        .expect("git rev-parse");
    String::from_utf8(output.stdout)
        .expect("utf-8 revision")
        .trim()
        .to_owned()
}

/// Read one producer result, or fail naming the target that makes it.
fn producer_output(name: &str) -> String {
    let path = root().join("target/assurance").join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{} is absent ({error}). Run `make assurance-inputs`. This is a failure and \
             not a skip: these tests read producer output and never create it.",
            path.display()
        )
    })
}

fn producer_json(name: &str) -> Value {
    serde_json::from_str(&producer_output(name)).expect("producer output is JSON")
}

/// The chain report, computed once. Driving it per test would be four identical runs.
fn chain_report() -> &'static Value {
    static REPORT: OnceLock<Value> = OnceLock::new();
    REPORT.get_or_init(|| {
        let revision = head_revision();
        let output = Command::new("python3")
            .args([
                "scripts/assurance_chain.py",
                "--candidate-revision",
                &revision,
                "--json",
            ])
            .current_dir(root())
            .output()
            .expect("failed to run the assurance chain");
        assert!(
            output.status.success(),
            "the assurance chain did not pass:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("chain report is JSON")
    })
}

/// Write an executable shim for each name that records every invocation.
///
/// The log is the point. A shim that is never consulted and a producer that is
/// never run look identical from the outside, so the shims write down every call
/// and the test reads the file rather than assuming.
///
/// A version query is answered rather than refused, and deliberately so. Asking
/// a tool its version is an observation — it is what the compatibility matrix's
/// own `observe` column does — and it is not the thing this test forbids. What
/// is forbidden is asking a tool to build, compile, test, or replay anything.
/// Every such invocation is logged and the log must be empty.
///
/// The exemption is "some argument is a version flag", not "the first one is",
/// because the MSRV proof must attest the pinned toolchain and the only way to
/// read it is `rustup run 1.75.0 cargo --version`. Selecting a toolchain and
/// then asking for a version is still asking for a version. The widening costs
/// nothing: `cargo build`, `cargo check` and `quire coverage` carry no version
/// flag, so they are still logged — verified by injecting both into the driver
/// and watching this test fail with their invocations named.
fn producer_shims(directory: &Path, names: &[&str]) -> PathBuf {
    fs::create_dir_all(directory).unwrap();
    let log = directory.join("invocations.log");
    let _ = fs::remove_file(&log);
    for name in names {
        let path = directory.join(name);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\n\
                 for argument in \"$@\"; do\n\
                   case \"$argument\" in\n\
                   --version|-V) echo \"{name} 9.9.9 (shim)\"; exit 0 ;;\n\
                   esac\n\
                 done\n\
                 echo \"$0 $@\" >> {}\n\
                 exit 97\n",
                log.display()
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    log
}

fn run_chain_with_path(shims: &Path) -> std::process::Output {
    let inherited = std::env::var("PATH").unwrap_or_default();
    let revision = head_revision();
    Command::new("python3")
        .args([
            "scripts/assurance_chain.py",
            "--candidate-revision",
            &revision,
        ])
        .current_dir(root())
        .env("PATH", format!("{}:{inherited}", shims.display()))
        .output()
        .expect("failed to run the assurance chain")
}

/// TC-011: the driver runs the tool and never the producer.
/// Trace: TC-011, FR-006-AC-2, FR-006-AC-3
#[test]
fn the_chain_never_executes_a_producer_and_the_probe_can_prove_it() {
    // Two runs, because one proves nothing.
    //
    // Run A replaces every producer — cargo, rustup, rustc, and quire — with a
    // stub that logs and fails. The chain must finish, and the log must be empty:
    // not one producer was invoked.
    //
    // Run B is the control. It stubs `quoin`, which the chain is supposed to run,
    // and requires the chain to fail and the log to be non-empty. Without it, an
    // empty log in run A would be equally consistent with PATH never being
    // consulted at all, which is exactly how the Wave 1 version of this test read
    // before an adversarial review caught it.
    let producers = root().join("target/producer-shims");
    let producer_log = producer_shims(&producers, &["cargo", "rustup", "rustc", "quire"]);
    let output = run_chain_with_path(&producers);
    let logged = fs::read_to_string(&producer_log).unwrap_or_default();
    assert!(
        output.status.success(),
        "the assurance chain failed with producers stubbed, which means it ran one:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        logged.trim().is_empty(),
        "the assurance driver asked a producer to do work, not just to name its version:\n{logged}"
    );

    let tools = root().join("target/tool-shims");
    let tool_log = producer_shims(&tools, &["quoin"]);
    let control = run_chain_with_path(&tools);
    let tool_logged = fs::read_to_string(&tool_log).unwrap_or_default();
    assert!(
        !tool_logged.trim().is_empty(),
        "stubbing quoin produced no invocation, so PATH is not being consulted by \
         the subprocess and the run above proves nothing"
    );
    assert!(
        !control.status.success(),
        "the chain succeeded with quoin stubbed out, so it is not actually using it"
    );
}

/// TC-011: every attestation result is read from the producer's bytes.
/// Trace: TC-011, FR-006-AC-2, FR-006-AC-5
#[test]
fn attestation_results_are_read_from_producer_output() {
    let report = chain_report();
    let observed = report["observedResults"]
        .as_object()
        .expect("observed results object");
    assert_eq!(observed.len(), 5, "every declared proof reports a result");

    // Each result must equal the verdict the producer's own document states, so
    // a result that was assumed rather than read cannot agree by accident.
    assert_eq!(
        observed["PROOF-solver-state-census"],
        producer_json("solver-state-census.json")["outcome"],
        "the census attestation must state what the census document says"
    );
    assert_eq!(
        observed["PROOF-engine-availability"],
        producer_json("engine-availability.json")["outcome"],
        "the engine attestation must state what the availability document says"
    );
    assert_eq!(
        observed["PROOF-shared-pins"],
        producer_json("shared-pins.json")["outcome"],
        "the pin attestation must state what the pin document says"
    );

    // Not every result may be `passed`. A chain in which nothing can be anything
    // but a pass has no way to report the state this repository most needs.
    let distinct: BTreeSet<&str> = observed.values().filter_map(Value::as_str).collect();
    assert!(
        distinct.len() > 1,
        "every proof reported the same result, so no state is distinguishable: {distinct:?}"
    );
}

/// TC-011: an engine that was not there stays unavailable all the way to the receipt.
/// Trace: TC-011, FR-006-AC-5
#[test]
fn an_absent_engine_is_unavailable_and_never_a_decision() {
    let availability = producer_json("engine-availability.json");
    let outcome = availability["outcome"].as_str().expect("outcome");
    assert!(
        matches!(outcome, "passed" | "unavailable"),
        "an availability probe reports presence or absence, never a verdict: {outcome}"
    );

    if outcome == "unavailable" {
        assert_ne!(
            outcome, "failed",
            "an absent engine did not fail; it did not run"
        );
        let report = chain_report();
        let carried = report["scenarios"]
            .as_array()
            .expect("scenarios")
            .iter()
            .find(|scenario| scenario["scenario"] == "unavailable-survives-to-the-receipt")
            .expect("the unavailable-propagation scenario must run");
        assert_eq!(
            carried["matched"],
            Value::Bool(true),
            "unavailability did not survive to the receipt: {carried}"
        );
    }
}

/// TC-011: the census keeps every non-conclusive solver state distinct.
/// Trace: TC-011, TC-005, FR-006-AC-5, NFR-002-AC-2
#[test]
fn every_non_conclusive_solver_state_stays_its_own_answer() {
    let census = producer_json("solver-state-census.json");
    assert_eq!(
        census["outcome"], "passed",
        "the census must have matched: {census}"
    );

    let states = census["solverStates"].as_array().expect("solver states");
    let observed: BTreeSet<&str> = states
        .iter()
        .filter_map(|state| state["observedOutcome"].as_str())
        .collect();
    assert_eq!(
        observed.len(),
        states.len(),
        "two provoked conditions produced the same outcome, so a state was collapsed"
    );

    // sat and unsat are the only conclusive answers. Everything else the solver
    // can say is a way of not deciding, and must not read as a decision.
    for state in states {
        let outcome = state["observedOutcome"].as_str().expect("outcome");
        let conclusive = state["conclusive"].as_bool().expect("conclusive");
        assert_eq!(
            conclusive,
            matches!(outcome, "sat" | "unsat"),
            "{outcome} is reported as conclusive={conclusive}, which is not its meaning"
        );

        // The analysis status is checked against what the row declared, not
        // merely recorded next to it. Recording it unchecked is how every
        // timed-out, signaled or version-mismatched solver could report the
        // status of a decided analysis with nothing noticing.
        let expected = state["expectedAnalysisStatus"]
            .as_str()
            .expect("every row declares the status its outcome must produce");
        let observed = state["analysisStatus"].as_str().expect("analysis status");
        assert_eq!(
            observed, expected,
            "{outcome} produced analysis status {observed}, not the declared {expected}"
        );
        assert_eq!(
            observed == "satisfied" || observed == "refuted",
            matches!(outcome, "sat" | "unsat"),
            "{outcome} produced the analysis status of a decided analysis ({observed})"
        );
    }

    // The denominator is published and checked. A bare count of distinct
    // outcomes cannot be read as coverage without knowing how many there are.
    let total = census["totalOutcomes"].as_u64().expect("total outcomes");
    let distinct = census["distinctOutcomes"]
        .as_u64()
        .expect("distinct outcomes");
    let unexercised = census["unexercisedOutcomes"]
        .as_array()
        .expect("unexercised outcomes");
    assert_eq!(
        distinct + unexercised.len() as u64,
        total,
        "the census arithmetic does not close: {distinct} + {} != {total}",
        unexercised.len()
    );
    assert!(
        distinct >= 20,
        "the census reached only {distinct} of {total} solver outcomes"
    );

    // All four differential dispositions must be reachable. A disposition that
    // cannot be produced is a disposition that cannot be distinguished.
    let dispositions: BTreeSet<&str> = census["differentialDispositions"]
        .as_array()
        .expect("dispositions")
        .iter()
        .filter(|case| case["matched"] == Value::Bool(true))
        .filter_map(|case| case["observedDisposition"].as_str())
        .collect();
    assert_eq!(
        dispositions,
        BTreeSet::from(["agreement", "disagreement", "inconclusive", "unavailable"]),
        "a differential disposition was not demonstrated"
    );
}

/// TC-011: the report validator accepts the authoritative bytes and refuses each tamper.
/// Trace: TC-011, TC-007, FR-006-AC-2, NFR-002-AC-3
#[test]
fn report_identity_accepts_the_real_bytes_and_refuses_every_tamper() {
    let identity = &producer_json("solver-state-census.json")["reportIdentity"];
    assert_eq!(
        identity["authoritativeBytesAccepted"],
        Value::Bool(true),
        "the positive control failed, so every refusal below proves nothing"
    );
    let probes = identity["tamperProbes"].as_array().expect("tamper probes");
    assert!(probes.len() >= 5, "the tamper corpus shrank: {probes:?}");
    for probe in probes {
        assert_eq!(
            probe["refused"],
            Value::Bool(true),
            "a tampered report was accepted: {probe}"
        );
    }
}

/// TC-012: no local generic assurance machinery remains in the execution path.
/// Trace: TC-012, FR-006-AC-6
#[test]
fn no_local_generic_machinery_remains() {
    // `make ci` must not run a repository-local evidence verifier, and no script
    // may reference one. The retained records and their reader are gone too:
    // engineering-assurance#7 released the preservation constraint for the
    // pre-stable phase, so an `evidence/` tree reappearing here is a local
    // retention store returning, not a record being preserved.
    assert!(
        !MAKEFILE.contains("verify-evidence:"),
        "a repository-local evidence verifier target is still defined"
    );
    assert!(
        !root().join("evidence").exists(),
        "a local evidence retention tree is back; this repository retains nothing of its own"
    );

    let scripts = root().join("scripts");
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&scripts).expect("scripts directory") {
        let path = entry.expect("script entry").path();
        if path.is_file() {
            names.insert(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    // A closed allow-list, because a blocklist is not a census. A blocklist of
    // six names is defeated by a `.py` extension or a `_v2` suffix; this fails
    // on any script the repository has not declared, which is the actual claim.
    let allowed = BTreeSet::from([
        "assurance_chain.py",
        "check_shared_pins.py",
        "check_unsafe_comments.sh",
        "unsafe_comment_baseline.txt",
    ]);
    let names: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    assert_eq!(
        names, allowed,
        "scripts/ is not what this repository declares it to be; an undeclared \
         script is how a generic collector, envelope builder or verifier returns"
    );
}

/// TC-011: a producer that reports failure, or nothing readable, stops the chain.
///
/// These are the two steps TC-011 declares and nothing implemented — which is
/// how FG-01 survived: the one declared step that would have caught a chain
/// reporting `passed` over degraded proofs was the one never automated.
///
/// Each case runs against a mutated COPY of producer output, so the real
/// producer results are never touched and the driver still creates nothing.
///
/// Trace: TC-011, FR-006-AC-2, FR-006-AC-5
#[test]
fn a_producer_that_did_not_pass_cannot_produce_a_green_chain() {
    let source = root().join("target/assurance");
    assert!(
        source.is_dir(),
        "target/assurance is absent. Run `make assurance-inputs`."
    );

    // Positive control first. The same harness, unmutated, must go green — or
    // every refusal below is just the harness being broken.
    let (code, _) = chain_against(&source, "control", |_| {});
    assert_eq!(
        code,
        Some(0),
        "the unmutated copy did not pass, so nothing below means anything"
    );

    // Every producer reports failure. Exit must be non-zero.
    let (code, output) = chain_against(&source, "all-failed", |directory| {
        for name in [
            "solver-state-census.json",
            "engine-availability.json",
            "shared-pins.json",
        ] {
            rewrite_outcome(&directory.join(name), "failed");
        }
    });
    assert_ne!(
        code,
        Some(0),
        "every producer reported failure and the chain was green:\n{output}"
    );

    // Results that are neither pass nor failure — the states FG-01 let through.
    for degraded in ["inconclusive", "not_computed", "unavailable"] {
        let (code, output) = chain_against(&source, degraded, |directory| {
            rewrite_outcome(&directory.join("solver-state-census.json"), degraded);
        });
        assert_ne!(
            code,
            Some(0),
            "the census established nothing ({degraded}) and the chain was green:\n{output}"
        );
    }

    // Producer output that is not readable at all must exit 2 — an environment
    // fact, distinct from a proof that ran and did not match.
    let (code, output) = chain_against(&source, "unreadable", |directory| {
        fs::write(
            directory.join("solver-state-census.json"),
            b"not json at all",
        )
        .unwrap();
    });
    assert_eq!(
        code,
        Some(2),
        "unreadable producer output did not exit 2:\n{output}"
    );

    // An outcome the adapter's table does not name must be refused, not defaulted.
    let (code, output) = chain_against(&source, "unlisted", |directory| {
        rewrite_outcome(&directory.join("shared-pins.json"), "probably-fine");
    });
    assert_eq!(
        code,
        Some(2),
        "an unlisted outcome was not refused:\n{output}"
    );
}

fn rewrite_outcome(path: &Path, outcome: &str) {
    let mut document: Value =
        serde_json::from_slice(&fs::read(path).expect("producer output")).expect("JSON");
    document["outcome"] = Value::String(outcome.to_owned());
    fs::write(path, serde_json::to_vec(&document).expect("rewritten")).unwrap();
}

/// Copy producer output, mutate the copy, and run the chain against it.
fn chain_against(source: &Path, label: &str, mutate: impl FnOnce(&Path)) -> (Option<i32>, String) {
    let directory = root().join("target/assurance-probes").join(label);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    for entry in fs::read_dir(source).expect("producer output") {
        let entry = entry.expect("entry");
        if entry.path().is_file() {
            fs::copy(entry.path(), directory.join(entry.file_name())).unwrap();
        }
    }
    mutate(&directory);
    let output = Command::new("python3")
        .args([
            "scripts/assurance_chain.py",
            "--candidate-revision",
            &head_revision(),
        ])
        .current_dir(root())
        .env("QUIRE_ANALYZE_ASSURANCE_DIR", &directory)
        .output()
        .expect("assurance chain");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code(), combined)
}

/// TC-011: the declaration states what it derives, and derives nothing else.
/// Trace: TC-011, FR-006-AC-1
#[test]
fn the_declaration_is_a_statement_and_not_a_discovery() {
    let declaration: Value = serde_json::from_str(DECLARATION).expect("declaration is JSON");
    assert!(
        declaration["record"].get("digest").is_none(),
        "the declaration states a digest quoin is supposed to compute"
    );
    let derived = declaration["derived_fields"]
        .as_array()
        .expect("derived fields");
    assert!(
        derived.len() >= 5,
        "the derived-field list must name every field that is not stated"
    );

    // Every source the declaration names must exist, or the record is a claim
    // about files that are not there.
    for (id, path) in declaration["sources"].as_object().expect("sources") {
        assert!(
            root().join(path.as_str().expect("source path")).is_file(),
            "source {id} names {path}, which does not exist"
        );
    }

    // Every proof's configuration must exist too, since its digest is sealed.
    for obligation in declaration["record"]["definition"]["proof_obligations"]
        .as_array()
        .expect("proof obligations")
    {
        let configuration = obligation["configuration"].as_str().expect("configuration");
        assert!(
            root().join(configuration).is_file(),
            "{} names configuration {configuration}, which does not exist",
            obligation["proof_id"]
        );
    }
}

/// TC-011: the adopted pins are the accepted ones and the mirror is absent.
/// Trace: TC-011, FR-006-AC-1
#[test]
fn adopted_pins_are_classified_upstream_and_name_no_mirror() {
    let pins = producer_json("shared-pins.json");
    assert_eq!(
        pins["outcome"], "passed",
        "the pin gate did not pass: {pins}"
    );
    for component in pins["components"].as_array().expect("components") {
        assert_eq!(
            component["verdict"], "compatible",
            "a component is not the pinned version: {component}"
        );
    }
    assert_eq!(
        pins["acceptance_recorded_here"],
        Value::Bool(false),
        "this repository must not record an acceptance decision of its own"
    );

    // The workflow must carry the accepted quoin, not the one the matrix names
    // incompatible.
    assert!(
        CI_WORKFLOW.contains("@agent-ix/quoin@0.23.1"),
        "the hosted workflow does not pin the accepted quoin"
    );
    assert!(
        !CI_WORKFLOW.contains("@agent-ix/quoin@0.22.5"),
        "the hosted workflow still pins quoin 0.22.5, which the matrix names incompatible"
    );
    assert!(
        CI_WORKFLOW.contains("workflow_dispatch"),
        "hosted CI must remain manual-dispatch only"
    );

    // The mirror must not appear in an installable position anywhere.
    let pins_document: Value = serde_json::from_str(PINS).expect("pins are JSON");
    assert!(
        !pins_document["engineering_assurance"]["requirement"]
            .as_str()
            .expect("requirement")
            .contains("npm.ix"),
        "the assurance requirement names the internal mirror"
    );

    // FR-006 must record the acceptance gap rather than paper over it.
    assert!(
        FR_006.contains("engineering-assurance#20"),
        "FR-006 does not record the acceptance packaging gap"
    );
}

/// TC-011: the chain demonstrates each state with a case that ran and matched.
/// Trace: TC-011, FR-006-AC-5
#[test]
fn each_demonstrated_state_is_backed_by_a_case_that_matched() {
    let report = chain_report();
    assert_eq!(
        report["casesMatched"], report["casesTotal"],
        "a chain case did not match: {}",
        report["mismatches"]
    );

    let demonstrated: BTreeSet<&str> = report["statesDemonstrated"]
        .as_array()
        .expect("states")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    // Assert against an independent list of state -> the case that must produce
    // it. Recomputing `statesDemonstrated` with the same expression the driver
    // used would be a tautology: it cannot fail, and it would accept a case
    // claiming a state no case produced.
    let required: [(&str, &str); 6] = [
        ("passed", "honest-PROOF-solver-state-census"),
        ("unavailable", "honest-PROOF-engine-availability"),
        ("tampered", "tampered-retained-bytes-refused"),
        ("malformed", "presupplied-record-digest-refused"),
        ("unsupported", "unlisted-producer-outcome-refused"),
        ("inconclusive", "receipt-without-a-human-decision"),
    ];
    let cases: Vec<&Value> = report["scenarios"]
        .as_array()
        .expect("scenarios")
        .iter()
        .chain(report["controls"].as_array().expect("controls"))
        .collect();
    for (state, scenario) in required {
        let case = cases
            .iter()
            .find(|case| case["scenario"] == scenario)
            .unwrap_or_else(|| panic!("{scenario} did not run, so {state} rests on nothing"));
        assert_eq!(
            case["matched"],
            Value::Bool(true),
            "{scenario} did not match, so {state} is not demonstrated"
        );
        assert_eq!(
            case["demonstrates"].as_str(),
            Some(state),
            "{scenario} was supposed to demonstrate {state}"
        );
        assert!(
            demonstrated.contains(state),
            "{state} is missing from the reported states: {demonstrated:?}"
        );
    }

    // Nothing may be claimed that is not on the list above or produced by an
    // honest proof result, so a fabricated label cannot slip into the report.
    let allowed: BTreeSet<&str> = required
        .iter()
        .map(|(state, _)| *state)
        .chain(["not_computed", "failed", "partial"])
        .collect();
    for state in &demonstrated {
        assert!(
            allowed.contains(state),
            "{state} is claimed as demonstrated but is not a state this chain defines"
        );
    }
}
