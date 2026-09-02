# The shared assurance lane

What this directory is, and — more usefully — what it is not.

## What it is

Two declarations and nothing executable.

- `change-assurance.json` is what this repository *states* about the change under
  issue #25: its subject, the requirements and preservation constraints it claims
  to hold, the proof obligations it offers, and the things it does not know. It
  is projected into the record body Quoin's FR-063 schema requires, and the only
  values derived at seal time are the digests its own `derived_fields` list
  names.
- `pins.json` records the upstream artifacts this lane reads, by digest, so a
  change upstream is a visible failure here rather than a silent change of
  meaning.

## What it is not

**It is not an evidence envelope.** Quoin's packaged
`change-assurance-record-v1`, `proof-attestation-v1` and
`verification-receipt-v1` schemas are the shapes, obtained from
`quoin change-assurance schema`. This repository ships no evidence schema of its
own and defines no envelope, manifest, identity framework, retention store, audit
store or aggregate verdict.

**It is not a second acceptance authority.** `pins.json` deliberately does not
restate component versions. The packaged Engineering Assurance compatibility
matrix decides which versions are accepted; `scripts/check_shared_pins.py`
observes what is installed and delegates every verdict to
`engineering_assurance.compatibility`. A local copy of those numbers is a copy
that can drift from the one the decision was made against.

**It is not a producer, and nothing here runs one.** `make assurance-inputs` is
the only target that executes anything. Quire exports and does not execute;
Quoin transcribes and does not execute. That is asserted behaviourally in
`tests/shared_assurance.rs`, with producers replaced by logging stubs and a
control that stubs `quoin` — because an empty log and an unconsulted `PATH` are
otherwise the same observation.

## Running it

```bash
make assurance-env      # build .venv-assurance from requirements-assurance.txt
make assurance-inputs   # the only target that runs a producer
make pins               # classify the installed toolchain upstream
make compat-view        # read retained evidence, then run the mutation probes
make assurance-chain    # seal, intake, receipt — over bytes already produced
make assurance          # all of the above
```

The Python here runs in `.venv-assurance` and nowhere else.
`engineering-assurance` declares `jsonschema>=4.23,<5`; a Draft 7 interpreter
imports it and appears to work, because the code paths needing a 4.x validator
are exactly the ones a refusing record never reaches. Two jobs, two
environments.

## Two known gaps, reported rather than papered over

- The pinned `engineering-assurance` v0.2.0 records
  `accepted.state = pending_human_acceptance` and ships no
  `human_acceptance_recorded` predicate. The human acceptance is recorded on that
  repository's `main` at `ae50e13`, after the tag, and no v0.2.1 exists. This
  lane reports the state the pinned release records and gates only on version
  compatibility. `agent-ix/engineering-assurance#20`.
- `map_pgm01_bytes` reads `quire.pgm01-evidence` v1 and v2. This repository's
  eight retained records are Markdown validation summaries, so the mapping
  answers `unreadable` for every one. That refusal is the reported compatibility
  result. `agent-ix/engineering-assurance#21`.
