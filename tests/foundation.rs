//! Requirement-backed tests for the specification and assurance foundation.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use sha2::{Digest as _, Sha256};

const CARGO_MANIFEST: &str = include_str!("../Cargo.toml");
const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");
const MAKEFILE: &str = include_str!("../Makefile");
const EVIDENCE_MANIFEST: &str = include_str!("../evidence/manifest.sha256");
const MASTER_SPEC: &str = include_str!("../spec/index.md");
const FUNCTIONAL_REQUIREMENTS: [&str; 5] = [
    include_str!("../spec/functional/FR-001-analysis-algebra.md"),
    include_str!("../spec/functional/FR-002-smt-lowering.md"),
    include_str!("../spec/functional/FR-003-bounded-adapters.md"),
    include_str!("../spec/functional/FR-004-conclusions.md"),
    include_str!("../spec/functional/FR-005-evidence-cli.md"),
];
const INTERFACE: &str = include_str!("../spec/interface/interface-001-analysis-api.md");
const TEST_MATRIX: &str = include_str!("../spec/test-matrix.md");
const ASSURANCE_PROFILE: &str = include_str!("../spec/assurance/AP-001-analyze-release.md");
const ARCHITECTURE: &str = include_str!("../spec/assurance/AD-001-analyze-architecture.md");
const COMPONENT_CONTRACT: &str = include_str!("../spec/assurance/CAC-001-analyze-controls.md");
const MEASUREMENT_PLAN: &str = include_str!("../spec/assurance/MP-001-analyze-measurements.md");
const ASSURANCE_ARGUMENT: &str = include_str!("../spec/assurance/AA-001-analyze-argument.md");
const PLAN: &str = include_str!("../plan/PLAN-001-analyze-v01/plan.md");

/// Issue #2 baseline and policy gate; NFR-002-AC-4.
/// Trace: NFR-002-AC-4
#[test]
fn foundation_keeps_license_publication_and_ci_authority_bounded() {
    assert!(CARGO_MANIFEST.contains("license = \"MIT OR Apache-2.0\""));
    assert!(CARGO_MANIFEST.contains("publish = false"));
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(CI_WORKFLOW).expect("valid workflow YAML");
    let triggers = workflow
        .get("on")
        .and_then(serde_yaml::Value::as_mapping)
        .expect("workflow on mapping");
    let trigger_names: BTreeSet<_> = triggers
        .keys()
        .map(|key| key.as_str().expect("string trigger"))
        .collect();
    assert_eq!(trigger_names, BTreeSet::from(["workflow_dispatch"]));
    assert!(ASSURANCE_ARGUMENT.contains("status: open"));
    assert!(ASSURANCE_ARGUMENT.contains("No automated sufficiency decision"));
}

/// Issue #2 specification corpus and failure-domain gate; NFR-002-AC-2.
/// Trace: NFR-002-AC-2
#[test]
fn foundation_defines_closed_requirements_and_non_conclusive_states() {
    assert!(MASTER_SPEC.contains("FR-001 through FR-005"));
    for (requirement, artifact) in ["FR-001", "FR-002", "FR-003", "FR-004", "FR-005"]
        .into_iter()
        .zip(FUNCTIONAL_REQUIREMENTS)
    {
        assert!(artifact.contains(&format!("id: {requirement}")));
        assert!(TEST_MATRIX.contains(requirement));
    }

    for outcome in [
        "unknown",
        "unsupported",
        "timed-out",
        "cancelled",
        "tool-unavailable",
        "tool-error",
        "invalid-input",
        "internal-error",
    ] {
        assert!(INTERFACE.contains(outcome), "missing outcome {outcome}");
    }

    assert!(INTERFACE.contains("conclusive: [satisfied, refuted]"));
    assert!(INTERFACE.contains("only a complete recognized sat/unsat response"));
    for analysis in [
        "consistency",
        "contradiction",
        "implication",
        "redundancy",
        "dead-antecedent",
    ] {
        assert!(INTERFACE.contains(analysis), "missing analysis {analysis}");
    }
}

/// Issue #2 assurance-artifact completeness gate.
#[test]
fn foundation_names_assurance_boundary_evidence_and_owner() {
    for artifact in [
        ASSURANCE_PROFILE,
        ARCHITECTURE,
        COMPONENT_CONTRACT,
        MEASUREMENT_PLAN,
        ASSURANCE_ARGUMENT,
    ] {
        assert!(artifact.contains("PGM-01") || artifact.contains("release"));
    }

    assert!(ASSURANCE_PROFILE.contains("Intended Use"));
    assert!(ARCHITECTURE.contains("System Boundary"));
    assert!(COMPONENT_CONTRACT.contains("Failure Handling"));
    assert!(MEASUREMENT_PLAN.contains("Retention"));
    assert!(ASSURANCE_ARGUMENT.contains("human-release-owner"));
}

/// PLAN-001 workflow gate: only the first unfinished DAG child advances.
/// Trace: PLAN-001
#[test]
fn foundation_plan_advances_only_first_unblocked_child() {
    for issue in ["#6", "#7", "#3", "#4", "#5"] {
        assert!(PLAN.contains(issue));
    }

    assert!(PLAN.contains("| Task-004 | #7 deterministic SMT lowering | done |"));
    for row in [
        "| Task-006 | #4 analyses/counterexamples | not_started |",
        "| Task-007 | #5 evidence/differential/CLI | not_started |",
    ] {
        assert!(PLAN.contains(row), "missing guarded child row {row}");
    }
    assert!(PLAN.contains("| Task-005 | #3 bounded solver adapters | in_progress |"));
    assert!(PLAN.contains("| Task-003 | #6 ADR-0010 algebra/identity | done |"));
    assert!(PLAN.contains("All implementation children remain Backlog"));
    assert!(TEST_MATRIX.contains("The placeholder"));
    assert!(TEST_MATRIX.contains("satisfy no row"));
    for semantic_test in [
        "TC-001", "TC-002", "TC-003", "TC-004", "TC-006", "TC-007", "TC-008",
    ] {
        let row = TEST_MATRIX
            .lines()
            .find(|line| line.starts_with(&format!("| {semantic_test} |")))
            .expect("semantic test row must exist");
        assert!(row.ends_with("🚧 Planned |"));
    }
    assert!(TEST_MATRIX.contains("| TC-005 | Adapter resource and failure isolation |"));
    assert!(TEST_MATRIX.contains("| ✅ Linux adapter v1 complete |"));
    assert!(TEST_MATRIX.contains("| TC-009 | ADR-0010 identity research |"));

    let complete_rows: Vec<_> = TEST_MATRIX
        .lines()
        .filter(|line| line.starts_with('|') && line.contains("| ✅"))
        .collect();
    assert_eq!(
        complete_rows.len(),
        14,
        "a new complete matrix row requires an executable trace binding"
    );
    assert_eq!(
        complete_rows
            .iter()
            .filter(|line| line.starts_with("| FR-002 |"))
            .count(),
        4
    );
    assert_eq!(
        complete_rows
            .iter()
            .filter(|line| line.starts_with("| FR-003 |"))
            .count(),
        5
    );
    assert!(complete_rows
        .iter()
        .any(|line| line.starts_with("| TC-009 |")));
    assert!(complete_rows
        .iter()
        .any(|line| line.starts_with("| TC-010 |")));
    assert_eq!(TEST_MATRIX.matches("Coverage Status |").count(), 2);
}

/// Local CI policy: command failures and tool-variable attacks cannot be silently ignored.
#[test]
fn make_ci_has_a_closed_unsuppressed_gate_census() {
    assert!(MAKEFILE.contains("override CARGO := cargo"));
    assert!(MAKEFILE.contains("local CI refuses a CARGO override"));
    assert!(MAKEFILE.contains("local CI refuses non-empty MAKEFLAGS"));
    assert!(!MAKEFILE.contains("CARGO ?="));
    assert!(!MAKEFILE.lines().any(|line| {
        line.trim_start().starts_with(".IGNORE") || line.trim_start().starts_with(".SILENT")
    }));

    let ci = MAKEFILE
        .lines()
        .find(|line| line.starts_with("ci:"))
        .expect("ci target");
    let actual: BTreeSet<_> = ci.trim_start_matches("ci:").split_whitespace().collect();
    let expected = BTreeSet::from([
        "audit-unsafe",
        "coverage",
        "deny",
        "fmt-check",
        "lint",
        "msrv",
        "rustdoc",
        "spec",
        "test",
        "verify-evidence",
    ]);
    assert_eq!(actual, expected);

    for (line_number, line) in MAKEFILE.lines().enumerate() {
        if let Some(recipe) = line.strip_prefix('\t') {
            let recipe = recipe.trim_start_matches(['@', ' ']);
            assert!(
                !recipe.starts_with('-'),
                "line {} ignores failure",
                line_number + 1
            );
            for control in ["&&", "||", ";", "|"] {
                assert!(
                    !recipe.contains(control),
                    "line {} uses shell control {control}",
                    line_number + 1
                );
            }
        }
    }
}

/// Retained legacy records have set-equal SHA-256 coverage and no evidence authority.
#[test]
fn retained_evidence_is_censused_and_cannot_claim_machine_verification() {
    fn visit(directory: &Path, files: &mut BTreeSet<PathBuf>) {
        for entry in fs::read_dir(directory).expect("read evidence directory") {
            let entry = entry.expect("evidence entry");
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("evidence metadata");
            assert!(
                !metadata.file_type().is_symlink(),
                "evidence symlink: {path:?}"
            );
            if metadata.is_dir() {
                visit(&path, files);
            } else {
                assert!(metadata.is_file(), "unexpected evidence object: {path:?}");
                files.insert(path);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let evidence_root = root.join("evidence");
    let manifest_path = evidence_root.join("manifest.sha256");
    let mut observed = BTreeSet::new();
    visit(&evidence_root, &mut observed);
    observed.remove(&manifest_path);

    let mut declared = BTreeSet::new();
    for line in EVIDENCE_MANIFEST.lines() {
        let (expected, relative) = line.split_once("  ").expect("checksum manifest row");
        assert_eq!(expected.len(), 64);
        assert!(expected.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let path = root.join(relative);
        assert!(declared.insert(path.clone()), "duplicate manifest path");
        let bytes = fs::read(&path).expect("manifest artifact");
        let actual = format!("{:x}", Sha256::digest(bytes));
        assert_eq!(actual, expected, "changed evidence artifact {relative}");
        let text =
            String::from_utf8(fs::read(path).expect("text evidence")).expect("UTF-8 evidence");
        if relative.contains("smt-lowering-0da1747") {
            assert!(text.contains("Producer validation attestation"));
            assert!(text.contains("no raw command transcript"));
            assert!(text.contains("independent review"));
        } else {
            assert!(text.contains("Legacy narrative only"));
            assert!(text.contains("discharges no\n> acceptance criterion"));
        }
    }
    assert_eq!(
        observed, declared,
        "evidence file census differs from manifest"
    );
}
