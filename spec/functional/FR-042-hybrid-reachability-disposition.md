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
| FR-042-AC-2 | Under delegate-and-remove, reach, replay_hybrid, bind_hybrid_result and hybrid_method_outcome are absent from the crate, and no code path computes a hybrid enclosure inside the crate. | Inspection |
| FR-042-AC-3 | Under delegate-and-remove, FR-007, FR-040, NFR-040, TC-013 and TC-040, including their index entries and test-matrix rows, are removed. | Inspection |
| FR-042-AC-4 | For each fit case in TC-041 that returns an enclosure, the enclosure contains every state reached by the independent oracle for every mode and step. | Test |
| FR-042-AC-5 | Under delegate-and-remove with a named tool, the tool is started only through producer-execute with the time and output limits of NFR-001, and a stub executor records exactly one request per valid hybrid call. Malformed request models are rejected before producer-execute and cause zero external launches. | Test |
| FR-042-AC-6 | Under delegate-and-remove with the configured tool none, every valid hybrid request ends Refused with reason `tool_not_configured`, producer-execute request count and external invocation count are both zero, and no enclosure is returned. | Test |
| FR-042-AC-7 | With a named tool, the crate validates the exact declared result schema, requires every requested mode at every requested step to be present exactly once or explicitly empty, and rejects unknown or missing members and inverted intervals. Timeout maps to Incomplete with reason `tool_timeout`; output-limit breach maps to Incomplete with reason `tool_output_limit_exceeded`; non-zero exit maps to Incomplete with reason `tool_exit_nonzero`; missing tool maps to Refused with reason `tool_unavailable`; schema-invalid output maps to Refused with reason `result_schema_invalid`; and an inverted interval maps to Refused with reason `interval_inverted`. A complete valid result maps to an enclosure. No failed tool call maps to an enclosure. | Test |
| FR-042-AC-8 | The applied path is the recorded one only. Neither path contains a switch, flag or retry that selects the other. | Inspection |
| FR-042-AC-9 | When IR-1469 is recorded, the applied path agrees with it, or the application ticket records why it does not. When IR-1469 is not recorded, the application ticket records that fact and the path proceeds. | Inspection |
| FR-042-AC-10 | When a named external tool is selected, the application pull request records the tool version, the license terms published by its authors, and evidence that invoking it as an external executable is permitted. | Inspection |

## Dependencies

- [FR-041](./FR-041-hybrid-reachability-review.md) supplies the recorded disposition and test set.
- [FR-009](./FR-009-package-runner.md) owns the producer-execute invocation model.
- [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md) owns the time and output limits.
