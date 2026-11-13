---
id: FR-040
title: "State the soundness obligations of a hybrid reachability enclosure"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-007
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-040: State the soundness obligations of a hybrid reachability enclosure

## Description

The analysis provider SHALL include in a returned hybrid reachability enclosure every state that any concrete execution of the bounded model can reach, under the semantics the result names.

When the provider cannot enclose a request that way, the provider SHALL end the request as a typed non-conclusion.

This requirement refines [FR-007](./FR-007-sound-hybrid-reachability.md) into obligations that an adversarial test can fail. A concrete execution is a sequence of flow steps and discrete transitions that respects the submitted flow bounds, guards and resets, and "reach" means reachable by such an execution within the declared horizon. Intervals are closed 64-bit integer intervals. A mode is named by a non-empty mode identity, either the initial mode or a transition endpoint; the request carries no separate mode list and no invariant.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-040-AC-1 | A bound computation that stays inside the 64-bit range gives the exact value, never a narrower interval. A flow step, reset or widening that adds to the interval's lower bound never returns a value above the exact sum, and one that adds to its upper bound never returns a value below it. | Test (TC-040) |
| FR-040-AC-2 | A computed bound that cannot be represented in the 64-bit interval type (overflow, underflow or saturation), including one reached at a bound one unit from the representable limit, yields `Incomplete` or `Refused`. The provider never clamps such a bound into an enclosure that excludes states beyond the clamp. | Test (TC-040) |
| FR-040-AC-3 | A guard is closed: a flowed interval that touches the guard at exactly one point enables the transition, and that boundary state is inside the target-mode enclosure. | Test (TC-040) |
| FR-040-AC-4 | When a guard overlaps the flowed interval without covering it, the source mode keeps the whole flowed interval, including the part that satisfies the guard, because a state inside the guard may also not take the transition. The target mode receives the reset image of the part that satisfies the guard. Only a result that names a forced-transition semantics may remove the guard-satisfying part from the source mode, and it still keeps every state outside the guard. The result names which semantics applied. | Test (TC-040) |
| FR-040-AC-5 | When several transitions are enabled from one mode, each target mode receives the join of every contribution from every enabled transition. The join is the smallest interval that contains all contributions. No contribution is dropped by order or by deduplication, and a transition repeated verbatim gives the same enclosure as the transition once. | Test (TC-040) |
| FR-040-AC-6 | A reset maps only the part of the flowed interval that satisfies the guard. A request whose reset image of the whole flowed interval would contain a state that no concrete execution reaches is a precision loss, and the review records it as a precision finding. A result that omits a reachable state is an unsound finding. | Test (TC-040) |
| FR-040-AC-7 | The request type carries no mode invariant, so an invariant cannot reach the provider. A caller or adapter that translates a model with a mode invariant refuses it as unsupported before building the request, and never builds a request that drops the invariant. If the review adds an invariant field, a reset image or flow step that leaves it is excluded as blocked or ends `Incomplete`, and is never reported as an invariant-respecting state. | Test (TC-040) |
| FR-040-AC-8 | The result names whether each enclosure covers the states at the horizon step only or every state at every step up to and including the horizon, and the enclosure is sound for exactly the named claim. | Test (TC-040) |
| FR-040-AC-9 | A horizon above the admitted step bound ends `Incomplete`. The iteration count of the enclosure method is measured by the provider, not read from the caller's `convergence_steps` field; a caller-reported count above the step bound ends `Incomplete`, and a count that the provider measures above the bound ends `Incomplete`. No end of computation before the horizon returns an enclosure for the full horizon. | Test (TC-040) |
| FR-040-AC-10 | The declared error bound is applied outward on both ends of every final enclosure and is reported as a declared assumption. The result does not present it as a bound the provider has verified. | Test (TC-040) |
| FR-040-AC-11 | Each of these inputs ends `Refused` and never as an enclosure: an interval whose lower bound exceeds its upper bound, an empty mode identity on the initial mode or on a transition endpoint, and a flow whose lower delta exceeds its upper delta. A transition whose endpoint is not the initial mode or another transition's endpoint is well formed and is an unreached mode. | Test (TC-040) |
| FR-040-AC-12 | Every request ends in exactly one of `Enclosure`, `Incomplete` or `Refused`, and no failure path of the computation reports an enclosure or a proved result. | Test (TC-040) |

## Dependencies

- [FR-007](./FR-007-sound-hybrid-reachability.md) owns the enclosure, witness and typed non-conclusion contract that these obligations refine.
- [StR-001](../stakeholder/StR-001-reviewable-analysis.md) owns the reviewable-analysis need.
