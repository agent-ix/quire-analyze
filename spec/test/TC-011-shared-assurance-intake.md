---
id: TC-011
title: "Verify the shared assurance intake path and result derivation"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-006
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-002
    type: verifies
---
# TC-011: Verify the shared assurance intake path and result derivation

## Description

Verify that this repository's own tools produce the declared structured results,
that Quoin transcribes them without executing a producer, and that every result
an attestation states was read out of the bytes a producer wrote.

## Test Procedure

Run `make assurance-inputs` once, then drive the chain over its output.

Replace `cargo`, `rustup`, `rustc` and `quire` on `PATH` with executable shims
that answer only `--version` and log every other invocation. Require the chain to
finish and the log to be empty. Then replace `quoin` with the same shim and
require the chain to fail with a non-empty log, so that an empty log in the first
run cannot be explained by `PATH` never having been consulted.

Compare each attestation's result against the `outcome` field of the producer
document it describes. Rewrite every producer's outcome to `failed` and require
the chain to exit non-zero. Replace a producer document with bytes that are not
JSON and require exit 2. Declare an outcome the adapter's table does not name and
require exit 2.

Exercise the solver-state census: every provoked condition must produce the
outcome it was built to provoke, `sat` and `unsat` must be the only conclusive
outcomes, and all four differential dispositions must be reachable. Validate the
authoritative differential report bytes, then require each tampered semantic
field, tampered retained stream, stale report digest and truncated document to be
refused.

Seal a record body that states its own digest, intake a sealed attestation whose
retained bytes were altered, and intake an attestation edited after sealing.

Request a receipt with no ix-flow decision event.

## Expected Results

The chain completes with no producer invocation logged, and the control run fails.
Every attestation result equals its producer's declared outcome. A producer that
reports failure makes the chain fail; a producer whose output cannot be read, and
an outcome the table does not name, both exit 2 rather than defaulting to a pass.

`sat` and `unsat` are the only conclusive outcomes and no two provoked conditions
share one. The authoritative report is accepted and every tamper is refused.
Quoin refuses the presupplied digest, the altered retained bytes and the edited
attestation.

The receipt reads `incomplete` with a missing decision — not valid and not
invalid — and an engine that was not installed reaches the receipt as
`result_unavailable`, never as passed and never as failed.
