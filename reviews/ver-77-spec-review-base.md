---
id: SR-17031
title: "Base spec review of VER-77 analysis_claim translation (FR-009, TC-015)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; quire-analyze#82 diff origin/main...HEAD: spec/functional/FR-009-package-runner.md, spec/test/TC-015-package-runner.md"
review_set: subset
---

## Summary

Ticket: VER-77. Base checklist review of the spec-only diff of quire-analyze#82. Each
translation decision was checked against `src/smt.rs` and against `quire-contract-model`
at the revision in `Cargo.lock`, and against current `quire-contract-ir` main.
`quire validate --scope . "spec/**/*.md"` exits 0. The diff adds no hash, digest, pin or
vendored content. FR-009-AC-27 forbids committed contract-package documents.

These decisions are grounded in the code:
- the synthetic anchor `Handler` `analysis` with `ClauseKind::Assertion` (`Clause::new` refuses
  an executable clause with no anchor, `Assertion` accepts every point, and `from_clause`
  refuses a clause with no anchor);
- the single shared requirement `analysis` (`variable_key` keys on the owner `RequirementRef`
  and the execution point);
- the `ClauseId` `claim-N`, `RequirementId` `analysis` and `SourceDocumentId`
  `contract-package`, which all fit the identifier grammar;
- `PackageId::new`, and `SchemaVersion::V1_1`, which `canonical_clause` requires;
- `Input` declarations with `Current` observation (`check_value_reference`), and the `nN`
  names, which fit `SymbolName`;
- the six `quire.op.boolean.*` rows, which are binary or unary in
  `checked-operation-catalog-v1.json` and match `CAPABILITY_CONTRACT.exact_constructs`;
- `Compare` `Equal`/`NotEqual` over Booleans, which `check_compare` accepts;
- the line 1, column 1 span rule (`SourceLocation::new` and `SourceSpan::new`).

The `CatalogUnavailable` question has an answer. At the revision in the lock,
`ContractPackageReadResult` has three variants (`Admitted`, `Refused`, `Incomplete`).
`CatalogUnavailable(CatalogReadError)` exists on `quire-contract-ir` main, which
`Cargo.toml` tracks with `branch = "main"`. The Input section is right about the dependency.
The lock is stale (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Input section lists a `CatalogUnavailable` reader result, but the `quire-contract-model` revision in `Cargo.lock` has only `Admitted`, `Refused` and `Incomplete`. The variant was added later on quire-contract-ir main, which Cargo.toml tracks. The new sentence "The rules below were measured against `quire-contract-model` at the revision locked in `Cargo.lock`" is a provenance statement, and it now points at a revision the spec already disagrees with. Fix: keep the four-variant Input text, which is correct against main. Delete the "measured against ... locked in Cargo.lock" sentence. Add a gap saying the implementing change updates the `quire-contract-model` lock to current main, because the locked revision has no `CatalogUnavailable`. The public translation APIs the section uses (`CheckedTerm`, `CheckedApplicationOperation::read`, `ExecutionPoint`, `Clause::new`, `SourceLocation`, `check_value_reference`) are unchanged on main. | spec/functional/FR-009-package-runner.md:32-35, spec/functional/FR-009-package-runner.md:55-56 |
| FND-002 | medium | The Source spans refusal and the second sentence of FR-009-AC-24 cannot be reached. The reader requires `source_map.nodes` to have exactly one entry per graph node, in graph order, with the same `node_id`. It refuses an occurrence with empty `regions` (contract_package.rs validate_occurrences). A package with no entry for a claim is refused by the reader, so the run exits 2 for the reader's reason and does not name the node. The only case the translation can see is an entry with an empty `occurrences` list. Fix: define the span source as `source_map.nodes[N].occurrences[0].regions[0]`. Reduce the refusal to an empty `occurrences`. Reword AC-24 to that case. | spec/functional/FR-009-package-runner.md:81-92, FR-009-AC-24 |
| FND-003 | medium | Gap 5 says the tests of AC-2, AC-3, AC-14 and AC-19 to AC-26 cannot be written until quire-contract-ir adds a fixture builder. The public APIs may already cover this path: a `CheckedPackageV2` built in test code, then `encode_checked_package_v2`, the V2 reader, `CheckedPackageV2::lower`, and the canonical bytes of the `CompleteContractPackageV2`. TC-015 already uses that path for the QSL package. The spec states a blocker that was not measured. Fix: try the public path. If it works, delete the blocker and specify the path. If it does not, name the exact missing public API. | spec/functional/FR-009-package-runner.md:335-341 |
| FND-004 | low | The Dependencies paragraph names `ContractPackageV1::read`. The reader is `ContractPackageWireV1::read`, which returns a `ContractPackageV1`, and the Input section names it correctly. Fix: rename it in the Dependencies paragraph. | spec/functional/FR-009-package-runner.md:309 |
| FND-005 | low | TC-015 case 19 runs AC-23 and AC-25 as a unit test of the translation function. Those ACs require exit `2` and no report, which only a run of the binary can show. Fix: restate AC-23 and AC-25 as "the translation returns a refusal naming ...", with AC-19 covering the binary exit. Or run those cases through the binary in TC-015. | spec/test/TC-015-package-runner.md:58-62, FR-009-AC-23, FR-009-AC-25 |

## Verdict

PASS WITH FINDINGS: no high-severity defect in the base checklist. The translation
decisions are grounded in the code as listed in the Summary. AC numbering (20-27) is
contiguous, and AC-16, AC-17 and AC-14 now agree with the result-state table and with
AC-27. AC-20, AC-21, AC-22 and AC-26 can fail and can be tested against `QueryBundle`. Fix
FND-001 to FND-003 before the implementing change starts.
