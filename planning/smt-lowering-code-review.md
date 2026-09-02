---
id: REV-007
title: "Deterministic SMT lowering code review"
type: Review
---
# Deterministic SMT lowering code review

## Scope

Producer review of native issue #7: the Boolean v1 lowering API, authoritative contract-IR input
boundary, binding model, identities, source maps, capability contract, supply-chain policy, golden,
requirements, and tests. This is not independent approval or a release decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| SMT-F01 | critical | A caller-provided clause digest or source map would make the evidence boundary falsifiable. | Closed: `StatementInput::from_clause` accepts an authoritative typed package/requirement/clause, verifies membership and owner, rechecks the typed expression, and recomputes the canonical clause digest and source span. |
| SMT-F02 | high | Equality could accidentally admit non-Boolean operands, or unsupported expressions could be approximated. | Closed: every recursive operand is lowered through the exhaustive `ExpressionKind` match; non-Boolean references and all unmatched constructs return `UnsupportedConstruct`. Arithmetic, ordering, quantification, and text fixtures fail before a query is returned. |
| SMT-F03 | high | Display-name variable reuse or malformed explicit bindings could change the logical problem. | Closed: unbound symbols hash complete owner/kind/observation/name/point/type identity; bound symbols require sorted, unique, compatible, fully consumed groups. Duplicate groups, duplicate members, multi-group reuse, incompatibility, and unused members fail closed. |
| SMT-F04 | high | State observations could alias even when declaration names match. | Closed: observation and execution point are identity-bearing, and TC-010 now exercises distinct `current`/`pre` symbols and rejects a mixed-observation binding. |
| SMT-F05 | medium | Host collection order could affect declarations, assertions, maps, or digests. | Closed: ordered maps/sets and explicit sorting define binding, variable, statement, assertion, and structural named-type order. The retained golden is identical under reversed statement and declaration order. |
| SMT-F06 | medium | Resource limits could be documented without being bound into request identity or enforced before solver use. | Closed for the issue #7 boundary: statement/depth/node/query bounds are public, bound into the request digest, and checked before any solver adapter exists. Statement overflow has a runtime regression test; remaining boundary campaigns are tracked by issue #19. |
| SMT-F07 | medium | The dependency graph initially contained two RustSec findings. | Closed for this slice: `url`/`idna` were upgraded with a Rust-1.75-compatible adapter pin. RUSTSEC-2026-0009 affects `time`'s RFC 2822 parser, while `jsonschema 0.17.1` contains no `Rfc2822` reference and uses `time` only for date/RFC 3339 validation. The exact exception is documented in `deny.toml`; upstream cleanup/MSRV resolution remains tracked by `quire-contract-ir#37`. |
| SMT-F08 | medium | A partial local gate could pass while MSRV, specification, documentation, coverage, or evidence-integrity checks were absent or suppressible. | Closed: `make ci` has a parsed closed prerequisite census, rejects command overrides and ignore mode, and includes all cargo-deny lanes, Rust 1.75, rustdoc, Quire validation, 90% line coverage, and evidence checksum verification. Hosted CI remains manual-only. |

## Code Quality

The production module has no unsafe code and no solver/runtime dependency. Public hashes are
domain-separated and length-prefixed. SMT symbols are ASCII and injective for their encoded identity
inputs. The query declares SMT-LIB 2.6, model production, and `QF_UF` before declarations and
assertions. Clippy warnings-as-errors, formatting, default and Rust 1.75 tests, rustdoc, Quire
validation, cargo-deny, evidence checksums, and the coverage floor are local gates.

## Verdict

PASS for the issue #7 Boolean lowering slice. No unresolved correctness or soundness finding blocks
review. Broader solver execution, analysis conclusions, differential validation, evidence tooling,
and CLI behavior remain deliberately outside this issue and are owned by later native tasks.
