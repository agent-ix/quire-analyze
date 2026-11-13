---
id: SR-17046
title: "Integrity review of VER-70 linear integer arithmetic spec"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze; quire-analyze#90 diff origin/main...HEAD: spec/functional/FR-002, FR-009, FR-010, FR-011, FR-012; spec/nonfunctional/NFR-004; spec/test/TC-016..018; spec/index.md; spec/test-matrix.md"
review_set: subset
---

## Summary

Ticket: VER-70. Consistency check of the PR #90 diff (FR-010..012, NFR-004, TC-016..018 and the FR-002, FR-009, index and test-matrix edits) against unedited FR-001, FR-002, FR-005, FR-009, interface_001, src/smt.rs and src/report.rs on main, and quire-contract-ir and quire-verification-contracts main. The quire.op.integer.* ids FR-009-AC-23 and FR-012 use (negate, add, sub, mul, div, rem, mod, eq, ne, lt, le, gt, ge) exist in contracts/checked-operation-catalog-v1.json on quire-verification-contracts main (blob identical on GitHub and the fetched origin/main). Five contradictions remain: the 64-bit range claim, range assertions vs the AssertionMap/report v2 contract, and stale Boolean-only statements in FR-009 and FR-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-010 says a literal or declared bound outside 'the signed 64-bit range of the IR' is refused by the IR. On quire-contract-ir main, IntegerType minimum/maximum and ExpressionKind::IntegerLiteral value are i128 (crates/quire-contract-model/src/expression.rs:78-85, 1152-1155); there is no 64-bit bound. The range the lowering, the model parser and the exact replay arithmetic must handle is i128, and intermediate sums of i128 bounds exceed i128. Fix: state the i128 range, and the width of the replay arithmetic. | spec/functional/FR-010-linear-integer-arithmetic.md:43-48 |
| FND-002 | high | NFR-004-AC-2 tests literals and bounds at the signed 64-bit extremes. The IR admits i128 bounds, so a test at i64::MIN/MAX passes while a bound at i128::MIN/MAX (the real hostile extreme) can still panic or wrap. Fix: test at i128::MIN and i128::MAX for literals, declared bounds and model values. | spec/nonfunctional/NFR-004-integer-exactness.md:38 |
| FND-003 | high | FR-010 puts each declared-range assertion in the assertion map 'with the symbol's declaration origin'. On main, AssertionMap (src/smt.rs:437-444) requires clause, clause_digest, source, role and polarity, and the closed quire.differential-report/v2 schema (FR-005-AC-6) requires assertions[].clause, assertions[].source and assertions[].clauseDigest for every assertion. A range assertion has no clause, digest, role or polarity, and FR-010 does not amend FR-005. Fix: either keep range assertions out of the assertion map and report them in the variable map, or amend the AssertionMap and FR-005 v2 contract in this PR to define a declaration-origin assertion entry. | spec/functional/FR-010-linear-integer-arithmetic.md:39-42 |
| FND-004 | high | FR-009 still says integer types, IntegerDomain and OverflowPolicy are not used and 'a declaration of any type other than Boolean is not produced (Dependencies, gap 5)'; gap 5 still says v1/QF_UF lists integers as unsupported; and the Terms paragraph still says the Boolean subset 'is exactly the exact_constructs of CAPABILITY_CONTRACT'. FR-012 now produces integer declarations with integer_range bounds and FR-010 adds integer exact constructs, so FR-009 contradicts FR-012/FR-010. Fix: amend these three FR-009 passages (gap 5 asked for exactly this extension). | spec/functional/FR-009-package-runner.md:120-126, 160-161, 360-364 |
| FND-005 | high | FR-001 (not edited) says the v0.1 analysis surface accepts exactly the Boolean constructs of quire.smtlib2/v1 and every arithmetic construct rejects before a query. FR-010 admits linear integer arithmetic under v2, which replaces v1. Fix: amend FR-001 to name v2 and the FR-010 constructs. | spec/functional/FR-001-analysis-algebra.md:55-58 |
| FND-006 | medium | FR-002 keeps stating quire.smtlib2/v1 as Boolean-only QF_UF and lists constructs 'unsupported by this profile', while the new bullet says v2 replaces v1. After the change no v1 query exists, so 'this profile' points at a retired profile, and neither FR-002 nor FR-010 says which logic a Boolean-only query declares under v2 (QF_LIA, or QF_UF as today). TC-010 goldens and tests/smt_lowering.rs:838 depend on that. Fix: restate FR-002 for v2 and state the logic every v2 query declares. | spec/functional/FR-002-smt-lowering.md:24-31 |
| FND-007 | medium | The report validator on main accepts only encodingProfile 'quire.smtlib2/v1' and logic 'QF_UF' (src/report.rs:336-343). FR-012-AC-1 requires validate_report_document to accept an integer report, but neither FR-010 nor FR-005 states the v2 report values or that a report carrying the v1 profile is now refused. Fix: state in FR-010 (or FR-005) that reports carry encodingProfile quire.smtlib2/v2 and its logic, and that the validator refuses v1, with an AC. | spec/functional/FR-010-linear-integer-arithmetic.md:27-30 |
| FND-008 | medium | FR-010 implements interface_001, but interface_001's lowering_v1 block still states logic QF_UF and lists arithmetic as unsupported. Fix: update the interface block to the v2 profile, logic and supported set. | spec/interface/interface-001-analysis-api.md:68-73 |
| FND-009 | low | FR-010-AC-9 says 'With the profile selected', which implies a selectable profile, while FR-010 says v2 replaces v1 with no second profile. Fix: drop 'With the profile selected'. | spec/functional/FR-010-linear-integer-arithmetic.md:77 |
| FND-010 | medium | The FR-009-AC-23 edit removes the 'integer-typed parameter operand of a Boolean operation' case. That input is still a refusal under FR-012 (an integer operand to quire.op.boolean.and is ill-typed), so the refusal loses its AC. TC-015 case 19 (spec/test/TC-015-package-runner.md:59-67) still describes the integer_range and integer-operand cases against AC-23. Fix: keep the integer-operand-of-Boolean-op case in AC-23 (or move it to FR-012-AC-3) and update TC-015 case 19. | spec/functional/FR-009-package-runner.md:320 |

## Scope

| Unit | Role | Path |
| --- | --- | --- |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-9 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| NFR-004-AC-2 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| FR-002 | examined | spec/functional/FR-002-smt-lowering.md |
| FR-009 | examined | spec/functional/FR-009-package-runner.md |
| FR-009 | examined | spec/functional/FR-009-package-runner.md |
| FR-009-AC-23 | examined | spec/functional/FR-009-package-runner.md |
| FR-001 | examined | spec/functional/FR-001-analysis-algebra.md |
| interface_001 | examined | spec/interface/interface-001-analysis-api.md |
| FR-012 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-1 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-3 | examined | spec/functional/FR-012-integer-claim-translation.md |

## Verdict

FAIL: FND-001 to FND-005 are contradictions a coder cannot resolve without a spec change. Clean: the v2-replaces-v1 wording adds no fallback or compatibility reader (ADR-0010 already says profile changes invalidate evidence with no silent migration); ids FR-010..012, NFR-004 and TC-016..018 do not collide with PRs #87 (FR-030..034), #88 (FR-020..022, NFR-010) or #89 (FR-040..042); test-matrix rows are only edited by this PR; no hash, pin or version-tracking content, no paywalled text and no roadmap content was added.
