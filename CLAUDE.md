# quire-analyze

SMT-backed consistency and implication analysis for versioned requirement contracts.

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
make pins               # classify the installed toolchain against the upstream matrix
make assurance-chain    # quoin seal / intake / receipt over already-produced bytes
make assurance          # pins + assurance-chain
```

Everything downstream of `assurance-inputs` consumes files and refuses to create
them. Quire exports and never executes a producer; Quoin transcribes and never
executes one. See `assurance/README.md`.

The Python here runs in `.venv-assurance` and nowhere else: `engineering-assurance`
declares `jsonschema>=4.23,<5`, and a Draft 7 interpreter imports it and appears
to work because the paths needing a 4.x validator are the ones a refusing record
never reaches.

## Safety scaffolding

Backported from `agent-ix/ecaz`:

- `clippy.toml` pins MSRV to `1.75` and caps cognitive complexity / arg count
- `deny.toml` allow-lists licenses and denies unknown registries/git sources
- `scripts/check_unsafe_comments.sh` runs in CI and locally via `make audit-unsafe`. Every `unsafe {` block must have a `// SAFETY:` comment within the 3 preceding lines, or be listed in `scripts/unsafe_comment_baseline.txt`. Update the baseline with `bash scripts/check_unsafe_comments.sh --update-baseline`.
- `rustfmt.toml` uses 100-char width and `StdExternalCrate` import grouping. CI fails on drift.
- `rust-toolchain.toml` pins to stable + rustfmt + clippy.

## Layout

```
src/lib.rs             # crate root
tests/integration.rs   # end-to-end tests
benches/               # criterion benchmarks (opt-in; add criterion to dev-deps)
spec/                  # requirements artifacts (from /spec-create-spec)
scripts/               # local tooling
```
