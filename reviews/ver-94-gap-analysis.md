---
id: SR-17051
title: "Gap analysis of the VER-94 hybrid enclosure review against FR-040, FR-041, TC-040 and NFR-040"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; tests/hybrid_enclosure_soundness.rs, src/hybrid_synthesis.rs, spec/functional/FR-040-hybrid-enclosure-soundness.md, spec/functional/FR-041-hybrid-reachability-review.md, spec/test/TC-040-hybrid-enclosure-soundness.md, spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md, VER-94 findings draft"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-040", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-041", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-040", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-040", type: reviews }
---
# Gap Analysis of the VER-94 Hybrid Enclosure Review

## Summary

Ticket: VER-94. Plan completion: not assessed. Every FR-040-AC-1..12 and
NFR-040-AC-2 has at least one test named after it and traced with
`#[trace("TC-040", ...)]`. Findings F1, F2, F3, F4, F5, F8 and F9 in the draft
reproduce exactly (measured with `--ignored`). F7 reproduces; its unsound label
holds only under the every-step reading, which the draft states. The callers
list (reach re-exported by `pub use hybrid_synthesis::*` in src/lib.rs, called by
replay_hybrid and tests; replay_hybrid test-only; no other HybridRequest
construction in src or crates) is accurate.

External-tool research (FR-041-AC-4, sources read, nothing installed): the model
class is a one-variable discrete-step system with interval nondeterministic
additive flow, closed interval guards, additive resets and non-forced
transitions. With a clock that dwells one time unit per step it is a linear
hybrid automaton (constant derivative bounds, affine resets, may-transitions),
the class PHAVer (also the PHAVer/SX scenario of SpaceEx) analyses exactly in
rational arithmetic (Frehse, STTT 2008, doi 10.1007/s10009-007-0062-x). JuliaReach
ReachabilityAnalysis.jl also supports hybrid automata whose discrete transitions
are affine assignments post_d(X) = (R(X ∩ G ∩ I-) ⊕ W) ∩ I+, plus discrete-time
systems (juliareach.github.io/ReachabilityAnalysis.jl, lib/systems). With integer
data in one dimension the real hull equals the integer hull, and neither tool
has 64-bit overflow. So an established external tool supports the model class.

With unsound findings that all have a bounded fix and a supporting tool, case
(1) of FR-041-AC-4 applies: delegate-and-remove with that tool. IR-1469 is cited
by id only and not decided here.

## Verdict

**FAIL.** The draft's recommended disposition (keep-and-harden, case 3) does not
follow the ordered rule once the tool fact is measured, and its wording is a
conditional switch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Draft disposition recommends keep-and-harden under case (3) with the tool fact unmeasured and phrased "unless the reviewer identifies ... in which case case (1)"; an established tool supports the class (PHAVer/SpaceEx LHA, JuliaReach), so case (1) delegate-and-remove applies, and the conditional wording is what FR-041-AC-5 calls incomplete. | `ver-94-findings.md:32-39` |
| FND-002 | medium | NFR-040-AC-1 has no test and no recorded finding: the module states no hybrid limit for step bound, transitions or distinct modes, and TC-040 requires the crate-limit requests of NFR-040. | `src/hybrid_synthesis.rs:172-238` |
| FND-003 | medium | F6 is recorded as a precision finding although, under FR-041-AC-2, the result contains no extra states for the submitted model and AC-7 holds (no invariant field, no adapter). F6 should be withdrawn and AC-7 recorded clean. | `ver-94-findings.md:18` |
| FND-004 | low | AC-4 "the result names which semantics applied" is a statically measured violation (HybridOutcome has no semantics field) but is listed as untestable instead of being recorded with F4, as F7 does for AC-8. | `ver-94-findings.md:23` |
| FND-005 | low | FR-041-AC-1 asks for a test of "a reset that leaves an invariant"; the set has only a flow step against a dropped invariant. | `tests/hybrid_enclosure_soundness.rs:434-452` |
| FND-006 | low | F1 row says the exact lower bound MIN-1 "is above the clamped value"; it is below it. | `ver-94-findings.md:13` |
| FND-007 | medium | The draft does not state whether IR-1469 is recorded, as FR-041-AC-6 requires; IR-1469 is in Backlog (not recorded), so the disposition proceeds and the application ticket re-checks agreement under FR-042-AC-6. | `ver-94-findings.md:41` |

## Coverage

FR-040-AC-1..12, FR-041-AC-1..7, NFR-040-AC-1..3 and TC-040 examined. Oracle is
independent of src/hybrid_synthesis.rs and correct. src/hybrid_synthesis.rs is
unchanged by the PR.

## Dispositions

Round 1, verified against commit 0b18e9c and the reworked findings draft.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The draft records exactly one disposition, delegate-and-remove with PHAVer (SpaceEx PHAVer/SX), under FR-041-AC-4 case (1), with no conditional wording. |
| FND-002 | fixed | The ignored test `nfr040_ac1_no_crate_limits_on_steps_transitions_modes` is added and F10 is recorded. |
| FND-003 | fixed | F6 is withdrawn and AC-7 is recorded clean. |
| FND-004 | fixed | The missing semantics field is recorded with F4. |
| FND-005 | accepted-no-change | No request can carry an invariant, so a reset leaving one cannot be built; the test module records FR-041-AC-1's clause as not applicable and the AC-7 test pins the absence. |
| FND-006 | fixed | The F1 row now says MIN-1 is below the clamped value. |
| FND-007 | fixed | The draft states IR-1469 is in Backlog and not recorded, and that the application ticket re-checks agreement under FR-042-AC-6. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | F9 is classed "unsound (non-termination in practice)". It omits no reachable state and reports no unreached horizon as reached, so under FR-041-AC-2 it is an NFR-040 violation, not an unsound finding. Criterion 1 already counts it apart, so the disposition is unaffected. | `ver-94-findings.md:22` |
| FND-009 | medium | The disposition states the tool facts as established, but they come from literature and docs only. No tool was run, the one-clock encoding was not exercised, and the JuliaReach docs show continuous-time hybrid automata and discrete-time systems only separately. The draft does not say these facts are unverified in practice, or that the application ticket must confirm them with the TC-040 enumeration as a differential. | `ver-94-findings.md:41` |
