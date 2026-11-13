---
id: FR-030
title: "State the soundness obligations of a hybrid reachability enclosure"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-007
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-030: State the soundness obligations of a hybrid reachability enclosure

## Description

When the analysis provider returns a hybrid reachability enclosure for a bounded model, the analysis
provider SHALL include in the enclosure every state that any concrete execution of that model can reach,
under the semantics the result names, and SHALL end every request it cannot enclose that way as a typed
non-conclusion. This requirement refines [FR-007](./FR-007-sound-hybrid-reachability.md) into
obligations that an adversarial test can fail. A concrete execution is a sequence of flow steps and
discrete transitions that respects the submitted flow bounds, guards and resets; "reach" below means
reachable by such an execution within the declared horizon.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-030-AC-1 | Interval bounds only widen: every arithmetic step that produces a lower bound rounds toward negative infinity and every step that produces an upper bound rounds toward positive infinity. For a model whose bounds sit within one unit of a representable boundary, the returned lower bound is at most, and the upper bound at least, the exact value; a lower bound above or an upper bound below the exact value is a failure. | Test (TC-030) |
| FR-030-AC-2 | A computed bound that cannot be represented in the interval type (overflow, underflow, saturation, a non-finite value) yields `Incomplete` or `Refused`; the provider never clamps such a bound into an enclosure that excludes states beyond the clamp. | Test (TC-030) |
| FR-030-AC-3 | A guard is closed unless the model says otherwise: a flowed interval that touches the guard at exactly one point enables the transition, and the exact boundary state is inside the target-mode enclosure. | Test (TC-030) |
| FR-030-AC-4 | When a guard overlaps the flowed interval without covering it, the result contains the states that take the transition and, in the source mode, the states that do not, unless the result names a semantics in which the transition is forced when enabled. The result names which semantics applied. | Test (TC-030) |
| FR-030-AC-5 | When several transitions are enabled from one mode, each target mode receives the join of every contribution from every enabled transition; the join is the smallest interval containing all contributions, and no contribution is dropped by order or by deduplication. | Test (TC-030) |
| FR-030-AC-6 | A reset maps only the part of the flowed interval that satisfies the guard. The target enclosure is that image, not the image of the whole flowed interval and not the whole interval. | Test (TC-030) |
| FR-030-AC-7 | When the model declares a mode invariant, a reset image or flow step that leaves the invariant is handled by the semantics the result names, either as excluded (blocked) states or as `Incomplete`; it is never reported inside an enclosure as an invariant-respecting state. A model feature the provider does not implement, including an invariant, is `Refused` as unsupported and is not ignored. | Test (TC-030) |
| FR-030-AC-8 | The result names whether each enclosure covers the states at the horizon step only or every state at every step up to and including the horizon, and the enclosure is sound for exactly the named claim. | Test (TC-030) |
| FR-030-AC-9 | A horizon or iteration count above the admitted step bound yields `Incomplete`; a horizon that is not reached because the computation ends early never yields an enclosure for the full horizon. | Test (TC-030) |
| FR-030-AC-10 | The declared error bound is applied outward on both ends of every final enclosure and is reported as a declared assumption. The result does not present it as a bound the provider has verified. | Test (TC-030) |
| FR-030-AC-11 | Each of these inputs yields `Refused` and never an enclosure: an interval whose lower bound exceeds its upper bound, a transition whose source or target mode is not declared, an initial mode that is not declared, and a flow whose lower delta exceeds its upper delta. | Test (TC-030) |
| FR-030-AC-12 | Every request ends in exactly one of `Enclosure`, `Incomplete` or `Refused`, and no failure path of the computation reports an enclosure or a proved result. | Test (TC-030) |

## Dependencies

- [FR-007](./FR-007-sound-hybrid-reachability.md) owns the enclosure, witness and typed
  non-conclusion contract that these obligations refine.
- [StR-001](../stakeholder/StR-001-reviewable-analysis.md) owns the reviewable-analysis need.
