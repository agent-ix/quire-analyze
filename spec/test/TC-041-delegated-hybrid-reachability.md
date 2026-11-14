---
id: TC-041
title: Verify delegated hybrid reachability
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-042
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-001
    type: verifies
---
# TC-041: Verify delegated hybrid reachability

## Description

Verify that every complete enclosure returned by the selected PHAVer path covers the reachable integer states in its fit case, that expected resource exhaustion returns the required typed non-conclusion, that the tool is invoked only through producer-execute, and that invalid tool results fail closed.

## Test Procedure

1. Build the finite-execution oracle independently of PHAVer. At step zero, the oracle contains every integer in the initial interval. At each later step, it adds every integer produced by every allowed flow delta, keeps the flowed state in its source mode under non-forced transition semantics, and adds the reset image of each flowed state that satisfies a closed guard. Retain reachable states for every mode at every step through the declared horizon.

2. Encode each finite model in the VER-94 adversarial set as a linear hybrid automaton with one clock advancing one time unit per step, interval flow, closed interval guards, additive resets, and non-forced transitions. Run every recorded F1, F2, F3, F4, F7, F8, F9 and F10 case and each passing bounded case through the selected PHAVer/SX scenario using producer-execute. The set includes flow underflow and overflow by one unit beyond the representable interval; reset overflow; lower and upper widening beyond the representable interval; an overlapping guard that must retain source states; a result that must include intermediate-horizon states; the 1,000,000-step horizon case; and the F10 cases with 100,000 transitions to one target and 100,000 distinct target modes. Passing cases include a guard touched at one point, a partial-guard reset, multiple transitions joined into one target, duplicate and reordered transitions, exact bounded-state enumeration, valid error-bound widening, malformed inputs, an unreachable target, and bounded small models.

3. For every run returning a complete result, parse the quire-analyze.hybrid-tool-result/v1 object. Require each requested mode to appear exactly once or be explicitly empty, reject unknown object members, and require every closed rational interval to have a lower bound no greater than its upper bound. Convert each interval to its integer hull by rounding the lower endpoint up and the upper endpoint down; an empty integer hull denotes an empty enclosure.

4. For every fit case that returns a complete enclosure, compare every mode and every step with the independent oracle and fail if any reachable state is omitted or if the output claims a horizon different from the request. Each F10 request with 100,000 transitions is expected to reach the [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md) 5,000 ms tool limit and return Incomplete with reason tool_timeout and no enclosure. This expected F10 timeout is the AC4 resource boundary, not a fit failure; keep both F10 requests unchanged and do not split, simplify, or route around them. For every other fit case, failure to produce a complete valid enclosure within the declared limits fails the fit check. Report any fit failure on VER-95 and stop the disposition; do not substitute the in-crate algorithm or omit a case.

5. With a stub executor, verify one producer-execute request per hybrid call and the exact [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md) input, output and time limits. Inject a successful result, a timeout, an output-limit breach, a non-zero exit, a missing tool, an unknown schema member, a missing requested mode, malformed rational bounds, and an inverted interval. Verify timeout maps to Incomplete with reason tool_timeout, output-limit breach and non-zero exit map to Incomplete, missing tool maps to Refused with reason tool_unavailable, schema-invalid or inverted output maps to Refused, and no failed result contains an enclosure, as required by [FR-042](../functional/FR-042-hybrid-reachability-disposition.md)-AC-4.

6. Run the canonical-synthesis cases in tests/hybrid_synthesis.rs independently to ensure deletion of the reachability provider does not alter the [FR-008](../functional/FR-008-bounded-canonical-synthesis.md) path.

## Expected Results

Every complete PHAVer enclosure for a fit case contains every integer state found by the independent oracle for all modes and every step in its requested horizon. The F10 requests with 100,000 transitions return Incomplete with reason tool_timeout at the 5,000 ms NFR-001 limit and return no enclosure; that expected bounded outcome passes AC4 and does not fail the fit check. Any other missing state, incomplete fit case, malformed response, or out-of-limit run fails the fit check. Stub failures end only in the named Incomplete or Refused outcome and never return an enclosure. The registered tool call uses producer-execute, obeys NFR-001 limits, and no direct tool process is started by crate code. Canonical synthesis behavior remains unchanged.

## Limitations

The oracle covers the finite integer state sets in the declared cases. It does not prove soundness for arbitrary hybrid automata or for models outside the VER-94 case set.
