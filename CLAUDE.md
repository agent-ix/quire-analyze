# quire-analyze

SMT-backed consistency and implication analysis for versioned requirement contracts.

## Hash / digest / pin antipattern: present, slated for removal

This repository still contains hashes, digests, SHAs, pins, checksum catalogs and
records that track files, versions or tools. That is an antipattern and it is slated
for removal. Do not introduce any new use of it. When you touch code or specs that
contain one, remove it as part of the change. The only hash that stays is a canonical
identity digest that binds a proof to the exact content it proved. Package versions
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
make audit-unsafe   # check that every unsafe block has a // SAFETY: comment
make ci             # every local gate, including the shared assurance lane
```

## Shared assurance (issue #25)

```bash
make assurance-env      # build .venv-assurance from requirements-assurance.txt
make assurance-inputs   # THE ONLY TARGET THAT RUNS A PRODUCER
```

Everything downstream of `assurance-inputs` consumes files and refuses to create
them. Quire exports and never executes a producer; Quoin transcribes and never
executes one. See `assurance/README.md`.

The Python here runs in `.venv-assurance` and nowhere else: `engineering-assurance`
declares `jsonschema>=4.23,<5`, and a Draft 7 interpreter imports it and appears
to work because the paths needing a 4.x validator are the ones a refusing record
never reaches.

## Safety scaffolding

- `deny.toml` allow-lists licenses and denies unknown registries/git sources
- `scripts/check_unsafe_comments.sh` runs in CI and locally via `make audit-unsafe`. Every `unsafe {` block must have a `// SAFETY:` comment within the 3 preceding lines, or be listed in `scripts/unsafe_comment_baseline.txt`. Update the baseline with `bash scripts/check_unsafe_comments.sh --update-baseline`.
- `rustfmt.toml` uses 100-char width and `StdExternalCrate` import grouping. CI fails on drift.

## Layout

```
src/lib.rs             # crate root
tests/integration.rs   # end-to-end tests
benches/               # criterion benchmarks (opt-in; add criterion to dev-deps)
spec/                  # requirements artifacts (from /spec-create-spec)
scripts/               # local tooling
```
