#!/usr/bin/env python3
"""Read retained evidence through the pinned Engineering Assurance mapping.

This is the read-only compatibility view. It is the only place in this
repository permitted to call `map_pgm01_bytes`, it opens every retained file
read-only, and it writes nothing back — the retained bytes under `evidence/` are
an immutable record and this script's job is to report what the upstream mapping
makes of them, not to improve the answer.

The answer for this repository is a refusal, and that is the honest result.
`map_pgm01_bytes` reads `quire.pgm01-evidence` v1 and v2. What `evidence/` holds
is Markdown validation summaries: narrative records, not JSON envelopes of any
version. The count is read from the census below rather than restated here. The mapping therefore answers `unreadable` for every one. No local
mapper was written to turn that into a pass, because a mapping invented here
would be this repository deciding what upstream evidence means. Tracked as
`agent-ix/engineering-assurance#21`, which records 142 such envelopes across six
of the campaign's eight repositories.

`unreadable` is not `incompatible` and neither is `failed`. A record the mapping
could not parse, a record whose schema version it does not recognise, and a
record it read and found wanting are three different findings, and this script
reports whichever one it got.

The fixture corpus is the other half. Every negative case is **one named edit to
the pinned release's own bytes**, re-derived here at run time from the installed
`engineering_assurance` fixtures: if the committed fixture does not equal the
declared derivation, that is a failure and not a fixture. Each negative is paired
with a positive control observed to be accepted, because a refusal never seen to
accept anything is indistinguishable from a validator that refuses everything.

    <ASSURANCE_PYTHON> scripts/legacy_evidence_view.py --json
    <ASSURANCE_PYTHON> scripts/legacy_evidence_view.py --mutation-probes

Exit status: 0 when every case matched, 1 on a mismatch, 2 on a usage or
environment error.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_ROOT = ROOT / "evidence"
EVIDENCE_MANIFEST = EVIDENCE_ROOT / "manifest.sha256"
FIXTURE_ROOT = ROOT / "tests" / "fixtures" / "legacy-compat"
EXPECTATIONS = FIXTURE_ROOT / "expectations.json"

VIEW_SCHEMA = "quire-analyze.legacy-compatibility-view/v1"


class ViewError(RuntimeError):
    """The environment cannot answer the question. Distinct from a mismatch."""


def load_mapper() -> Callable[..., dict[str, Any]]:
    """Bind the pinned mapping, or fail loudly.

    A missing assurance distribution is an error and never a skip. A skipped
    compatibility check and a passing one look identical in a summary, and only
    one of them is true.
    """
    try:
        from engineering_assurance.verification_semantics import map_pgm01_bytes
    except ImportError as error:  # pragma: no cover - exercised by a mutation probe
        raise ViewError(
            f"the pinned assurance distribution is unusable: {error}. "
            "Run `make assurance-env`. This is an error and not a skip."
        ) from error
    return map_pgm01_bytes


def release_fixture(name: str) -> bytes:
    """Read one pinned release fixture, checking it is the byte string pinned."""
    from engineering_assurance import PACKAGE_ROOT

    path = PACKAGE_ROOT / "fixtures" / "verification-semantics" / name
    if not path.is_file():
        raise ViewError(f"the pinned release does not ship {name}")
    raw = path.read_bytes()
    pins = json.loads((ROOT / "assurance" / "pins.json").read_text(encoding="utf-8"))
    expected = {
        artifact["path"]: artifact["sha256"] for artifact in pins["consumed_artifacts"]
    }
    key = f"engineering_assurance/fixtures/verification-semantics/{name}"
    actual = hashlib.sha256(raw).hexdigest()
    if expected.get(key) != actual:
        raise ViewError(
            f"{key} is {actual} and assurance/pins.json records {expected.get(key)}; "
            "the pinned release changed underneath this repository"
        )
    return raw


# --------------------------------------------------------------------------
# Derivations
#
# Each is one named edit to pinned release bytes. They are deliberately small and
# deliberately different from one another: a corpus of negatives that all fail
# the same way measures one check repeatedly.
# --------------------------------------------------------------------------


def derive(name: str, source: bytes) -> bytes:
    decoded = json.loads(source)
    if name == "unsupported-schema":
        # A version the mapping has never seen. Must be `incompatible`, which is
        # not the same as unreadable: the bytes parsed fine.
        decoded["schemaVersion"] = "quire.pgm01-evidence/v99"
    elif name == "malformed-missing-checks":
        # A recognised version whose required structure is gone. Must be
        # `unreadable` — read as far as the contract and refused there, which is
        # a different finding from a version the mapping does not know.
        if "checks" not in decoded:
            raise ViewError("the pinned v1 fixture no longer has a `checks` field to remove")
        decoded.pop("checks")
    elif name == "stale-disposition":
        # A record upstream itself marks retracted. Must stay visible as stale
        # rather than disappearing or being read as a current pass.
        decoded["historicalDisposition"] = "retracted"
    elif name == "unreadable-not-json":
        # Not JSON at all. Truncation is the commonest real corruption.
        return source[: len(source) // 3]
    elif name == "unreadable-not-an-object":
        return b"[]"
    else:
        raise ViewError(f"unknown derivation {name}")
    return json.dumps(decoded, indent=2, sort_keys=True).encode("utf-8") + b"\n"


DERIVATIONS = {
    "derived-unsupported-schema.json": ("pgm01-v1.json", "unsupported-schema"),
    "derived-malformed.json": ("pgm01-v1.json", "malformed-missing-checks"),
    "derived-stale.json": ("pgm01-v2.json", "stale-disposition"),
    "derived-unreadable-truncated.json": ("pgm01-v1.json", "unreadable-not-json"),
    "derived-unreadable-not-an-object.json": ("pgm01-v1.json", "unreadable-not-an-object"),
}


def rederive_all() -> dict[str, bytes]:
    return {
        name: derive(edit, release_fixture(source))
        for name, (source, edit) in DERIVATIONS.items()
    }


def write_fixtures() -> None:
    """Regenerate the committed fixtures from the pinned release bytes."""
    FIXTURE_ROOT.mkdir(parents=True, exist_ok=True)
    for name, raw in rederive_all().items():
        (FIXTURE_ROOT / name).write_bytes(raw)


# --------------------------------------------------------------------------
# The view
# --------------------------------------------------------------------------


def retained_files() -> list[Path]:
    if not EVIDENCE_ROOT.is_dir():
        raise ViewError("evidence/ is absent; the retained record is what is being read")
    return sorted(
        path
        for path in EVIDENCE_ROOT.rglob("*")
        if path.is_file() and path != EVIDENCE_MANIFEST
    )


def manifest_digests() -> dict[str, str]:
    """The digests the repository already committed for its retained records.

    Binding these as `expected_digest` is what makes an altered retained byte
    read `incompatible` with a tampered-source reason instead of being mapped as
    though nothing had happened.
    """
    if not EVIDENCE_MANIFEST.is_file():
        raise ViewError("evidence/manifest.sha256 is absent")
    digests: dict[str, str] = {}
    for line in EVIDENCE_MANIFEST.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        expected, relative = line.split("  ", 1)
        digests[relative.strip()] = expected.strip()
    return digests


def view_retained(mapper: Callable[..., dict[str, Any]]) -> dict[str, Any]:
    digests = manifest_digests()
    entries = []
    outcomes: dict[str, int] = {}
    census_mismatches = []
    for path in retained_files():
        relative = path.relative_to(ROOT).as_posix()
        raw = path.read_bytes()
        actual = hashlib.sha256(raw).hexdigest()
        declared = digests.get(relative)
        if declared is None:
            census_mismatches.append(f"{relative} is retained but not declared in the manifest")
        elif declared != actual:
            census_mismatches.append(
                f"{relative} is {actual} and the manifest declares {declared}"
            )
        view = mapper(raw, expected_digest=declared) if declared else mapper(raw)
        outcomes[view["outcome"]] = outcomes.get(view["outcome"], 0) + 1
        entries.append(
            {
                "path": relative,
                "sha256": actual,
                "manifestDigest": declared,
                "outcome": view["outcome"],
                "sourceDigest": view["source_digest"],
                "sourceDigestMatches": view["source_digest"] == actual,
            }
        )
    for declared_path in digests:
        if not (ROOT / declared_path).is_file():
            census_mismatches.append(f"{declared_path} is declared but not retained")
    return {
        "retainedFiles": len(entries),
        "outcomes": dict(sorted(outcomes.items())),
        "censusMismatches": census_mismatches,
        "entries": entries,
    }


def evidence_states(view: dict[str, Any]) -> set[str]:
    """The evidence states the mapping assigned, from its own `mappings`.

    Restricted to the `evidence` concept on purpose. A legacy record also carries
    per-command `check_result` states, and counting those as states this view
    demonstrated would let it claim `passed` and `failed` coverage it did not
    establish.

    This is how the stale case discriminates at all: a retracted record stays
    readable, so its `outcome` is identical to its control's, and only the mapped
    `/historicalDisposition -> evidence.state = "stale"` tells them apart.
    """
    return {
        str(mapping.get("value"))
        for mapping in view.get("mappings", [])
        if mapping.get("target_concept") == "evidence"
        and mapping.get("target_field") == "state"
    }


def view_fixtures(mapper: Callable[..., dict[str, Any]]) -> dict[str, Any]:
    expectations = json.loads(EXPECTATIONS.read_text(encoding="utf-8"))
    derived = rederive_all()
    cases = []
    mismatches = []
    demonstrated: set[str] = set()

    for case in expectations["cases"]:
        name = case["fixture"]
        if case["kind"] == "release-control":
            raw = release_fixture(name)
            drift = None
        else:
            path = FIXTURE_ROOT / name
            if not path.is_file():
                mismatches.append(f"{name}: fixture is absent; run `make compat-fixtures`")
                continue
            raw = path.read_bytes()
            expected_bytes = derived.get(name)
            # The committed fixture must BE the declared derivation. A fixture
            # that has drifted from its stated edit is a hand-written blob
            # wearing a derivation's name.
            drift = None if raw == expected_bytes else "committed bytes are not the declared derivation"
            if drift:
                mismatches.append(f"{name}: {drift}")

        expected_digest = case.get("expected_digest")
        if expected_digest == "self":
            expected_digest = hashlib.sha256(raw).hexdigest()
        view = mapper(raw, expected_digest=expected_digest)
        observed = view["outcome"]
        matched = observed == case["outcome"]
        if not matched:
            mismatches.append(
                f"{name}: expected outcome {case['outcome']} and observed {observed}"
            )
        # Only a case that ran AND matched may claim to demonstrate its state. A
        # scenario that demonstrated nothing carries null rather than borrowing
        # the label it was aiming at.
        if matched:
            demonstrated.add(case["outcome"])
            # A case may also demonstrate a mapped state, which is how `stale`
            # is shown: its outcome is identical to its control's, so an outcome
            # alone would prove the fixture changed nothing.
            demonstrated.update(evidence_states(view))
        cases.append(
            {
                "fixture": name,
                "kind": case["kind"],
                "pairsWith": case.get("pairs_with"),
                "expectedOutcome": case["outcome"],
                "observedOutcome": observed,
                "matched": matched,
                "demonstrates": case["outcome"] if matched else None,
                "sourceRecordId": view.get("source_record_id"),
                "sourceSchemaVersion": view.get("source_schema_version"),
                # The mapper surfaces staleness in `mappings`, not in `outcome`:
                # a retracted record is still readable. Recording the mapped
                # concept is what lets the stale case differ from its control,
                # which by outcome alone it does not.
                "mappedStates": sorted(evidence_states(view)),
                "derivationDrift": drift,
            }
        )

    # A control that names a scenario which does not exist pairs with nothing,
    # and an assertion over an empty pairing is vacuously true. Refuse it.
    names = {case["fixture"] for case in expectations["cases"]}
    for case in expectations["cases"]:
        partner = case.get("pairs_with")
        if partner is not None and partner not in names:
            mismatches.append(
                f"{case['fixture']}: pairs_with names {partner}, which is not a case"
            )

    return {
        "cases": cases,
        "casesMatched": sum(1 for case in cases if case["matched"]),
        "casesTotal": len(cases),
        "statesDemonstrated": sorted(demonstrated),
        "mismatches": mismatches,
    }


def build_report() -> dict[str, Any]:
    mapper = load_mapper()
    retained = view_retained(mapper)
    fixtures = view_fixtures(mapper)
    mismatches = retained["censusMismatches"] + fixtures["mismatches"]
    return {
        "schema": VIEW_SCHEMA,
        "mapping": "engineering_assurance.verification_semantics.map_pgm01_bytes",
        "mappingCoverage": "quire.pgm01-evidence/v1 and /v2 only",
        "retainedRecordFormat": (
            "Markdown validation summaries. Not PGM-01 envelopes of any version, so the "
            "pinned mapping refuses them. Reported as the compatibility result; see "
            "agent-ix/engineering-assurance#21."
        ),
        "outcome": "passed" if not mismatches else "failed",
        "retained": retained,
        "fixtures": fixtures,
        "mismatches": mismatches,
    }


# --------------------------------------------------------------------------
# Mutation probes
#
# No exception handling anywhere in here. A probe that crashes is a broken probe,
# and counting it as a detection is how a probe table inflates itself.
# --------------------------------------------------------------------------


def mutation_probes() -> list[dict[str, Any]]:
    mapper = load_mapper()
    v1 = release_fixture("pgm01-v1.json")
    probes = []

    # The positive control must be accepted, or every refusal below is meaningless.
    probes.append(
        {
            "probe": "pinned-v1-release-fixture-is-readable",
            "expects": "the control is mapped, not refused",
            "detected": mapper(v1)["outcome"] not in {"unreadable", "incompatible"},
        }
    )

    # Binding a wrong expected digest must be caught as a tampered source.
    probes.append(
        {
            "probe": "wrong-expected-digest-reads-tampered",
            "expects": "incompatible with a tampered-source reason",
            "detected": mapper(v1, expected_digest="0" * 64)["outcome"] == "incompatible",
        }
    )

    # A single altered byte, checked against the digest the record was retained
    # under, must read as a tampered source. The earlier form of this probe
    # compared two source digests of two different inputs — which asserts that
    # SHA-256 is a hash, not that the mapping binds an identity to anything.
    altered = bytearray(v1)
    altered[-2] = altered[-2] ^ 0x20
    intact_digest = hashlib.sha256(v1).hexdigest()
    probes.append(
        {
            "probe": "single-altered-byte-against-its-retained-digest-is-tampered",
            "expects": "incompatible, with no field of the altered source interpreted",
            "detected": mapper(bytes(altered), expected_digest=intact_digest)["outcome"]
            == "incompatible",
        }
    )

    # The derivation check must notice a fixture that no longer matches its edit.
    derived = rederive_all()
    name = "derived-unsupported-schema.json"
    committed = (FIXTURE_ROOT / name).read_bytes()
    probes.append(
        {
            "probe": "committed-fixture-equals-its-declared-derivation",
            "expects": "the committed bytes are exactly the re-derived bytes",
            "detected": committed == derived[name],
        }
    )

    # An unknown schema version must be `incompatible` and never silently empty.
    probes.append(
        {
            "probe": "unknown-schema-version-is-incompatible-not-empty",
            "expects": "incompatible",
            "detected": mapper(derived[name])["outcome"] == "incompatible",
        }
    )

    # This repository's own retained bytes must refuse, and refuse as unreadable.
    sample = retained_files()[0].read_bytes()
    probes.append(
        {
            "probe": "markdown-narrative-is-unreadable-not-passed",
            "expects": "unreadable",
            "detected": mapper(sample)["outcome"] == "unreadable",
        }
    )

    return probes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit the view as JSON")
    parser.add_argument(
        "--mutation-probes", action="store_true", help="run the probes and report each result"
    )
    parser.add_argument(
        "--write-fixtures",
        action="store_true",
        help="regenerate the committed fixtures from the pinned release bytes",
    )
    args = parser.parse_args(argv)

    try:
        if args.write_fixtures:
            write_fixtures()
            print(f"re-derived {len(DERIVATIONS)} fixtures into {FIXTURE_ROOT}")
            return 0

        if args.mutation_probes:
            probes = mutation_probes()
            for probe in probes:
                print(
                    f"{'detected' if probe['detected'] else 'MISSED  '}  "
                    f"{probe['probe']}: {probe['expects']}"
                )
            missed = [probe for probe in probes if not probe["detected"]]
            print(f"\n{len(probes) - len(missed)}/{len(probes)} probes detected")
            return 0 if not missed else 1

        report = build_report()
    except ViewError as error:
        print(f"legacy evidence view: {error}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(
            f"retained records: {report['retained']['retainedFiles']} "
            f"-> {report['retained']['outcomes']}"
        )
        print(
            f"fixtures: {report['fixtures']['casesMatched']}/"
            f"{report['fixtures']['casesTotal']} matched; "
            f"states demonstrated: {', '.join(report['fixtures']['statesDemonstrated'])}"
        )
        for mismatch in report["mismatches"]:
            print(f"  mismatch: {mismatch}")
        print(f"outcome: {report['outcome']}")

    return 0 if not report["mismatches"] else 1


if __name__ == "__main__":
    sys.exit(main())
