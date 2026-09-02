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
/// `--version` is answered rather than refused, and deliberately so. Asking a
/// tool its version is an observation — it is what the compatibility matrix's own
/// `observe` column does — and it is not the thing this test forbids. What is
/// forbidden is asking a tool to build, compile, test, or replay anything. Every
/// such invocation is logged and the log must be empty.
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
                 case \"$1\" in\n\
                 --version|-V) echo \"{name} 9.9.9 (shim)\"; exit 0 ;;\n\
                 esac\n\
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
    assert_eq!(observed.len(), 6, "every declared proof reports a result");

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
        observed["PROOF-legacy-compatibility"],
        producer_json("legacy-compatibility.json")["outcome"],
        "the compatibility attestation must state what the view document says"
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
    }

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

/// TC-012: retained evidence is read unmodified and the mapping's answer stands.
/// Trace: TC-012, FR-006-AC-4, NFR-002-AC-3
#[test]
fn retained_evidence_is_read_through_the_pinned_mapping_without_being_changed() {
    let view = producer_json("legacy-compatibility.json");
    assert_eq!(
        view["outcome"], "passed",
        "the compatibility view did not match: {view}"
    );

    let retained = &view["retained"];
    assert_eq!(
        retained["censusMismatches"]
            .as_array()
            .expect("census")
            .len(),
        0,
        "a retained record changed or went missing: {retained}"
    );
    let entries = retained["entries"].as_array().expect("entries");
    assert!(
        !entries.is_empty(),
        "the retained census is empty, so it measured nothing"
    );
    for entry in entries {
        assert_eq!(
            entry["sourceDigestMatches"],
            Value::Bool(true),
            "the mapping changed a record's source identity: {entry}"
        );
        // The honest answer for this repository. It is asserted rather than
        // tolerated, so that a future record which the mapping CAN read shows up
        // here as a change instead of passing silently.
        assert_eq!(
            entry["outcome"], "unreadable",
            "a retained record mapped to something other than the refusal this \
             repository's Markdown records earn: {entry}"
        );
    }

    // The fixture corpus is what shows the mapping can answer other things.
    let fixtures = &view["fixtures"];
    assert_eq!(
        fixtures["casesMatched"], fixtures["casesTotal"],
        "a compatibility fixture did not match: {fixtures}"
    );
    let demonstrated: BTreeSet<&str> = fixtures["statesDemonstrated"]
        .as_array()
        .expect("states")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        demonstrated.contains("incompatible")
            && demonstrated.contains("unreadable")
            && demonstrated.contains("lossy"),
        "the corpus did not separate incompatible, unreadable and a readable control: \
         {demonstrated:?}"
    );
}

/// TC-012: no local generic assurance machinery remains in the execution path.
/// Trace: TC-012, FR-006-AC-6
#[test]
fn no_local_generic_machinery_remains() {
    // The retained records stay; the *verifier* is what was removed. `make ci`
    // must no longer run a repository-local evidence verifier, and no script may
    // reference one.
    assert!(
        !MAKEFILE.contains("verify-evidence:"),
        "a repository-local evidence verifier target is still defined"
    );
    assert!(
        root().join("evidence/manifest.sha256").is_file(),
        "the retained manifest was deleted; it is frozen, not removed"
    );

    let scripts = root().join("scripts");
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&scripts).expect("scripts directory") {
        let path = entry.expect("script entry").path();
        if path.is_file() {
            names.insert(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    // A census, not a spot check: a new generic collector, envelope builder or
    // verifier appearing here is the thing this migration exists to prevent.
    for forbidden in [
        "verify_evidence.py",
        "build_evidence_envelope.py",
        "collect_evidence.sh",
        "finalize_collection.py",
        "tool_identity.py",
        "verify_evidence_manifest.py",
    ] {
        assert!(
            !names.contains(forbidden),
            "a generic evidence script reappeared: {forbidden}"
        );
    }
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
    assert!(
        FR_006.contains("engineering-assurance#21"),
        "FR-006 does not record the PGM-01 mapping gap"
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

    // Every claimed state must be traceable to a case that both ran and matched.
    // A scenario that demonstrated nothing carries null and contributes nothing,
    // rather than borrowing the label it was aiming at.
    let mut backed = BTreeSet::new();
    for case in report["scenarios"]
        .as_array()
        .expect("scenarios")
        .iter()
        .chain(report["controls"].as_array().expect("controls"))
    {
        if case["matched"] == Value::Bool(true) {
            if let Some(state) = case["demonstrates"].as_str() {
                backed.insert(state);
            }
        }
    }
    assert_eq!(
        demonstrated, backed,
        "a state was reported as demonstrated without a matching case behind it"
    );

    for required in ["unavailable", "tampered", "inconclusive"] {
        assert!(
            demonstrated.contains(required),
            "{required} was not demonstrated by any case: {demonstrated:?}"
        );
    }
}
