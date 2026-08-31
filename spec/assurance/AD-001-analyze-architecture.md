---
id: AD-001
title: "Contract analysis architecture"
type: ArchitectureDescription
status: proposed
owner: analyze-maintainers
system: quire-analyze v0.1
relationships:
  - target: ix://agent-ix/quire-analyze/AP-001
    type: realizes
---
# Contract analysis architecture

## System Boundary

The component owns request validation, semantic modeling, exact lowering, bounded adapter execution,
response normalization, conclusion checking, source mapping, and evidence emission. Contract IR,
solver implementations, operating-system process behavior, and release authority remain external.

## Views

The derivation view is: pinned validated IR → analysis model → canonical query bundle → bounded
engine adapter → normalized solver record → checked conclusion → immutable evidence report.

The identity view binds package/schema/corpus, selected clauses and revisions, shared variables,
encoding profile, query/assertion map, engine executable/version/configuration, raw response,
normalized result, producer, and outputs.

The failure view routes every invalid, unsupported, unknown, timed-out, cancelled, unavailable,
malformed, truncated, contradictory, undecodable, discrepant, or internal state to a non-conclusive
variant. Adapters cannot construct conclusions directly.

## Decisions

- One engine-neutral semantic model and query bundle prevent adapters from reinterpreting clauses.
- Complete semantic identity, not display name, controls variable sharing and source mapping.
- Canonical SMT-LIB2 and explicit assertion names make derivation reviewable.
- External engines run as bounded child processes with argv, never a shell command.
- Raw records are immutable inputs to normalization and differential comparison.
- Exact encoding is required for conclusive v0.1 results; approximation is fail-closed.

## Interfaces and Trust

Contract-IR validation is trusted only at the exact recorded revision and schema digest. Solver
answers are evidence, not self-authenticating truth: finite-model and cross-engine campaigns reduce
risk but do not qualify engines. The OS process API, filesystem atomicity, hashing library, and
schema validator remain named external dependencies.

## Risks

Encoding drift, solver defects, platform process differences, unbounded output, identity collisions,
model-decoding errors, and unavailable cross-platform execution can weaken evidence. The measurement
plan retains these outcomes and no automated control may waive them into success.
