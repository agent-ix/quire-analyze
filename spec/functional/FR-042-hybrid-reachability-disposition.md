---
id: FR-042
title: "Apply the recorded hybrid reachability disposition"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-041
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-009
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-042: Apply the recorded hybrid reachability disposition

## Description

When a disposition is recorded under [FR-041](./FR-041-hybrid-reachability-review.md), the maintainers SHALL carry out exactly that path.

Keep-and-harden turns every unsound case from the review into a sound enclosure or a typed non-conclusion. Delegate-and-remove deletes the in-house reachability algorithm and runs the external tool through `producer-execute` as [FR-009](./FR-009-package-runner.md) runs solvers. When the recorded tool is "none", delegate-and-remove runs no tool and ends every hybrid request `Refused`.

The crate SHALL contain no fallback from one path to the other.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-042-AC-1 | Under keep-and-harden, every test of the FR-041-AC-1 adversarial set passes: each case yields either an enclosure that contains all reachable states or a typed `Incomplete` or `Refused` outcome. | Test (TC-040) |
| FR-042-AC-2 | Under delegate-and-remove, `reach`, `replay_hybrid`, `bind_hybrid_result` and `hybrid_method_outcome` are absent from the crate, no code path computes a hybrid enclosure inside the crate, and FR-007, FR-040, NFR-040, TC-013, TC-040, their index lines and their rows in the test matrix are removed. FR-040 does not apply to a delegated result, because the external tool owns that result's soundness and the crate checks only the tool's output under FR-042-AC-4. | Inspection |
| FR-042-AC-3 | Under delegate-and-remove with a tool, the external tool is started only through `producer-execute` with the time and output limits of NFR-001. | Test (TC-040) |
| FR-042-AC-4 | Under delegate-and-remove with the tool "none", every hybrid request ends `Refused`, no process is started, and no enclosure is returned. Under delegate-and-remove with a tool, the crate checks the external tool's output against the declared result schema and checks that each reported interval has a lower bound not above its upper bound. A tool timeout, over-limit output, a non-zero exit or a missing tool maps to `Incomplete`. Output that fails the schema or the interval check maps to `Refused`. A complete, valid result with a declared bound maps to an enclosure. No tool failure maps to an enclosure. | Test (TC-040) |
| FR-042-AC-5 | The applied path is the recorded one only. Neither path contains a switch, flag or retry that selects the other. | Inspection |
| FR-042-AC-6 | When IR-1469 is recorded, the applied path agrees with it, or the application ticket records why it does not. When IR-1469 is not recorded, the application ticket records that fact and the path proceeds. | Inspection |

## Dependencies

- [FR-041](./FR-041-hybrid-reachability-review.md) supplies the recorded disposition and the test set.
- [FR-009](./FR-009-package-runner.md) owns the `producer-execute` invocation model.
- [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md) owns the time and output limits.
