---
id: REV-005
title: "ADR-0010 code and specification review"
type: Review
---
# ADR-0010 code and specification review

## Scope

Review issue #6 changes to ADR-0010, FR-001, TC-009, the retained fixture, the reproduced research
report, matrix, and plan. This is producer review, not independent approval or a release decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| A10-F01 | high | “Value-type canonical digest” was not a defined contract-IR artifact; declaration canonicalization includes its requirement owner and therefore cannot establish cross-requirement structural compatibility. | Closed: ADR-0010 defines a source-free, owner-free, fully resolved analysis type-shape digest while retaining named-type names and complete structure. Issue #7 owns implementation. |
| A10-F02 | high | “Compatible execution points” was undefined and could silently alias different temporal frames. | Closed: v0.1 requires exact execution-point equality and reports cross-point binding unsupported. |
| A10-F03 | high | The first direct SMT-LIB example declared bounded integers but constrained only their lower bounds, so it was not equivalent to the algebra example. | Closed: all three declarations now assert both zero and `i64::MAX` bounds; TC-009 checks three upper-bound assertions. |
| A10-F04 | medium | Binding arbitrary `DependencyKind` values could make fields, enum variants, or pure functions independent variables rather than derived expression dependencies. | Closed: only root input/state dependencies are bindable; other dependency kinds remain derived. |
| A10-F05 | medium | The complete-identity measurement included a research row ID not present in the real IR identity. | Closed: the candidate key now uses only package, requirement/revision, kind, observation, name, type, and execution point; results are unchanged. |
| A10-F06 | medium | A 100% explicit-binding row could be mistaken for measured inference accuracy. | Closed: ADR and report state it measures deterministic application of human-reviewed labels, not an inference algorithm. |
| A10-F07 | medium | `@current` in the real-FR transcription was not an immutable source identity. | Closed: the report records exact historical source revision and file digest and labels the owner as a research transcription. |
| A10-F08 | low | ADR relationships used an edge not allowed by the active Quire ADR archetype. | Closed: unsupported ADR edges were removed; FR-001 retains the normative ADR relationship. |

## Code Quality

The TC-009 test uses no new dependency, parses a committed ten-field fixture, exhaustively evaluates
all 55 unordered pairs, and separately probes incompatible kind, type, and observation/point cases.
Its fixed expected counts make changes to fixture labels or candidate keys visible. Rust 1.75,
Clippy warnings-as-errors, formatting, documentation, license, and unsafe-code gates pass.

The test's ADR/report string assertions are structural guards, not a proof that the architecture is
sound. The pair measurement and explicit negative compatibility checks are the executable evidence;
Quire supplies document structure validation, and human review supplies architectural judgment.

## Verdict

PASS. No unresolved blocking issue #6 finding remains. Production identity encoding, type-shape
projection, and lowering remain deliberately unimplemented and owned by issue #7.
