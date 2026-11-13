---
id: SR-17054
title: "Code review of the FR-022 datatype package translation"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/package_translation.rs, tests/common/mod.rs, tests/datatype_translation.rs, tests/package_runner.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-022", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-009", type: reviews }
---
# Code Review of the FR-022 Datatype Package Translation

## Summary

Ticket: VER-88. PR agent-ix/quire-analyze#96, diff `origin/main...HEAD` (one
commit, four files, +1094/-29). Includes the rust-review lane (rust-style loaded;
repo `AGENTS.md`/`CLAUDE.md`, `rustfmt.toml`, `clippy.toml` applied).

Measured with toolchain 1.98.1: `cargo fmt --check` exit 0 (the coder's post-fmt
state is clean; the nightly-only rustfmt options only warn); `cargo clippy
--all-targets -- -D warnings` exit 0. Focused tests: `datatype_translation` 11
passed, 4 ignored; `package_runner` 31 passed, 2 ignored; `package_translation`
17 passed. With `--include-ignored`, the three record/option tests fail in
`lower_boolean_statements` ("record expressions are unsupported", "option
expressions are unsupported", "only Boolean, integer and enum value references
are exact"), that is in the src/smt.rs lowering VER-86 owns, not in the
translation; the real-solver test fails on the missing `QUIRE_Z3`, as its ignore
reason states. z3/cvc5 are not installed and nothing was installed.

## Verdict

**FAIL (two medium).** The term table of FR-022 is implemented as written:
`enum.eq/ne` and `structural.eq/ne` map to `Compare` over the datatype class,
`record.project` takes one operand and the `field` member to `FieldAccess`,
`option.present` to `IsPresent`, `option.value` to `Unwrap` with no translation
guard check, enum and `none` literals to `EnumLiteral`/`OptionNone`, and every
other identity exits `2` naming `semantic_graph.nodes[N]`. Declarations come only
from reached `semantic_type` nodes, no variant or field is invented, and type
errors surface either at the node or as the upstream `check_expression` clause
error. The admission quirk is sound in the failure direction: a field name the
operand's record lacks is refused by `check_expression` (exit `2`, `claim-N`),
never translated silently. Runner tests spawn the real binary against a stub
`producer-execute` that records requests, which is the FR-009 test seam and is
legitimate; every non-ignored test can fail. Public-repo hygiene is clean: no
build artifact, committed fixture, SHA, digest, roadmap or compatibility layer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unguarded `quire.op.option.value` tests do not assert the validator's path that FR-022-AC-1 (sixth fixture) and FR-022-AC-3 require: the unit test asserts only that the path does not start with `semantic_graph.nodes[`, and the runner test asserts only `claim-N` on stderr. Pin the exact path `check_expression` reports. | `tests/datatype_translation.rs:272-289`, `tests/package_runner.rs:1614-1634` |
| FND-002 | medium | `projected_field` reads `operation.member.name` from the wire body but checks neither `member.kind == "field"` nor that `member.declaration` is the operand's record type node; `ContractPackageWireV1::read` does not admit catalog members (identity edits pass admission, as `with_identity` shows), so a member of another kind, or naming another record's same-named field, translates to a projection the package did not state. | `src/package_translation.rs:624-635` |
| FND-003 | low | The datatype class conflates enum, record and option, so `quire.op.enum.eq/ne` over two records or two options translates to a structural `Compare` that `check_expression` accepts, and `option.present`/`option.value` over a record or enum exits `2` naming `claim-N` instead of the application node FR-022 Refusals name. The operand's type kind is known at translation and can be checked per identity. | `src/package_translation.rs:1058-1066`, `src/package_translation.rs:1103` |
| FND-004 | low | Spans for enum variants, records and fields are taken from the type node (`self.spans.node(type_node)?`), so a type node with empty `source_map` occurrences (an imported or built-in type) refuses every claim that reaches it. FR-009 states the empty-occurrence rule for claim nodes only; for type nodes this is spec-silent and a zero span would not refuse a valid claim. | `src/package_translation.rs:652`, `src/package_translation.rs:697` |
| FND-005 | low | The satisfiable-enum runner test asserts exit `3`, explanation `incomplete` and disposition inconclusive but not the witness reason `sort_not_replayable` naming the enum variable that FR-020 "Replay until FR-021" and FR-020-AC-10 fix, so a different incompleteness cause would also pass. | `tests/package_runner.rs:1525-1541` |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | `type_span` falls back to the claim span on every `SpanSource::node` error, not only the missing source region FR-022 now names: a type node whose region exists but yields an invalid `SourceLocation` or `SourceSpan` is silently given the claim span instead of exiting `2`. Match on `TranslationErrorKind::SourceRegion` from the missing-region branch only, or split the lookup. | `src/package_translation.rs:624-630` |
| FND-007 | low | The SR files were committed as `reviews/2026-11-13-ver-88-*.md`; the reviewer convention and the repository's recent files (`reviews/ver-94-*.md`) carry no date prefix. Copy the updated files to `reviews/ver-88-code-review.md` and `reviews/ver-88-gap-analysis.md` and remove the dated copies. | `reviews/2026-11-13-ver-88-code-review.md`, `reviews/2026-11-13-ver-88-gap-analysis.md` |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 7e067c9: unit test asserts path `expression` and the message; runner test asserts `claim-N at expression: option presence is not established`. |
| FND-002 | fixed | 7e067c9: `projected_field` refuses a member whose `kind` is not `field` or whose `declaration` is not the operand's type node; matches the QSL FR-093 `field{declaration: the record's node, name}` shape, so no legitimate projection is refused; both mutations are tested and each check alone fails its case. |
| FND-003 | fixed | 7e067c9: `operand_sort` limits `enum.*` to enums, `structural.*` to enum, record or option, `record.project` to records and `option.*` to options, refused at the application node; tested by two identity mutations. |
| FND-004 | fixed | 7e067c9: FR-022 now states the claim-span fallback and `type_span` implements it; tested. See FND-006 for its breadth. |
| FND-005 | fixed | 7e067c9: `a_satisfiable_translated_enum_claim_is_incomplete_with_sort_not_replayable` pins `ReplayFailureReason::SortNotReplayable` and the enum variable through the analysis API the runner uses. |
