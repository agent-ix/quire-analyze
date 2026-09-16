---
id: TC-013
title: "Verify sound bounded hybrid reachability"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-007
    type: verifies
---
# TC-013: Verify sound bounded hybrid reachability

## Description

Verify the finite hybrid provider produces conservative enclosures rather than point estimates,
retains an exact replay witness, and preserves typed non-conclusions at malformed and exhausted bounds.

## Test Procedure

Run `tests/hybrid_synthesis.rs::tc_013_preserves_mode_guard_reset_and_exact_replay` and
`tests/hybrid_synthesis.rs::tc_013_refuses_malformed_models_and_keeps_bound_exhaustion_nonconclusive`.
Exercise a guarded transition and reset, replay its retained witness, mutate the bound model, exhaust
the admitted step budget, and submit an invalid guard interval.

## Expected Results

The returned enclosure carries its declared error bound and reaches the target mode only through the
guard/reset transition. The witness replays only against the exact model. An exhausted bound is
`Incomplete` and malformed model input is `Refused`; neither outcome is a proof.

## Limitations

This case verifies the finite provider's explicit flow/guard/reset model and declared bounds. It does
not claim completeness outside those submitted bounds or discharge a human release decision.
