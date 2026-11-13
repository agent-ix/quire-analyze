---
id: SR-17045
title: "Failure-domain review of VER-93 hybrid enclosure soundness obligations"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-analyze; quire-analyze#89 diff origin/main...HEAD: spec/functional/FR-040-hybrid-enclosure-soundness.md, spec/functional/FR-041-hybrid-reachability-review.md, spec/functional/FR-042-hybrid-reachability-disposition.md, spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md, spec/test/TC-040-hybrid-enclosure-soundness.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-93. Failure-domain lens over the FR-040 soundness obligations, NFR-040 bounds and the FR-041/FR-042 disposition paths, checked against `reach` and `validate_hybrid` in src/hybrid_synthesis.rs. The author's unconfirmed observations were checked against the code and all hold: bounds use `saturating_add`/`saturating_sub` (190-193, 209-210, 223-224); when any transition is enabled the source-mode states are dropped (201-214); `HybridRequest` has no invariant field (86-106); only the horizon step is returned because `current` is replaced each step (186-217); `error_bound` is caller-supplied and only widens (185, 223-224). FR-040-AC-2, AC-8 and AC-10 cover saturation, the final-step claim and the caller-supplied error bound correctly. FR-040-AC-3, AC-5 and AC-12 are sound and testable.

## Scope examined

- FR-040-AC-2 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-3 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-4 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-5 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-7 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-8 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-9 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-10 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-11 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-040-AC-12 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- NFR-040-AC-1 (examined): spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md
- FR-041-AC-6 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-042-AC-3 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md
- FR-042-AC-4 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md
- FR-042-AC-6 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-040-AC-4 exempts the source-mode states 'unless the result names a semantics in which the transition is forced when enabled'. Under forced (urgent) semantics only states inside the guard must take the transition; states of the flowed interval outside the guard cannot take it (FR-040 says a concrete execution respects guards) and still remain in the source mode. The exemption therefore lets `reach`, which drops every source-mode state once any transition is enabled (src/hybrid_synthesis.rs:201-214), pass AC-4 by naming a semantics, and it omits reachable states. Fix: limit the exemption to the guard-satisfying states; the states outside every enabled guard always stay in the source-mode enclosure. | spec/functional/FR-040-hybrid-enclosure-soundness.md:30, src/hybrid_synthesis.rs:201-214 | wrong-requirement |
| FND-002 | medium | FR-040-AC-7 requires a declared invariant to be honoured or `Refused`, but `HybridRequest` has no invariant or other model-feature field (src/hybrid_synthesis.rs:86-106), so no request to `reach` can declare one and the 'Refused as unsupported' clause cannot be tested there. An invariant is dropped where a source hybrid model is lowered into `HybridRequest`, and the spec does not name that boundary. Fix: name the boundary (the lowering into `HybridRequest`, or the request type itself gaining the field) at which an unsupported feature must be refused. | spec/functional/FR-040-hybrid-enclosure-soundness.md:33, src/hybrid_synthesis.rs:86-106 | missing-requirement |
| FND-003 | medium | FR-040-AC-11 refuses transitions whose source or target mode 'is not declared' and an undeclared initial mode, but `HybridRequest` has no mode declaration set: modes exist only as strings in `initial_mode` and transitions, and `validate_hybrid` checks only that they are non-empty (src/hybrid_synthesis.rs:529-543). Two implementers would read 'declared' differently. Fix: define the declared mode set (for example, an explicit `modes` list) in FR-040. | spec/functional/FR-040-hybrid-enclosure-soundness.md:37, src/hybrid_synthesis.rs:529-543 | missing-requirement |
| FND-004 | medium | NFR-040-AC-1 bounds work by 'the step bound', but `step_bound` is caller-supplied with no crate ceiling and transitions are unbounded in number (src/hybrid_synthesis.rs:87-106, 179). horizon = step_bound = u64::MAX is admitted and loops 2^64 times, each over every mode times every transition, so a request with 'a maximal horizon' does not return in practice. Contrast `synthesize`, which has MAX_SYNTHESIS_TERMS and MAX_SYNTHESIS_ATOMS. Fix: require crate-owned numeric ceilings on step bound, transition count and mode count, with exact boundary tests. | spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md:29, src/hybrid_synthesis.rs:179-217 | missing-requirement |
| FND-005 | medium | FR-040-AC-9's 'iteration count above the admitted step bound yields Incomplete' maps onto `convergence_steps`, which the caller supplies and the provider never measures (src/hybrid_synthesis.rs:104-105, 179). A caller can report 0 and always get an enclosure. Fix: state that the iteration count is the provider's own count, or remove the caller-reported field from the obligation. | spec/functional/FR-040-hybrid-enclosure-soundness.md:35, src/hybrid_synthesis.rs:104-105 | wrong-requirement |
| FND-006 | medium | FR-041-AC-6 and FR-042-AC-6 judge against 'the engine-ownership ruling for hybrid models recorded in IR-1469'. IR-1469 is the right ticket and is cited by id only, but it is in Backlog and records no ruling today. The spec does not say whether the disposition waits for that ruling or proceeds without it. Fix: state the order (FR-041 records its disposition and IR-1469 consumes it, or FR-041 waits for IR-1469), or reword to 'states the ruling it assumes'. | spec/functional/FR-041-hybrid-reachability-review.md:31, spec/functional/FR-042-hybrid-reachability-disposition.md:30 | missing-requirement |
| FND-007 | medium | FR-042-AC-4 maps 'a complete result with a bound' from the external tool straight to an `Enclosure`, and FR-042-AC-3's 'checked before use' names no check. The external tool is untrusted, so a wrong bound becomes a crate enclosure with nothing comparing it to FR-040. The failure mapping also leaves open which tool failures are `Incomplete` and which are `Refused`. Fix: state the check (for example, FR-040's soundness obligations on the parsed enclosure, or an independent validation as FR-008 does for synthesis) and give each tool failure one outcome. | spec/functional/FR-042-hybrid-reachability-disposition.md:27-28 | missing-requirement |

## Verdict

FAIL on one high finding: FR-040-AC-4's 'forced when enabled' exemption lets the current unsound drop of non-taking source-mode states pass. Six medium findings name boundaries that the obligations depend on but the request type cannot express or the crate does not bound.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | The fix makes the default (non-forced) case keep only 'the part of the flowed interval that does not satisfy the guard' in the source mode. Under the default may-transition semantics a state that satisfies the guard can also decline the transition and stay in the source mode, so dropping the guard-satisfying part omits reachable states. The default sentence and the forced-semantics sentence now say the same thing, which shows the distinction collapsed. Fix: in the default semantics the source mode keeps the whole flowed interval and the target receives the reset image of the guard-satisfying part; only a named forced semantics drops the guard-satisfying part from the source. | spec/functional/FR-040-hybrid-enclosure-soundness.md:28 |
| FND-009 | low | A request above a crate limit 'ends Refused or Incomplete', so two implementers can pick different outcomes, and a step bound above the crate limit overlaps FR-040-AC-9 (horizon above the admitted step bound ends Incomplete). Fix: give each limit one outcome, for example Refused for a request above a crate limit and Incomplete only for exhaustion within it. | spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md:30, spec/functional/FR-040-hybrid-enclosure-soundness.md:33 |

## Dispositions

Round 1, checked against fix commit 83fcd4e on ver-93-hybrid-spec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

Round 2, checked against fix commit cec5bd3 on ver-93-hybrid-spec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
