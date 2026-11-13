---
id: SR-17037
title: "Spec review of VER-89 bounded quantifiers (FR-030..FR-034, NFR-030, TC-030)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; quire-analyze#87 diff origin/main...HEAD: spec/functional/FR-030..FR-034, spec/nonfunctional/NFR-030-quantifier-resource-bounds.md, spec/test/TC-030-bounded-quantifiers.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-89. Reviewer pass over PR #87 (spec-only), method spec-review/base. Examined units:
FR-030, FR-031, FR-032, FR-033, FR-034, NFR-030, NFR-030-AC-1, NFR-030-AC-2, FR-032-AC-3. Context only: FR-002, FR-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The expansion ceiling contradicts FR-032. NFR-030-AC-1 counts 4,096 instances per request over nested quantifiers and returns `unsupported` (`expansion-ceiling`) one above it. FR-032 compares each range quantifier's own width with the ceiling and emits a wider range as a bounded solver quantifier, not `unsupported`. FR-031 checks 'query size'. Two nested literal ranges of width 100 are expanded by FR-032 (each width <= 4,096) and refused by NFR-030 (10,000 > 4,096); a single range of width 4,097 is a bounded quantifier under FR-032 and `unsupported` under NFR-030. Fix: state one unit (instances per request) and say which forms the ceiling refuses (enum/collection) and which it reroutes to the bounded form (ranges). | spec/nonfunctional/NFR-030-quantifier-resource-bounds.md:36 |
| FND-002 | medium | The limits 4,096 (expansion) and 65,536 (replay) have no stated rationale, and they equal the unrelated NFR-001 byte ceilings (4,096-byte executable path, 65,536-byte version output). Their interaction with existing bounds is unspecified: FR-009 stops translation at 10,000 Expression nodes per claim and 100,000 in total, and FR-002 profiles carry statement/expression/query byte limits (MAX_EXPRESSION_NODES 100,000). An expansion of 4,096 instances of a 25-node body is 102,400 nodes, under the instance ceiling and over the node limit. Fix: give the reason for each number and state which bound applies first and what reason class the loser returns. | spec/nonfunctional/NFR-030-quantifier-resource-bounds.md:34-37 |
| FND-003 | medium | The FR-031 decidability argument is overstated. A quantifier-free expansion is decidable only when the body's own logic is decidable; 'both solvers decide it without quantifier instantiation heuristics' is false for a body with nonlinear integer arithmetic (exact under an integer profile, undecidable as QF_NIA). Fix: limit the claim to bodies in a decidable quantifier-free logic (QF_UF, QF_LIA, QF_DT combinations) and say that other bodies return `unknown` or `unsupported`, as FR-032 already does for the bounded form. | spec/functional/FR-031-enum-and-collection-quantifier-lowering.md:21-24 |
| FND-004 | low | FR-032 credits decidability to the finite interval guards. Quantified linear integer arithmetic (Presburger) is decidable with or without guards; what the guards add is replay finiteness. The 'declared quantified linear-integer logic' is also wrong once the body references enum or record sorts from VER-84 (then UFDTLIA or similar). Fix: state the actual reason and name the declared logic per body sort set. | spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md:27-31 |

## Verdict

FAIL on FND-001 (high). Checked and clean: unbounded quantification returns `unsupported` and is never approximated (FR-030 Behavior bullets 2-4); exhaustion is inconclusive (NFR-030-AC-3); replay is over the original quantifier, not the expansion (FR-033). No paywalled standard text, no roadmap or delivery-order content, no hash/SHA/version-tracking records (byte-identical lowering is a functional determinism property), and no compatibility layer. Numbering: FR-030..FR-034 / NFR-030 / TC-030 leaves the FR-010..FR-029 gap free for sibling milestones (VER-84, PR #88, uses FR-020 / TC-020) and is internally consistent; no defect. Sibling PR #88 uses NFR-010 rather than NFR-020, so the NFR block convention differs between the two PRs; that is PR #88's choice and is not a defect here.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in eb95c03 |
| FND-002 | fixed | verified in eb95c03 |
| FND-003 | fixed | verified in eb95c03 |
| FND-004 | fixed | verified in eb95c03 |
