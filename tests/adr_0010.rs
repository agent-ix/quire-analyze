//! TC-009: executable measurements for ADR-0010 shared-variable identity.

use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("../research/adr-0010-shared-variable-fixture.tsv");
const ADR: &str = include_str!("../spec/architecture/ADR-0010-analysis-algebra.md");
const REPORT: &str = include_str!("../planning/adr-0010-research.md");

#[derive(Debug)]
struct Row<'a> {
    id: &'a str,
    package: &'a str,
    requirement: &'a str,
    revision: &'a str,
    kind: &'a str,
    observation: &'a str,
    name: &'a str,
    value_type: &'a str,
    anchor: &'a str,
    truth_binding: &'a str,
}

impl<'a> Row<'a> {
    fn parse(line: &'a str) -> Self {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 10, "invalid TC-009 fixture row: {line}");
        Self {
            id: fields[0],
            package: fields[1],
            requirement: fields[2],
            revision: fields[3],
            kind: fields[4],
            observation: fields[5],
            name: fields[6],
            value_type: fields[7],
            anchor: fields[8],
            truth_binding: fields[9],
        }
    }

    fn complete_ir_identity(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.package,
            self.requirement,
            self.revision,
            self.kind,
            self.observation,
            self.name,
            self.value_type,
            self.anchor
        )
    }

    fn typed_name_identity(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.package, self.kind, self.observation, self.name, self.value_type, self.anchor
        )
    }

    fn compatible_with(&self, other: &Self) -> bool {
        self.package == other.package
            && self.kind == other.kind
            && self.observation == other.observation
            && self.value_type == other.value_type
            && self.anchor == other.anchor
    }
}

fn rows() -> Vec<Row<'static>> {
    FIXTURE.lines().skip(1).map(Row::parse).collect()
}

fn measure(rows: &[Row<'_>], candidate: impl Fn(&Row<'_>) -> String) -> (usize, usize, usize) {
    let mut true_positive = 0;
    let mut false_positive = 0;
    let mut false_negative = 0;

    for left_index in 0..rows.len() {
        for right_index in (left_index + 1)..rows.len() {
            let left = &rows[left_index];
            let right = &rows[right_index];
            let predicted = candidate(left) == candidate(right);
            let expected = left.truth_binding == right.truth_binding;
            match (predicted, expected) {
                (true, true) => true_positive += 1,
                (true, false) => false_positive += 1,
                (false, true) => false_negative += 1,
                (false, false) => {}
            }
        }
    }

    (true_positive, false_positive, false_negative)
}

/// FR-001-AC-2 and TC-009: quantify the cost of implicit alias rules.
#[test]
fn candidate_identity_rules_match_the_retained_measurement() {
    let rows = rows();
    assert_eq!(rows.len(), 11);

    assert_eq!(
        measure(&rows, |row| row.complete_ir_identity()),
        (0, 0, 2),
        "complete IR identity is safe but misses explicit cross-requirement sharing"
    );
    assert_eq!(
        measure(&rows, |row| row.name.to_owned()),
        (2, 12, 0),
        "name-only matching aliases unrelated scopes and types"
    );
    assert_eq!(
        measure(&rows, |row| row.typed_name_identity()),
        (2, 3, 0),
        "adding structural fields still cannot infer domain meaning"
    );
    assert_eq!(
        measure(&rows, |row| row.truth_binding.to_owned()),
        (2, 0, 0),
        "explicit reviewed bindings recover sharing without accidental aliases"
    );
}

/// FR-001-AC-2 and TC-009: explicit binding groups fail closed on structural mismatch.
#[test]
fn explicit_bindings_reject_incompatible_members() {
    let rows = rows();
    let by_id: BTreeMap<_, _> = rows.iter().map(|row| (row.id, row)).collect();

    assert!(by_id["A"].compatible_with(by_id["B"]));
    assert!(by_id["F"].compatible_with(by_id["G"]));
    assert!(!by_id["A"].compatible_with(by_id["D"]));
    assert!(!by_id["A"].compatible_with(by_id["E"]));
    assert!(!by_id["F"].compatible_with(by_id["H"]));
}

/// FR-001-AC-3/5 and TC-009: the binding decision and invalidation rules are normative.
#[test]
fn adr_binds_versions_hashes_and_supersession() {
    for required in [
        "quire.analysis-model/v1",
        "quire.smtlib2/v1",
        "explicit binding group",
        "clause canonical digest",
        "binding-set digest",
        "encoding-profile identity",
        "type-shape digest",
        "exact execution-point equality",
        "agent-ix/quire-rs#164",
    ] {
        assert!(ADR.contains(required), "ADR omits {required}");
    }

    assert!(REPORT.contains("(0, 0, 2)"));
    assert!(REPORT.contains("(2, 12, 0)"));
    assert!(REPORT.contains("(2, 3, 0)"));
    assert!(REPORT.contains("(2, 0, 0)"));
    assert_eq!(REPORT.matches("(<= ").count(), 3);
}
