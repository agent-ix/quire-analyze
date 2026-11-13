---
id: TC-018
title: "Verify integer analysis_claim translation in the package runner"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-012
    type: verifies
---
# TC-018: Verify integer analysis_claim translation in the package runner

## Description

Verify that `run-package` translates integer `analysis_claim` terms and still refuses non-linear ones.

## Test Procedure

Build contract-package fixtures at test time through the `quire-contract-*` APIs, as TC-015 does, with
integer parameters and linear claims, contradictory claims, a product of parameters, a division, a
reference cycle, and size-bound boundary trees. Spawn the built binary with a stub `producer-execute`
returning controlled results, and assert exit statuses, stderr naming, and report validity. Translate
the same bytes twice and compare. Where solvers and the real `producer-execute` exist, repeat the
satisfiable and contradictory fixtures against them.

## Expected Results

Linear integer claims run end to end and publish a valid v2 report; non-linear claims exit `2` naming
the node; translation is deterministic.

## Limitations

Real-solver cases are skipped visibly where the solvers are absent.
