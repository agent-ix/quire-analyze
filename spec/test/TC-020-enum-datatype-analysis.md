---
id: TC-020
title: "Verify enum, record and option analysis"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-020
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-021
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-022
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-010
    type: verifies
---
# TC-020: Verify enum, record and option analysis

## Description

Verify that enum, record and option constraints lower to datatypes, are decided by Z3 and cvc5,
replay from typed models, translate from contract packages, and that every unsupported datatype
construct is refused without approximation.

## Test Procedure

Lower fixtures for enum membership (satisfiable and unsatisfiable), a record-field constraint, and
an option-access constraint under an `IsPresent` guard; run each on both solvers where they are
installed. Parse hand-written model text in the spelling of each solver for every datatype kind and
for invalid values (unknown constructor, wrong arity, wrong sort, abstract element, malformed
text), and replay them. Permute environment, variant, field and statement order. Exceed each
NFR-010 bound by one. Build contract-package fixtures at test time for an enum-membership claim, a
record-field claim, and each refused term, and run `run-package` as a child process.

## Expected Results

Supported fixtures are decided correctly, with identical queries under permutation. Invalid models
and unsupported or over-bound constructs produce explicit non-conclusive outcomes or exit `2`, never
an approximate query. Real-solver cases run where Z3 and cvc5 are installed and are reported as not
run elsewhere; the model-parsing, lowering and translation cases need no solver.

## Limitations

The text-only lowering, parsing and translation cases run on any host. The solver cases are not
claimed on a host without the solvers.
