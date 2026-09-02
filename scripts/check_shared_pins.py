#!/usr/bin/env python3
"""Observe this machine's shared-assurance toolchain and report the verdict.

The split is the point, and it is not this module's invention: it is the shape
`engineering_assurance.compatibility` was written to enforce. This file observes
and never decides. Every version verdict is `classify_all`'s answer, transcribed
without restatement, because a matrix copied into a campaign repository is a
matrix that can drift from the one the decision was made against.

Three verdicts, and none of them collapses into another:

- `compatible`   — the exact pinned version.
- `incompatible` — a version the matrix names and rules out.
- `unknown`      — a version the matrix has never seen, or a tool that could not
                   be observed at all. Unknown is not a pass.

Human acceptance is reported separately from version compatibility and is never
synthesised here. Under the pinned `engineering-assurance` v0.2.0 the packaged
matrix records `pending_human_acceptance` and ships no `human_acceptance_recorded`
predicate; both landed on that repository's `main` after the tag, with no v0.2.1
released. That is `agent-ix/engineering-assurance#20`. This module therefore
reports the acceptance state it actually finds in the installed matrix and does
not invent a release that does not exist, pin a branch head, or let a
`pending_human_acceptance` matrix read as an approval.

    <ASSURANCE_PYTHON> scripts/assurance/toolchain.py --json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from engineering_assurance import PACKAGE_ROOT
from engineering_assurance.compatibility import (
    Classification,
    accepted,
    classify_all,
    load_matrix,
)

ROOT = Path(__file__).resolve().parent.parent
SEMVER = re.compile(r"\b(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)\b")

# The acceptance decision this campaign is gated on, as an external fact with a
# revision behind it rather than a value this repository asserts about itself.
ACCEPTANCE_OF_RECORD = {
    "repository": "agent-ix/engineering-assurance",
    "revision": "ae50e13",
    "recorded_by": "Peter Krenesky",
    "recorded_at": "2026-09-01",
    "known_gap": "agent-ix/engineering-assurance#20",
    "note": (
        "Recorded on engineering-assurance main after the v0.2.0 tag. The pinned "
        "v0.2.0 package therefore reports pending_human_acceptance. This field "
        "reports where the decision lives; it does not make it."
    ),
}


def observe(command: list[str]) -> str | None:
    """Read one tool's self-reported version, or None if it cannot be read.

    Every failure is None rather than a guess. A missing binary, a non-zero
    exit, a timeout and unparseable output are all "not observed", and the
    matrix turns that into `unknown` rather than into an absence that a caller
    might read as harmless.
    """
    if shutil.which(command[0]) is None:
        return None
    try:
        completed = subprocess.run(
            command, capture_output=True, text=True, timeout=60, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if completed.returncode != 0:
        return None
    match = SEMVER.search(completed.stdout.strip())
    return match.group(1) if match else None


def observe_quire() -> str | None:
    """quire reports structured provenance; read the CLI version from it.

    `quire --version` prints a human string. `quire provenance` is the JSON the
    tool publishes for exactly this question, so it is what gets read.
    """
    if shutil.which("quire") is None:
        return None
    try:
        completed = subprocess.run(
            ["quire", "provenance"], capture_output=True, text=True, timeout=60, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if completed.returncode != 0:
        return None
    try:
        return str(json.loads(completed.stdout)["cli"]["version"])
    except (json.JSONDecodeError, KeyError, TypeError):
        return None


def observe_engineering_assurance() -> str | None:
    """Read the installed module's own manifest version.

    Deliberately the installed package rather than a checkout's git tag: this
    repository consumes a released module, and what a neighbouring working copy
    happens to be checked out at is not evidence about what is installed here.
    """
    manifest = PACKAGE_ROOT / "manifest.yaml"
    if not manifest.is_file():
        return None
    for line in manifest.read_text(encoding="utf-8").splitlines():
        if line.startswith("version:"):
            return line.split(":", 1)[1].strip() or None
    return None


def observe_all() -> dict[str, str | None]:
    return {
        "quire-cli": observe_quire(),
        "quoin": observe(["quoin", "--version"]),
        "ix-flow": observe(["ix-flow", "--version"]),
        "engineering-assurance": observe_engineering_assurance(),
    }


def artifact_digest_mismatches(pins: dict[str, Any]) -> list[str]:
    """Re-hash every upstream artifact this repository pins by digest.

    A pinned artifact that changed upstream is drift this repository has to see,
    because the mapping and schema it reads are the meaning of its results.
    """
    mismatches = []
    for artifact in pins["consumed_artifacts"]:
        relative = artifact["path"].removeprefix("engineering_assurance/")
        path = PACKAGE_ROOT / relative
        if not path.is_file():
            mismatches.append(f"{artifact['path']}: absent from the installed release")
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != artifact["sha256"]:
            mismatches.append(
                f"{artifact['path']}: {actual}, pins.json records {artifact['sha256']}"
            )
    return mismatches


def mirror_references() -> list[str]:
    """Find any reference to the internal npm mirror.

    `npm.ix` is unreachable from CI and lags the public registry, so a pin that
    names it cannot be installed by anyone outside the network. The scan is
    line-by-line over the files a pin can actually live in.

    A `#` comment line is skipped, and the reason is that it cannot install
    anything — not that it is allowed to mention the mirror. That distinction
    matters: the exemption is a property of the line's syntax, not of its
    wording, so this check does not become defeatable by phrasing a real pin as
    prose, and it does not carve out the sentence that describes it. Wave 0's
    FND-003 was the opposite mistake — a substring exemption for its own rule
    text, which rewording would have evaded.
    """
    offenders = []
    candidates = [
        ROOT / "requirements-assurance.txt",
        ROOT / "Cargo.toml",
        ROOT / "Cargo.lock",
        ROOT / ".npmrc",
        ROOT / "package.json",
        ROOT / ".github" / "workflows" / "ci.yml",
    ]
    for path in candidates:
        if not path.is_file():
            continue
        for number, line in enumerate(
            path.read_text(encoding="utf-8", errors="replace").splitlines(), start=1
        ):
            if line.lstrip().startswith("#"):
                continue
            if "npm.ix" in line:
                offenders.append(f"{path.relative_to(ROOT)}:{number}")
    # assurance/pins.json states the rule in prose, so it is read as JSON and
    # only the fields that could actually install something are scanned.
    pins_path = ROOT / "assurance" / "pins.json"
    if pins_path.is_file():
        pins = json.loads(pins_path.read_text(encoding="utf-8"))
        requirement = pins.get("engineering_assurance", {}).get("requirement", "")
        if "npm.ix" in requirement:
            offenders.append("assurance/pins.json:engineering_assurance.requirement")
    return offenders


def report(matrix: dict[str, Any], observed: dict[str, str | None]) -> dict[str, Any]:
    """Project the delegated classification into this repository's report shape.

    Nothing here re-derives a verdict. `classify_all` decides; this transcribes.
    """
    classifications: list[Classification] = classify_all(matrix, observed)
    acceptance = matrix["accepted"]
    pins = json.loads((ROOT / "assurance" / "pins.json").read_text(encoding="utf-8"))
    mismatches = artifact_digest_mismatches(pins)
    offenders = mirror_references()
    versions_ok = accepted(classifications)
    return {
        "schema": "quire-analyze.shared-pin-report/v1",
        "matrix_version": matrix["matrix_version"],
        "versions_compatible": versions_ok,
        "artifact_mismatches": mismatches,
        "mirror_references": offenders,
        "outcome": "passed"
        if versions_ok and not mismatches and not offenders
        else "failed",
        "acceptance_state_in_pinned_matrix": acceptance.get("state"),
        "acceptance_recorded_here": False,
        "acceptance_authority": (
            "engineering_assurance/compatibility-matrix.json in the installed release. "
            "This repository reports it and is not a second acceptance authority."
        ),
        "acceptance_of_record": ACCEPTANCE_OF_RECORD,
        "components": [
            {
                "component": item.component,
                "observed": item.observed,
                "expected": item.expected,
                "verdict": item.verdict,
                "reason": item.reason,
            }
            for item in classifications
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit the observation as JSON")
    parser.add_argument("--out", type=Path, help="also write the observation to this path")
    args = parser.parse_args(argv)

    matrix = load_matrix()
    observation = report(matrix, observe_all())

    if args.out is not None:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(
            json.dumps(observation, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )

    if args.json:
        print(json.dumps(observation, indent=2, sort_keys=True))
    else:
        for item in observation["components"]:
            print(f"{item['verdict']:<12} {item['component']:<24} {item['reason']}")
        print()
        print(
            "versions: "
            + (
                "every component is the pinned version"
                if observation["versions_compatible"]
                else "NOT satisfied"
            )
        )
        for mismatch in observation["artifact_mismatches"]:
            print(f"{'mismatch':<12} {mismatch}")
        for offender in observation["mirror_references"]:
            print(f"{'npm.ix':<12} {offender}")
        print(
            "acceptance state in the pinned v0.2.0 matrix: "
            f"{observation['acceptance_state_in_pinned_matrix']}"
        )
        print(
            "acceptance of record: "
            f"{ACCEPTANCE_OF_RECORD['repository']}@{ACCEPTANCE_OF_RECORD['revision']} "
            f"({ACCEPTANCE_OF_RECORD['recorded_by']}, {ACCEPTANCE_OF_RECORD['recorded_at']}); "
            f"packaging gap {ACCEPTANCE_OF_RECORD['known_gap']}"
        )
        print(f"outcome: {observation['outcome']}")

    # Versions, pinned artifact digests and mirror references gate here.
    # Acceptance does not: it is a decision recorded in another repository at a
    # named revision, and this script reports where it lives rather than
    # manufacturing it from a matrix that says `pending`.
    return 0 if observation["outcome"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
