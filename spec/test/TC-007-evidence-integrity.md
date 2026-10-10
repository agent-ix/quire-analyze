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

For the planned `quire.differential-report/v2` lane, validate a genuine v2 document and mutate
each required producer identity and canonical clause, request, query, raw-stream, model, and report
digest. Add `contractIrRevision` to a v2 document and pass a valid v1 document to the v2 validator;
both must be rejected. This lane is not executable until the v2 renderer, schema, and validator land.

## Expected Results

The valid corpus passes. Every omission, mutation, contradiction, or census change fails closed with
a stable diagnostic. Optional explanation evidence is never inferred as present.
The planned v2 lane accepts only complete v2 evidence with verified content digests; it rejects the
duplicate revision field, omitted identities or digests, and v1 documents without a fallback.
