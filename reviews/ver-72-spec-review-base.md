---
id: SR-17027
title: "Base spec review of VER-72 package runner (FR-009, NFR-003, TC-015)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; quire-analyze#80 diff origin/main...HEAD: spec/functional/FR-009-package-runner.md, spec/nonfunctional/NFR-003-runner-invocation.md, spec/test/TC-015-package-runner.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-72. Base checklist review of the spec-only diff of quire-analyze#80 (head
d321187). `quire validate --scope . "spec/**/*.md"` exits 0 with two EARS warnings on
NFR-003 (quire 0.44.0). No hash, digest, pin, vendoring or roadmap content was added; the
QSL fixtures are referenced by checkout path, not copied. The blocking defect is that the
three fixture packages the ACs and TC-015 run over no longer exist on
`quire-spec-language` main, so FR-009-AC-1 and AC-2 cannot be executed as written. Several
criteria cannot fail, and the solver-configuration input and several error paths have no
criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The named fixtures `artifacts/compiled-protocol-v1`, `artifacts/compiled-protocol-v2` and `examples/protocol-handoff/` are absent from quire-spec-language main: `artifacts/` was removed by #1823 (handoffs are generated, not committed) and `examples/protocol-handoff/` by #2473 (QSL-840). FR-009-AC-1, AC-2 and TC-015 cannot run. Fix: name fixtures that exist on QSL main (for example `examples/checked-handoff/`) or say the test generates the handoff with QSL's own generator at test time. | FR-009:18-20, FR-009-AC-1, FR-009-AC-2, TC-015:20-23 |
| FND-002 | medium | `--solver-config` "when omitted the configured defaults of the library apply", but the library has no default solver executable: `SolverConfig::new` requires an executable path and only `AdapterLimits` has a `Default`. No AC covers the option, an unreadable or invalid config file, or a missing solver. Fix: make the option required, or define how the executables are resolved without it; add an AC that an invalid or unreadable config exits `2` with no report. | FR-009:25-26, src/solver.rs:86,204 |
| FND-003 | medium | FR-009-AC-2 says "the three exit statuses are asserted individually" without saying what each must equal, so it adds nothing AC-1 does not already cover and cannot fail on its own. Fix: state the expected exit class for each fixture under named stub solvers. | FR-009-AC-2 |
| FND-004 | medium | FR-009-AC-9 cannot fail in this repo: "the producer-execute definition above is stated in this spec" is self-referential, and "no Quire or Quoin code path spawns `run-package`" inspects other repositories. Fix: delete the AC, or reduce it to a check that `run-package --help` documents the arguments and exit classes. | FR-009-AC-9 |
| FND-005 | medium | Error paths have no criterion: an unknown or missing argument (exit `2`), a missing or unwritable `--output` parent directory and the FR-005 `published-durability-unknown` state (exit `4`). Fix: add criteria for each, or cite the FR-005 criteria that already cover them. | FR-009:31-34, FR-009-AC-4, FR-009-AC-5 |
| FND-006 | low | TC-015 runs a missing directory and an "empty package", but FR-009-AC-4 names three distinct cases: missing directory, unloadable package and a package declaring no request. One case is left out. Fix: list all three in the procedure. | TC-015:24-25, FR-009-AC-4 |
| FND-007 | low | NFR-003-AC-2 restates FR-009-AC-1 and AC-3, and NFR-003's hermetic statement and AC-3 restate the FR-009 producer-execute bullet and FR-009-AC-8. Fix: keep each obligation in one place. | NFR-003-AC-2, NFR-003-AC-3, FR-009-AC-8 |

## Verdict

FAIL: not mergeable until FND-001 is fixed. ID formats, sequential AC and CON numbering,
the index update and the TC-to-FR/NFR links are correct. FR-009-AC-5, AC-6 and AC-7 and
NFR-003-AC-1 are falsifiable and well targeted.
