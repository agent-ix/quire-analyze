---
id: Task-004
title: "Deterministic SMT-LIB2 lowering"
type: Task
status: done
track: B
priority: P0
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: references
---
# Task-004: Deterministic SMT-LIB2 lowering

## Scope

Complete native issue #7 with an exact capability census, canonical query bundle, injective symbols,
source assertion maps, unsupported fixtures, and reproducibility evidence.

## Guard

Task-003 must be done and ADR-0010 accepted.

## Verification

TC-010 owns the executable issue slice: order invariance, exact Boolean operator encodings, explicit
unsupported categories, binding validation, structural type-shape order, identity invalidation,
statement resource bounds, golden bytes, exact dependency pinning, and Rust 1.75 compatibility.
