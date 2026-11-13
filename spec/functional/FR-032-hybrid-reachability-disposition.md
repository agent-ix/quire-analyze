---
id: FR-032
title: "Apply the recorded hybrid reachability disposition"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-031
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-009
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-032: Apply the recorded hybrid reachability disposition

## Description

When a disposition is recorded under [FR-031](./FR-031-hybrid-reachability-review.md), the maintainers SHALL carry out exactly that path.

Keep-and-harden turns every unsound case from the review into a sound enclosure or a typed non-conclusion. Delegate-and-remove deletes the in-house reachability algorithm and runs the external tool through `producer-execute` as [FR-009](./FR-009-package-runner.md) runs solvers. The crate has no fallback from one path to the other.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-032-AC-1 | Under keep-and-harden, every test of the FR-031 adversarial set passes: each case yields either an enclosure that contains all reachable states or a typed `Incomplete` or `Refused` outcome. | Test (TC-030) |
| FR-032-AC-2 | Under delegate-and-remove, `reach`, `replay_hybrid` and FR-007 are absent from the crate and the spec, and no code path computes a hybrid enclosure inside the crate. | Inspection |
| FR-032-AC-3 | Under delegate-and-remove, the external tool is started only through `producer-execute` with the time and output limits of NFR-001, and its output is treated as untrusted input and checked before use. | Test (TC-030) |
| FR-032-AC-4 | Under delegate-and-remove, a tool result maps to the existing typed outcomes: a complete result with a bound maps to an enclosure; a tool timeout, over-limit output, a non-zero exit, a missing tool, or unparseable output maps to `Incomplete` or `Refused`; no tool failure maps to an enclosure. | Test (TC-030) |
| FR-032-AC-5 | The applied path is the recorded one only. Neither path contains a switch, flag or retry that selects the other. | Inspection |
| FR-032-AC-6 | The applied path agrees with the IR-1469 ruling, or the ticket that applies it records why it does not. | Inspection |

## Dependencies

- [FR-031](./FR-031-hybrid-reachability-review.md) supplies the recorded disposition and the test set.
- [FR-009](./FR-009-package-runner.md) owns the `producer-execute` invocation model.
- [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md) owns the time and output limits.
