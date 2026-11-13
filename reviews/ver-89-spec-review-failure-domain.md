---
id: SR-17040
title: "Failure-domain review of VER-89 bounded quantifiers"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-analyze; quire-analyze#87 diff origin/main...HEAD: spec/functional/FR-030..FR-034, spec/nonfunctional/NFR-030-quantifier-resource-bounds.md, spec/test/TC-030-bounded-quantifiers.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-89. Reviewer pass over PR #87 (spec-only), method spec-review/failure-domain. Examined units:
FR-030, FR-031, FR-032, FR-033, NFR-030-AC-2. Context only: FR-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Range width and instance counting can overflow. Width is upper - lower + 1 over integer literals or declared bounds; for a range such as [-2^63, 2^63-1] that overflows a 64-bit integer, and the nested product counted by NFR-030 overflows sooner. The spec gives no integer width for bounds and no rule for computing width or nested product without overflow. Fix: state that width and nested instance products are computed in checked or arbitrary-precision arithmetic, and that overflow counts as above the ceiling. | spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md:21-26 |
| FND-002 | medium | For a range with declared (non-literal) bounds, the replay domain depends on the model's values of those bounds, but FR-033 does not say so or say what happens when a bound symbol is missing or undecodable in the model, or when the model sets lower > upper. The instance count, and so the replay-limit check, is model-dependent. Fix: state that replay first decodes the bound values from the model, treats an undecodable bound as `incomplete`, and applies the empty-domain rule when lower > upper. | spec/functional/FR-033-quantified-assertion-replay.md:20-28 |
| FND-003 | low | 'Each instance substitutes the element for the bound variable' does not say the substitution is capture-avoiding. FR-034 gives reused names distinct symbols at translation, but FR-031 is stated over the analysis model and nested quantifiers there can shadow. Fix: say substitution replaces only free occurrences bound by this quantifier. | spec/functional/FR-031-enum-and-collection-quantifier-lowering.md:25 |
| FND-004 | low | 'with the lower bound not above the upper bound or the range empty' admits every range, so it constrains nothing and reads as a garbled condition. Fix: 'a range whose lower bound exceeds its upper bound is empty and is admitted under the empty-domain rule'. | spec/functional/FR-030-bounded-quantifier-domains.md:26-28 |

## Verdict

FAIL on two medium edge cases (overflow in width and instance counts; model-dependent replay domain). The empty-domain rule, nested refusal, refusal before solver invocation, and exhaustion-as-inconclusive are covered.
