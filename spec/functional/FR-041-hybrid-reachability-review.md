---
id: FR-041
title: Review the hybrid reachability module and record one disposition
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-041: Review the hybrid reachability module and record one disposition

## Description

When a ticket proposes work that builds on the in-house hybrid reachability module, the maintainers SHALL compare its reachable-state enclosures with an independent finite-execution enumeration and record each adversarial request and outcome.

The review SHALL record exactly one disposition: keep and harden the module, or remove it and delegate hybrid reachability to an established external reachability tool through producer-execute.

When no established external tool supports the model class, the maintainers SHALL record the tool none.

When the recorded tool is none, the analyzer SHALL end every hybrid request as Refused with zero external invocations.

The analyzer SHALL contain no fallback between the two paths.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-041-AC-1 | The review records a named adversarial set covering flow and reset overflow, outward widening, closed and overlapping guards, non-forced source-mode persistence, multiple transitions into one mode, invariant handling, horizon coverage and exhaustion, malformed inputs, and typed outcomes. Each case records its minimal request and the reachable states from independent finite-execution enumeration. | Inspection |
| FR-041-AC-2 | Every case that omits an enumerated reachable state or reports an unreached horizon as reached is recorded as an unsound finding with its case name and minimal request. A case that adds states without omitting reachable states is recorded as a precision finding. | Inspection |
| FR-041-AC-3 | The review lists the callers of reachability and replay operations and each requirement or planned consumer that depends on their output. | Inspection |
| FR-041-AC-4 | The recorded disposition is exactly one of keep-and-harden or delegate-and-remove. The reason is stated against the number and severity of unsound findings, whether each unsound finding has a bounded fix, whether an established external tool supports the model class, and the cost of maintaining an algorithm that no external engine cross-checks. An unsound finding with a bounded fix and a supported model class selects delegate-and-remove with that tool. An unsound finding without a bounded fix selects delegate-and-remove with a supporting tool, or with the tool none otherwise. When no tool supports a model class and every unsound finding has a bounded fix, or no unsound finding exists, the disposition is keep-and-harden. When a tool supports a model class and no unsound finding exists, keep-and-harden requires a differential check against that tool using the recorded finite-execution cases; otherwise the disposition is delegate-and-remove. | Inspection |
| FR-041-AC-5 | The disposition names no fallback between the two paths. A review that records both as alternatives or records a conditional switch from one to the other is incomplete. | Inspection |
| FR-041-AC-6 | The disposition states whether the engine-ownership ruling for hybrid models, IR-1469, is recorded when the disposition is made. When it is recorded, the disposition agrees with it or states why it does not. When it is not recorded, the disposition proceeds without waiting for it and the application ticket re-checks agreement under [FR-042](./FR-042-hybrid-reachability-disposition.md)-AC-6. | Inspection |
| FR-041-AC-7 | The findings and disposition are posted on the tracking ticket before the disposition is applied. | Inspection |

## Dependencies

- [FR-042](./FR-042-hybrid-reachability-disposition.md) governs application of the recorded disposition.
