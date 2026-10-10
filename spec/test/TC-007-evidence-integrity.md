---
id: TC-007
title: "Verify evidence schemas, identities, and truthfulness"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-002
    type: verifies
---
# TC-007: Verify evidence schemas, identities, and truthfulness

## Description

Verify retained evidence detects missing, altered, extra, and contradicted records.

## Test Procedure

Validate every report and envelope against their schemas. Remove and mutate each required identity,
artifact, digest, status, limitation, engine field, and output. Contradict raw responses and declared
outcomes, and alter the artifact census.

Runtime report validation re-derives canonical JSON, report digest, raw stdout/stderr/model digests,
query/configuration identities, and differential disposition. The PGM-01 envelope lane must return
unavailable until the component selected by `quire-contract-ir#20` is adopted.

For the planned `quire.differential-report/v2` lane, validate a genuine v2 document both standalone
and against its authoritative query and differential run. Omit each required producer field,
`assertions[].clause`, `assertions[].source`, and every named digest or retained-byte field; standalone
validation must reject each omission. Mutate retained query, stdout, stderr, and model bytes or their
digests, and the report digest; standalone validation must reject each mismatch. Separately mutate
the clause/source identities and clause, binding-set, or request digests, then recompute dependent
query and report digests so the document is internally consistent; validation against the unchanged
authoritative query and run must still reject it. Add `contractIrRevision` to a v2 document and pass
a valid v1 document to each v2 validation path; both paths must reject them. This lane is not
executable until the v2 renderer, schema, and validator land.

## Expected Results

The valid corpus passes. Every omission, mutation, contradiction, or census change fails closed with
a stable diagnostic. Optional explanation evidence is never inferred as present.
The planned v2 standalone lane checks identities and digests derivable from retained bytes; the
authoritative-input lane rejects substituted clause/source identities and clause, binding-set, or
request digests even when the document's own digests were recomputed. Both reject the duplicate
revision field, omitted required fields, and v1 documents without a fallback.
