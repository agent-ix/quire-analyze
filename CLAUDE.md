# quire-analyze

SMT-backed consistency and implication analysis for versioned requirement contracts.

## Hash / digest / pin antipattern: present, slated for removal

This repository still contains hashes, digests, SHAs, pins, checksum catalogs and
records that track files, versions or tools. That is an antipattern and it is slated
for removal. Do not introduce any new use of it. When you touch code or specs that
contain one, remove it as part of the change. Package versions
live in Cargo.toml / package.json and their lockfiles only; reports name the app
version they ran.

## Commands

```bash
make fmt            # format with rustfmt
make fmt-check      # verify formatting (CI gate)
make lint           # clippy with -D warnings
make test           # cargo test
make build          # release build
make clean          # cargo clean
make deny           # cargo deny check licenses
make ci             # every local gate
```

## Safety scaffolding

- `deny.toml` allow-lists licenses and denies unknown registries/git sources
- Workspace Clippy `undocumented_unsafe_blocks` checks safety comments in both crates.
- `rustfmt.toml` uses 100-char width and `StdExternalCrate` import grouping. CI fails on drift.

## Layout

```
src/lib.rs             # crate root
tests/integration.rs   # end-to-end tests
benches/               # criterion benchmarks (opt-in; add criterion to dev-deps)
spec/                  # requirements artifacts (from /spec-create-spec)
scripts/               # local tooling
```
