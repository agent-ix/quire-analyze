---
id: REV-021
title: "Differential evidence and CLI quality assurance assessment"
type: Review
---
# Differential evidence and CLI quality assurance assessment

## Assessment

The implemented issue #5 slice is well tested for its declared library-first Linux boundary, but it
is not fully tested in the absolute sense. The default local suite has 43 passing tests and one
explicitly ignored real-engine test. That test was separately executed against verified official
release assets and passed both SAT/model-replay and UNSAT/model-unavailable cases. LLVM line coverage
is 91.00% overall, 88.96% for `src/report.rs`, 90.47% for `src/solver.rs`, and 74.07% for the thin
`src/main.rs` process wrapper. The enforced project floor is 90% overall.

## Covered Risk Classes

- all four differential dispositions, both conclusive statuses, verified-model agreement, retained
  disagreement records, missing-engine unavailability, and incomplete-model inconclusiveness;
- official asset archive digests, extracted executable digests, complete version output, exact
  configuration identity, SAT model differences between Z3/cvc5, and Z3 non-sat exit behavior;
- strict schema application in production, exact field census, canonical bytes, report/query/raw
  stream digests, outcome/status consistency, engine order, and contextual reconstruction;
- resealed mutations of unknown fields, raw output, query bytes, engine order, status, disposition,
  plus missing fields and noncanonical JSON;
- CLI subprocess behavior for byte parity, empty stdout, stderr diagnostics, satisfied/refuted exit
  classes, invalid input, existing destination refusal, unchanged destination bytes, and no owned
  staging residue;
- local fmt, warnings-as-errors lint, stable and Rust 1.75 tests, rustdoc, dependency advisories,
  bans/licenses/sources, unsafe audit, specification validation, coverage floor, and evidence census.

## Residual Test Gaps

- deterministic create/write/sync/rename/crash publication injection and competing publishers (#23);
- complete retained positive/negative/unsupported/timeout/disagreement corpus through a future
  trusted direct-analysis CLI (#24);
- shared PGM-01 envelope/integrity verification (`quire-contract-ir#20`);
- generated/fuzz/model mutation campaigns (#22), process stress (#21), and non-Linux execution (#20).

## QA Verdict

PASS for the current application report/differential/publisher slice. NOT FULLY TESTED as a complete
end-to-end, cross-platform, PGM-authoritative analyzer; every identified residual has an owning
ticket and no unavailable lane is represented as success.
