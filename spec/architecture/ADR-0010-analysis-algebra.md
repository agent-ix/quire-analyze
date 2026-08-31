---
id: ADR-0010
title: "Contract analysis algebra, variable identity, and encoding invalidation"
type: ADR
---
# ADR-0010: Contract analysis algebra, variable identity, and encoding invalidation

**Status:** Accepted for v0.1 implementation

**Decision date:** 2026-08-31

**Decision authority:** architecture prepared under issue #6; source-release sufficiency remains with
the PGM-01 human owner and is not decided here.

## Context

The original ADR-0010 in `quire-rs` decided process-based SMT-LIB2 and separation from LLM
formalization, but left Q2—IR shape, cross-requirement shared-variable identity, and encoding
versioning—to spike `agent-ix/quire-rs#164`. That issue is closed and explicitly superseded by
`agent-ix/quire-analyze#6`.

The premise changed materially: accepted `quire-contract-ir` now supplies a typed expression tree,
closed value types, definedness obligations, complete requirement/dependency identities, execution
points, canonical semantic bytes, and clause digests. Recreating that expression algebra here would
create two semantic authorities. Emitting solver strings directly from an upstream formalizer would
bypass the accepted validation and identity boundary.

The accepted conformance corpus has 99 fixtures, including 66 expression fixtures, but all expression
fixtures use the single owner `agent-ix/conformance:REQ_alpha@1`. It proves construct behavior, not
cross-requirement alias precision. The retained TC-009 adversarial fixture therefore measures the
identity alternatives directly and discloses that limitation.

## Decision

### 1. Reuse the contract IR and add only an analysis algebra

`quire.analysis-model/v1` is a minimal engine-neutral algebra over already validated contract-IR
clauses. It contains:

- one of the five closed analysis kinds from FR-001;
- explicit assumption, left, right, antecedent, consequent, peer, candidate, or case groups as the
  selected kind requires;
- full clause references plus their accepted canonical clause digests;
- one execution point per statement instance;
- explicit variable binding groups;
- finite semantic and execution limits; and
- the exact contract schema, canonicalization profile, analysis-model profile, and backend-encoding
  profile identities.

It does not copy the contract expression tree into a second guard/obligation AST. Backend lowering
walks the authoritative typed expression under this analysis context. It does not accept raw
SMT-LIB2 as a statement representation.

`quire.smtlib2/v1` is the first backend encoding-profile identity. Issue #7 owns its exact lowering
and capability table. A future backend can consume the same analysis model without changing its
selection, binding, or source identities.

### 2. Cross-requirement sharing is explicit and fail closed

Within one statement, the upstream `DependencyIdentity` remains authoritative. Across requirements,
the default is no alias: every complete dependency identity is a singleton. Equal display names do
not share.

Across clauses within one requirement, the default analysis-variable identity is the complete input
or state `DependencyIdentity` plus exact execution point and type-shape digest. Thus identical root
declarations at one point share, while the same `current` spelling at different points does not.
Field access, enum variants, and pure functions are derived expression dependencies, not separately
bindable variables.

Cross-requirement sharing occurs only through an explicit binding group supplied with the analysis
request. Every member names a root input or state and contains its full package and
requirement/revision, dependency kind and one-element declaration path,
state observation, execution point, and analysis type-shape digest. Validation requires:

1. one package and schema/canonicalization profile;
2. only input or state kinds, with identical type-shape digests, kinds, observations, and complete
   execution points;
3. no member in more than one group;
4. unique group identifiers and members; and
5. every member resolves to the selected validated statements.

Any mismatch rejects the request before lowering. The group identifier is a label inside one
canonical binding set, not a global ontology. A lexicon, matching display name, or model-generated
entity guess may propose a group to a human workflow, but cannot create one inside the analyzer.

The type-shape digest is an analysis-specific domain-separated SHA-256 over a source-free, owner-free
structural projection of the fully resolved value type. It includes integer signedness, bounds, and
overflow; rational bounds; option and collection structure and bounds; and named enum or record names
plus their complete sorted variant or field closure. Named-type recursion is already rejected
upstream. Owner is excluded so structurally identical declarations in different requirements can be
explicitly bound; names remain included so a coincidentally isomorphic domain type cannot pass
compatibility silently.

V0.1 binding requires exact execution-point equality. Relationships between distinct points—even
`pre:op` and `post:op`—remain unsupported until a separately specified analysis-frame mapping can
state their temporal equivalence. This sacrifices coverage rather than guessing temporal aliasing.

TC-009 measured `(true-positive, false-positive, false-negative)` pairs as follows:

| Candidate | Result |
|---|---|
| Complete upstream identity only | `(0, 0, 2)` |
| Display name only | `(2, 12, 0)` |
| Name plus kind, observation, type, and execution point | `(2, 3, 0)` |
| Explicit reviewed binding groups | `(2, 0, 0)` |

The explicit rule is selected because the cost of a false positive is unsound cross-requirement
interaction, while the cost of a missing group is a visible incomplete-coverage limitation.
The explicit row measures faithful application of reviewed fixture labels, not an inference
algorithm: correctness still depends on the workflow and human review that author those labels.

### 3. Statement, binding, request, and query identities are separate

The source statement identity contains the complete clause reference (including requirement
revision), `quire.contract.canonical-json/v1`, and the accepted clause canonical digest. The
provenance record also retains the source document identity/revision. The historical `quire-rs`
natural-language `statement_hash` may be retained as provenance, but it is not an analysis identity
because it is outside the accepted contract IR.

The v1 identities are domain-separated SHA-256 values over canonical length-delimited fields:

```text
binding-set digest = H("quire-analyze", "bindings", analysis-model profile,
                       sorted complete binding groups)

analysis-statement digest = H("quire-analyze", "statement", analysis-model profile,
                              source statement identity, execution point,
                              type-shape digests, binding-set digest)

analysis-request digest = H("quire-analyze", "request", analysis-model profile,
                            analysis kind, canonical statement groups, semantic limits,
                            binding-set digest)

query digest = H("quire-analyze", "query", encoding-profile identity,
                 analysis-request digest, exact query bytes)
```

`H` is SHA-256 with zero-byte-separated domain fields and unsigned 64-bit big-endian
length-delimited variable fields; raw concatenation is forbidden. Complete canonical structures,
not display strings or filesystem paths, supply the variable fields.

Changing a requirement revision, clause canonical digest, execution point, binding membership,
analysis kind/grouping, semantic limit, analysis-model profile, encoding-profile identity, or exact
query bytes changes the applicable downstream identity. Any semantic encoding change requires a new
encoding-profile identity even if a sample query happens to remain byte-identical. Evidence whose
recorded profiles or recomputed identities differ is stale and must be rejected, never migrated by guess.

### 4. Keep the prior integration and formalization boundaries

Z3 and cvc5 remain external SMT-LIB2 processes. No solver library is linked. LLM or human
formalization is outside this crate and produces contract IR plus proposed explicit bindings; it
never emits a verdict. Disagreement among formalizations is a requirement-quality finding rather
than a retry-to-consensus loop. Choosing a sample count remains outside this deterministic component.

## Rejected Alternatives

- **A second typed guard/obligation expression algebra:** duplicates accepted IR semantics and makes
  drift likely. The selected analysis algebra adds only cross-statement structure.
- **Direct SMT-LIB2 from a formalizer:** fastest prototype, but it bypasses validation, capability,
  provenance, and backend-independent review boundaries.
- **Complete upstream identity as the only rule:** safe but makes every cross-requirement interaction
  invisible because upstream dependencies are requirement-scoped.
- **Name or typed-name inference:** the measured fixture retains false aliases even after structural
  fields are added; domain meaning cannot be inferred from spelling.
- **Lexicon-driven automatic aliasing:** useful for proposals, not authority. A vocabulary entry does
  not prove two typed observations at two execution points denote the same state.

## Consequences

- Issue #7 implements the analysis-model validator, canonical identity encoding, capability table,
  type-shape projection, and exact `quire.smtlib2/v1` lowering before any adapter executes.
- Every report carries binding coverage: selected dependencies, explicitly shared dependencies, and
  unbound cross-requirement name candidates. Missing bindings weaken scope and remain visible.
- A human or upstream workflow owns proposed bindings; the analyzer owns structural validation and
  deterministic use of accepted groups.
- Profile changes intentionally invalidate cached queries and evidence. There is no silent v1
  migration.
- TC-009 is architecture evidence. It does not mark any solver or semantic matrix row complete.
- Cross-execution-point relations require a future reviewed profile; v0.1 reports them unsupported.
