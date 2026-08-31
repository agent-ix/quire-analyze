//! Requirement-backed tests for the specification and assurance foundation.

const CARGO_MANIFEST: &str = include_str!("../Cargo.toml");
const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");
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
#[test]
fn foundation_keeps_license_publication_and_ci_authority_bounded() {
    assert!(CARGO_MANIFEST.contains("license = \"MIT OR Apache-2.0\""));
    assert!(CARGO_MANIFEST.contains("publish = false"));
    assert!(CI_WORKFLOW.contains("workflow_dispatch:"));
    assert!(!CI_WORKFLOW.contains("pull_request:"));
    assert!(!CI_WORKFLOW.contains("push:"));
    assert!(!CI_WORKFLOW.contains("schedule:"));
    assert!(ASSURANCE_ARGUMENT.contains("status: open"));
    assert!(ASSURANCE_ARGUMENT.contains("No automated sufficiency decision"));
}

/// Issue #2 specification corpus and failure-domain gate; NFR-002-AC-2.
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

/// Issue #2 workflow gate: after foundation completion, only the first DAG child advances.
#[test]
fn foundation_plan_advances_only_first_unblocked_child() {
    for issue in ["#6", "#7", "#3", "#4", "#5"] {
        assert!(PLAN.contains(issue));
    }

    for row in [
        "| Task-004 | #7 deterministic SMT lowering | not_started |",
        "| Task-005 | #3 bounded solver adapters | not_started |",
        "| Task-006 | #4 analyses/counterexamples | not_started |",
        "| Task-007 | #5 evidence/differential/CLI | not_started |",
    ] {
        assert!(PLAN.contains(row), "missing guarded child row {row}");
    }
    assert!(PLAN.contains("| Task-003 | #6 ADR-0010 algebra/identity | in_progress |"));
    assert!(PLAN.contains("All implementation children remain Backlog"));
    assert!(TEST_MATRIX.contains("The placeholder"));
    assert!(TEST_MATRIX.contains("crate tests count only as scaffold health and satisfy no row"));
    for semantic_test in [
        "TC-001", "TC-002", "TC-003", "TC-004", "TC-005", "TC-006", "TC-007", "TC-008",
    ] {
        let row = TEST_MATRIX
            .lines()
            .find(|line| line.starts_with(&format!("| {semantic_test} |")))
            .expect("semantic test row must exist");
        assert!(row.ends_with("🚧 Planned |"));
    }
    assert!(TEST_MATRIX.contains("| TC-009 | ADR-0010 identity research |"));
}
