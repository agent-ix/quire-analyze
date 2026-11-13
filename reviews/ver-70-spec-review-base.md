---
id: SR-17048
title: "Base checklist review of VER-70 linear integer arithmetic spec"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; quire-analyze#90 diff origin/main...HEAD: spec/functional/FR-002, FR-009, FR-010, FR-011, FR-012; spec/nonfunctional/NFR-004; spec/test/TC-016..018; spec/index.md; spec/test-matrix.md"
review_set: subset
---

## Summary

Ticket: VER-70. Base checklist (ID formats, AC testability, TC coverage, hash/pin antipattern scan) over the PR #90 diff. quire validate --scope . 'spec/**/*.md' exits 0 (42/42 docs grammar-clean). Every new AC maps to TC-016, TC-017 or TC-018 in the test matrix. Three ACs are not testable as written. criterion-strength analysis was not run: the quoin skill has no Jev client, so it is blocked. Its concerns are covered here by hand.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-010-AC-4 requires 'exponentiation' to return UnsupportedConstruct, and FR-010 Behavior lists modulo. The analysis model's NumericOperator on quire-contract-ir main is Add, Subtract, Multiply, Divide and Remainder only (expression.rs:987-993), so no exponentiation or modulo analysis expression can be built and that part of the AC cannot run. Fix: drop exponentiation and modulo from FR-010 (modulo stays in FR-012, where quire.op.integer.mod exists), or name the construct they map to. | spec/functional/FR-010-linear-integer-arithmetic.md:72 |
| FND-002 | medium | FR-012-AC-4 checks 'the stated expression kind and operator' for each integer operation, but FR-012 states neither. It gives no Terms table like FR-009's and names only quire.op.integer.lt. Fix: add rows mapping quire.op.integer.negate, add, sub, mul, eq, ne, lt, le, gt and ge to ExpressionKind::NumericNegate, Numeric with NumericOperator::Add, Subtract or Multiply, and Compare with the six ComparisonOperator values. These ids exist in the checked operation catalog on main. | spec/functional/FR-012-integer-claim-translation.md:52 |
| FND-003 | medium | FR-010-AC-5 says '2 * 3 * x' 'lowers as linear' but not to what bytes. FR-010 says '*' takes 'a literal factor', yet a constant term may be a product of literals. Folding to (* 6 x) and nesting as (* (* 2 3) x) both fit the text, the golden of FR-010-AC-2 needs one, and strict QF_LIA allows only a numeral coefficient. Fix: state that constant terms fold to one numeral before emission, with the emitted form in the AC. | spec/functional/FR-010-linear-integer-arithmetic.md:73 |
| FND-004 | low | The PR adds hand-written status rows ('🚧 Specified') to spec/test-matrix.md. The matrix and its coverage are computed by quire matrix / quoin matrix from trace tags, so a hand-kept status row is a tracking record that drifts. This convention was already in the repo. Fix: rely on the computed matrix, and raise removing the hand-kept rows as one repo decision, not per PR. | spec/test-matrix.md:37-42, 55-56 |

## Scope

| Unit | Role | Path |
| --- | --- | --- |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-1 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-2 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-3 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-4 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-5 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-6 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-7 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-8 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-010-AC-9 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-011 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-1 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-2 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-3 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-4 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-5 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-6 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-7 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-8 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-9 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-012 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-1 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-2 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-3 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-4 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-5 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-6 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-7 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-8 | examined | spec/functional/FR-012-integer-claim-translation.md |
| NFR-004 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| NFR-004-AC-1 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| NFR-004-AC-2 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| NFR-004-AC-3 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| NFR-004-AC-4 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| FR-009-AC-23 | examined | spec/functional/FR-009-package-runner.md |
| test-matrix | examined | spec/test-matrix.md |

## Verdict

FAIL at medium. Clean: id formats; TC frontmatter verifies relationships; real-solver criteria are marked skip-not-pass where solvers are absent; no hash, digest, pin or SHA tracking; no compat layer or fallback (the v2-replaces-v1 wording forbids one); no paywalled standard text; no roadmap or research content in this public repo.
