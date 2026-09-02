---
id: TC-010
title: "Verify the exact Boolean SMT-LIB2 v1 lowering slice"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-001
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-002
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-001
    type: verifies
---
# TC-010: Verify the exact Boolean SMT-LIB2 v1 lowering slice

## Description

Verify the production boundary delivered by native issue #7 without implying that solver adapters,
analysis-kind conclusions, evidence publication, or the CLI exist.

## Test Procedure

Run `tests/smt_lowering.rs` under the default toolchain and Rust 1.75. Permute statement and named-type
declaration order; exercise every exact Boolean operator; mutate clause digests; validate accepted,
duplicate, and unused binding groups; exceed the statement limit; lower the retained golden query;
and inject arithmetic, quantification, and text data. Inspect the exhaustive `ExpressionKind` match,
capability table, exact contract-IR revision, dependency lock, and absence of solver libraries.

## Expected Results

Equivalent supported inputs yield byte-identical query and map bytes. Every assertion and variable
retains its complete source identity. Material identity changes invalidate request and query digests.
Malformed bindings and bounded resources fail before query completion. Unsupported constructs return
stable `UnsupportedConstruct` diagnostics, and the committed golden is byte-identical to production
output. Both default and MSRV builds pass without warnings.

## Limitations

TC-010 completes the issue #7 Boolean lowering slice only. TC-001 through TC-008 retain the broader
analysis, adapter, collision-family, failure-injection, differential, evidence, and CLI campaigns.
