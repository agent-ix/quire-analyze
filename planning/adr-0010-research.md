---
id: REV-004
title: "ADR-0010 reproduced spike and measurements"
type: Review
---
# ADR-0010 reproduced spike and measurements

## Sources and Limits

This report reproduces the unfinished Q2 work from closed `agent-ix/quire-rs#164` against accepted
contract IR. The historical ADR source was inspected at quire-rs revision
`a642c91c3560c022276b77a31ee54141b3a8f97a`. The real example below uses quire-rs FR-063 as observed
at source revision `ce852bb77cc3a30df56aba5ce60e49aa44449e34` with file SHA-256
`c929d1fa4638430d201c4f9e28f856eb803e26cfdecb1bb82382f7c5c2a308db`.

The accepted contract-IR corpus at `5c49ebfd1c87415f74420ad047392bd03b1bd202` contains 99 fixtures:
22 package, 66 expression, 8 coverage, and 3 migration fixtures. Its 66 expression fixtures expose
one unique requirement owner, `agent-ix/conformance:REQ_alpha@1`; therefore it cannot measure
cross-requirement aliases. That absence is a corpus limitation, not a zero-error result. TC-009 adds
an adversarial 11-reference research fixture without claiming it is production prevalence data.

## One Real Requirement Encoded Both Ways

FR-063 defines a hollow ratio when population and examined are positive while matched is zero. The
hand formalization uses non-negative bounded integers and the Boolean predicate below. This is a
research transcription of one published requirement, not a claim that quire-rs already emitted
contract IR.

### Contract-IR-backed analysis algebra

```yaml
research_owner: agent-ix/quire-rs:FR-063
historical_source_revision: ce852bb77cc3a30df56aba5ce60e49aa44449e34
execution_point: post:compute_metric
declarations:
  population: {kind: input, type: {integer: {minimum: 0, maximum: 9223372036854775807}}}
  examined:   {kind: input, type: {integer: {minimum: 0, maximum: 9223372036854775807}}}
  matched:    {kind: input, type: {integer: {minimum: 0, maximum: 9223372036854775807}}}
predicate:
  total_and:
    - greater(population, 0)
    - greater(examined, 0)
    - equal(matched, 0)
analysis_model:
  kind: consistency
  selected: [FR-063:hollow-ratio]
  bindings: []
  profile: quire.analysis-model/v1
```

The value types, observations, definedness, bounds, expression tree, owner, and source identity stay
in contract IR. The analysis layer adds selection and binding only.

### Direct SMT-LIB2

```smt2
(set-logic QF_LIA)
(declare-const population Int)
(declare-const examined Int)
(declare-const matched Int)
(assert (! (and (>= population 0) (<= population 9223372036854775807))
           :named domain.population))
(assert (! (and (>= examined 0) (<= examined 9223372036854775807))
           :named domain.examined))
(assert (! (and (>= matched 0) (<= matched 9223372036854775807))
           :named domain.matched))
(assert (! (and (> population 0) (> examined 0) (= matched 0))
           :named FR_063_hollow_ratio))
(check-sat)
(get-model)
```

The direct form is shorter, but its three names do not establish package, requirement revision,
declaration kind, observation, execution point, value type, or source span. Those facts would need a
parallel map whose semantics duplicate the typed input. Direct SMT-LIB2 remains the backend output,
not the input algebra.

## Shared-Variable Measurement

`tests/adr_0010.rs` exhaustively compares all 55 pairs in
`research/adr-0010-shared-variable-fixture.tsv`. Ground truth contains two intended shared pairs.
Results are `(true positive, false positive, false negative)`:

| Rule | Result | Precision | Recall | Cost of being wrong |
|---|---|---:|---:|---|
| Complete upstream identity | `(0, 0, 2)` | undefined, no predicted joins | 0% | Real conflicts are missed and coverage is incomplete. |
| Display name | `(2, 12, 0)` | 14.3% | 100% | Unrelated types, scopes, and domain entities interact; conclusions are unsound. |
| Name + kind + observation + type + execution point | `(2, 3, 0)` | 40% | 100% | Same-shaped domain entities still alias without authority. |
| Explicit reviewed binding group | `(2, 0, 0)` | 100% | 100% | Omitted groups miss interactions but remain measurable; incompatible groups reject. |

The fixture deliberately contains same-named ambient/motor/controller temperatures, a Boolean
temperature alarm, pre/post state observations, and two unrelated `active` inputs. It demonstrates
why additional structural fields reduce but cannot eliminate spurious sharing.
The explicit row measures deterministic application of the fixture's reviewed labels, not an
automated inference algorithm; its correctness premise is the upstream human-reviewed binding.

## Recommendation and Cost

Adopt the minimal analysis algebra over contract IR and explicit reviewed binding groups. Default
singletons trade false negatives for a visible binding-coverage limitation; this is safer than false
positive aliases that can manufacture contradictions or implications. The cost is authoring or
reviewing bindings and carrying a binding census in every report. Being wrong in the other direction
is worse: an implicit alias changes the logical problem while still looking conclusive.

Use separate version identities for the analysis model and SMT encoding, and bind both into
domain-separated statement/request/query digests. Contract-IR clause digest plus requirement revision
replaces the historical natural-language statement hash as the semantic identity; the historical
hash remains provenance only. This makes source, binding, request, and encoding changes invalidate
stale evidence at the appropriate layer.

## Implementation Tickets

The replacement tickets already exist and are correctly ordered: #7 implements the model validator,
identities, capability contract, and SMT lowering; #3 implements bounded processes; #4 implements
the five analyses and mapped models; #5 implements reports, differential conformance, and CLI. No
duplicate ticket is filed. Closed `quire-rs#164` already links issue #6 as its replacement.

## Verdict

PASS for accepting ADR-0010 as the binding v0.1 architecture decision. The measurements support the
identity choice and disclose their non-prevalence limitation. No semantic implementation, solver
result, or source-release claim is made.
