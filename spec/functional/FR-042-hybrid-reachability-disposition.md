---
id: FR-042
title: Apply the recorded hybrid reachability disposition
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

Keep-and-harden turns every unsound case from the review into a sound enclosure or a typed non-conclusion. Delegate-and-remove deletes the in-house reachability algorithm and runs the external tool through producer-execute as [FR-009](./FR-009-package-runner.md) runs solvers. When the recorded tool is none, delegate-and-remove runs no tool and ends every hybrid request Refused.

The crate SHALL contain no fallback from one path to the other.

When delegate-and-remove names an external tool, the application SHALL compare that tool's enclosure with the independent finite-execution cases recorded by FR-041 before removing the in-crate algorithm. The external-tool invocation and result checks are specified by [TC-041](../test/TC-041-delegated-hybrid-reachability.md).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-042-AC-1 | Where the recorded path is keep-and-harden, each case in the review's adversarial set yields either an enclosure containing every enumerated reachable state or a typed Incomplete or Refused outcome. This branch does not apply when the recorded path is delegate-and-remove. | Test |
| FR-042-AC-2 | Under delegate-and-remove, reach, replay_hybrid, bind_hybrid_result and hybrid_method_outcome are absent from the crate, no code path computes a hybrid enclosure inside the crate, and FR-007, FR-040, NFR-040, TC-013 and TC-040 with their index entries and test-matrix rows are removed. For each fit case in TC-041 that returns an enclosure, the enclosure contains every state reached by the independent oracle for every mode and step. | Test, Inspection |
| FR-042-AC-3 | Under delegate-and-remove with a named tool, the tool is started only through producer-execute with the time and output limits of NFR-001, and a stub executor records exactly one request per hybrid call. | Test |
| FR-042-AC-4 | Under delegate-and-remove with the tool none, every hybrid request ends Refused, the external invocation count is zero, and no enclosure is returned. With a named tool, the crate validates the declared result schema, requires every requested mode to be present or explicitly empty, rejects unknown members and inverted intervals, and maps timeout to Incomplete with reason tool_timeout, over-limit output and non-zero exit to Incomplete, and a missing tool to Refused with reason tool_unavailable. Schema-invalid or inverted output maps to Refused; a complete valid result maps to an enclosure. No failed tool call maps to an enclosure. | Test |
| FR-042-AC-5 | The applied path is the recorded one only. Neither path contains a switch, flag or retry that selects the other. | Inspection |
| FR-042-AC-6 | When IR-1469 is recorded, the applied path agrees with it, or the application ticket records why it does not. When IR-1469 is not recorded, the application ticket records that fact and the path proceeds. | Inspection |
| FR-042-AC-7 | When a named external tool is selected, the application pull request records the tool version, the license terms published by its authors, and evidence that invoking it as an external executable is permitted. | Inspection |

## Dependencies

- [FR-041](./FR-041-hybrid-reachability-review.md) supplies the recorded disposition and test set.
- [FR-009](./FR-009-package-runner.md) owns the producer-execute invocation model.
- [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md) owns the time and output limits.
