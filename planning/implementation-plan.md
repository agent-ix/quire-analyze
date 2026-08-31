---
id: REV-003
title: "Contract analysis v0.1 implementation sequencing review"
type: Review
---
# Contract analysis v0.1 implementation sequencing review

The mechanically checkable bundle is `plan/PLAN-001-analyze-v01/plan.md`.

## Dependency DAG

```text
PGM-01 + accepted contract IR (#8, #10)
                  |
             foundation (#2)
                  |
        ADR/algebra/identity (#6)
                  |
       SMT-LIB2/capability (#7)
                  |
        Z3/cvc5 adapters (#3)
                  |
       analyses/counterexamples (#4)
                  |
       evidence/differential/CLI (#5)
                  |
        epic verification (#8)
                  |
       human release (PGM-02 Wave 4)
```

## Coordination Rules

Implementation children remain Backlog until issue #2 is Done. Each child begins with the accepted
dependency identities, adds requirement-tagged executable tests, retains failures and limitations,
and undergoes code review plus gap analysis. CI workflows remain manual-dispatch-only until the
operator later enables hosted CI. Local gates are required and do not imply hosted checks passed.
