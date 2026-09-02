#!/usr/bin/env python3
"""Drive the official change-assurance chain over already-produced results (FR-006).

Four things this file deliberately is not.

It is not a producer. It never runs a test, a solver, a compiler or an analysis.
Every input it reads was written by `make assurance-inputs`, and if one is absent
it says so and names that target. A driver that can produce its own inputs is a
driver that can produce a green run out of nothing.

It is not an envelope. Quoin's packaged FR-063 record, FR-064 attestation and
FR-065 receipt schemas are the shapes. This file projects
`assurance/change-assurance.json` into the record body Quoin requires and derives
nothing beyond the digests that file's own `derived_fields` names.

It is not a verdict. It runs `quoin` and reports what `quoin` said. Where a
scenario expects a refusal, the refusal is the expected result and the run is
green because the tool refused, not because the tool agreed.

It is not a retention store. Nothing is written under `evidence/`, nothing is
committed, and the Quoin store it uses lives under `target/`, which is ignored.

Exit status: 0 when every scenario, control and probe matched, 1 when one did
not, 2 on a usage or environment error — which is a different fact from a
mismatch and gets its own code.

    python3 scripts/assurance_chain.py --candidate-revision <sha> --json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
DECLARATION = ROOT / "assurance" / "change-assurance.json"
# Overridable so a test can drive the chain against a mutated COPY of producer
# output without touching the real one. The driver still never writes here.
ASSURANCE_DIR = Path(
    os.environ.get("QUIRE_ANALYZE_ASSURANCE_DIR", ROOT / "target" / "assurance")
)
STORE_ROOT = ROOT / "target" / "assurance-store"

# Every proof obligation's retained result, and the media type its producer
# declares. Stated rather than sniffed, because a producer's content type is part
# of what it produced.
INPUTS: dict[str, tuple[str, str]] = {
    "PROOF-solver-state-census": ("solver-state-census.json", "application/json"),
    "PROOF-engine-availability": ("engine-availability.json", "application/json"),
    "PROOF-shared-pins": ("shared-pins.json", "application/json"),
    "PROOF-legacy-compatibility": ("legacy-compatibility.json", "application/json"),
    "PROOF-quire-static-export": ("quire-static-export.json", "application/json"),
    "PROOF-msrv": ("msrv.jsonl", "application/x-ndjson"),
}

# The outcome vocabulary a producer may declare, and the attestation result each
# maps to. Every value is listed. An outcome this table does not name is refused
# rather than defaulted, because a silently defaulted unknown state is how a
# repository with twenty-four solver outcomes ends up reporting two.
PRODUCER_RESULTS: dict[str, str] = {
    "passed": "passed",
    "failed": "failed",
    "malformed": "failed",
    "unavailable": "unavailable",
    "not_computed": "not_computed",
    "not-computed": "not_computed",
    "vacuous": "not_computed",
    "inconclusive": "not_computed",
}

# Precedence when a stream carries more than one outcome. A single failure
# outranks any number of passes, and an unavailable outranks a not-computed,
# because the strongest thing observed is what the run has to be reported as.
RESULT_PRECEDENCE = ("failed", "unavailable", "not_computed", "passed")

SEMVER = re.compile(r"\b(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)\b")


class ChainError(RuntimeError):
    """The chain could not be driven. Distinct from a scenario that did not match."""


def quoin(*arguments: str, stdin: str | None = None) -> subprocess.CompletedProcess[str]:
    """Invoke the pinned Quoin CLI. It is the only command family this file runs."""
    if shutil.which("quoin") is None:
        raise ChainError("quoin is not on PATH; the pinned CLI is required")
    return subprocess.run(
        ["quoin", *arguments], input=stdin, capture_output=True, text=True, check=False
    )


def tool_version(identity: str, pinned_toolchain: str | None = None) -> str:
    """Observe a tool's version, or refuse.

    There is no default. A fabricated `0.0.0` in a sealed attestation is a lie
    about the environment that produced a result, and it is indistinguishable
    from a real version to everything downstream. If the version cannot be read,
    the chain stops.

    Quoin's attestation schema requires an immutable version — a bare semver or a
    full-length digest — so the semver is extracted from what the tool prints.
    That is a reading of the tool's own self-report, not a substitution: a tool
    that prints no semver at all raises rather than being given one.
    """
    if identity == "cargo" and pinned_toolchain is not None:
        # The MSRV proof runs `rustup run 1.75.0 cargo check`. Asking the default
        # `cargo` its version attests a compiler that did not produce the stream —
        # observed as 1.94.1 against a stream built by 1.75.0. The toolchain is
        # read out of the declared argv so the two can never drift apart.
        if shutil.which("rustup") is None:
            raise ChainError("rustup is not on PATH, so the pinned toolchain cannot be observed")
        completed = subprocess.run(
            ["rustup", "run", pinned_toolchain, "cargo", "--version"],
            capture_output=True,
            text=True,
            check=False,
        )
        if completed.returncode != 0:
            raise ChainError(
                f"rustup run {pinned_toolchain} cargo --version failed; refusing to attest "
                "the default toolchain in its place"
            )
        match = SEMVER.search(completed.stdout)
        if match is None:
            raise ChainError(f"toolchain {pinned_toolchain} printed no semantic version")
        return match.group(1)

    if identity == "quire":
        # `--version`, not `provenance`. Both report the same number, but only one
        # of them is unambiguously an identity question: `provenance` is a
        # subcommand, and a driver that runs subcommands of a producer cannot be
        # held to "it only ever asked tools their version". The narrower call
        # keeps that claim checkable, and the producer-isolation test is what
        # found this — the richer call read as producer work.
        command = ["quire", "--version"]
    elif identity == "quoin":
        command = ["quoin", "--version"]
    elif identity == "cargo":
        command = ["cargo", "--version"]
    else:
        # This repository's own tools carry the crate version, read from the
        # manifest rather than restated here.
        for line in (ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
            if line.startswith("version = "):
                return line.split("=", 1)[1].strip().strip('"')
        raise ChainError("the crate version is not readable from Cargo.toml")

    if shutil.which(command[0]) is None:
        raise ChainError(
            f"{command[0]} is not on PATH, so its version cannot be observed. "
            "An attestation will not be sealed with a made-up version."
        )
    completed = subprocess.run(command, capture_output=True, text=True, check=False)
    if completed.returncode != 0 or not completed.stdout.strip():
        raise ChainError(f"{command[0]} did not report a version; refusing to invent one")
    match = SEMVER.search(completed.stdout)
    if match is None:
        raise ChainError(
            f"{command[0]} printed no semantic version; refusing to invent one"
        )
    return match.group(1)


def require_inputs() -> dict[str, Path]:
    paths = {}
    for proof_id, (name, _) in INPUTS.items():
        path = ASSURANCE_DIR / name
        if not path.is_file():
            raise ChainError(
                f"{path.relative_to(ROOT)} is absent. Run `make assurance-inputs`. "
                "This driver consumes producer output and never creates it, so an "
                "absent input is an error rather than a step it can quietly do itself."
            )
        paths[proof_id] = path
    return paths


def derive_result(proof_id: str, path: Path) -> str:
    """Read the producer's own structured verdict out of the bytes it wrote.

    This is the difference between an attestation that states what happened and
    one that states what the caller hoped. Nothing here parses a transcript for
    words: every producer this repository owns emits a declared structured result
    with an `outcome` field, and `cargo` emits its own JSON message stream, so
    the verdict is read from a field in every case.

    A producer whose output cannot be read at all raises rather than defaulting.
    An attestation that says `passed` because its input was unreadable is the
    single worst failure this file could have, and it is the one Wave 1 shipped.
    Unreadable is raised as a ChainError — exit 2 — rather than left to surface as
    a decoder traceback, because "the producer wrote something I cannot parse" is
    an environment fact and needs to be distinguishable from a proof that ran and
    did not match.
    """
    raw = path.read_text(encoding="utf-8")

    def parse(text: str, where: str) -> Any:
        try:
            return json.loads(text)
        except json.JSONDecodeError as error:
            raise ChainError(
                f"{path.name} is not readable as JSON ({where}): {error}. "
                "A result that cannot be read is not a result that passed."
            ) from error

    if proof_id == "PROOF-msrv":
        # `cargo --message-format=json` emits one JSON object per line and ends
        # with `build-finished`. The verdict is that object's `success` field.
        messages = [
            parse(line, f"line {number}")
            for number, line in enumerate(raw.splitlines(), start=1)
            if line.strip()
        ]
        finished = [item for item in messages if item.get("reason") == "build-finished"]
        if not finished:
            return "not_computed"
        # `{"reason":"build-finished","success":true}` on its own is a one-line
        # file anybody can write. A real check of this crate emits an artifact
        # message for it, so the stream is required to contain one; without this
        # the MSRV proof establishes nothing about the MSRV.
        compiled = {
            item.get("target", {}).get("name")
            for item in messages
            if item.get("reason") == "compiler-artifact"
            and "quire-analyze" in str(item.get("package_id", ""))
        }
        if not compiled:
            raise ChainError(
                f"{path.name} contains no compiler-artifact for quire-analyze. A "
                "build-finished line on its own is not a compilation."
            )
        if any(
            item.get("reason") == "compiler-message"
            and item.get("message", {}).get("level") == "error"
            for item in messages
        ):
            return "failed"
        return "passed" if finished[-1].get("success") is True else "failed"

    if proof_id == "PROOF-quire-static-export":
        export = parse(raw, "document")
        # Quire's export is a static fact set, not a run, so it has no outcome
        # field. What it can be held to is that it actually contains the facts
        # the impact snapshot claims: an empty document is `not_computed`, which
        # is a different answer from a clean export and must not read as one.
        if not isinstance(export, dict) or not export:
            return "not_computed"
        populated = any(
            isinstance(export.get(key), (list, dict)) and export.get(key) for key in export
        )
        return "passed" if populated else "not_computed"

    document = parse(raw, "document")
    if not isinstance(document, dict):
        raise ChainError(f"{path.name} is not a structured result document")
    outcome = document.get("outcome")
    if outcome not in PRODUCER_RESULTS:
        raise ChainError(
            f"{path.name} declares outcome {outcome!r}, which this adapter does not "
            "name. An unlisted state is refused rather than defaulted."
        )
    return PRODUCER_RESULTS[outcome]


def worst(results: list[str]) -> str:
    for candidate in RESULT_PRECEDENCE:
        if candidate in results:
            return candidate
    raise ChainError("a producer result stream carried no outcome at all")


def digest_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pinned_toolchain(argv: list[str]) -> str | None:
    """The rustup toolchain a declared command runs under, if it names one."""
    if len(argv) >= 3 and argv[0] == "rustup" and argv[1] == "run":
        return argv[2]
    return None


class Chain:
    """Thin wrappers over the pinned Quoin CLI. No wrapper decides anything."""

    def __init__(self, store: Path, candidate_revision: str) -> None:
        self.store = store
        self.candidate_revision = candidate_revision
        self.declaration = json.loads(DECLARATION.read_text(encoding="utf-8"))

    def record_body(self) -> dict[str, Any]:
        """Project the declaration into Quoin's record body, deriving only digests."""
        declaration = self.declaration
        body = json.loads(json.dumps(declaration["record"]))
        sources = declaration["sources"]

        body["subject"]["base_revision"] = self.candidate_revision

        for connection in body["source_connections"]:
            source_id = connection["source_id"]
            path = ROOT / sources[source_id]
            if not path.is_file():
                raise ChainError(f"source {source_id} names {path}, which does not exist")
            connection["revision"] = self.candidate_revision
            connection["digest"] = digest_of(path)

        for obligation in body["definition"]["proof_obligations"]:
            # Declaration-local fields. The declaration is deliberately richer
            # than Quoin's record body — `accepted_results` is this repository's
            # own gate and has no place in the sealed record, whose schema
            # refuses fields it does not define.
            obligation.pop("accepted_results", None)
            configuration = obligation.pop("configuration")
            path = ROOT / configuration
            if not path.is_file():
                raise ChainError(
                    f"{obligation['proof_id']} names configuration {configuration}, "
                    "which does not exist"
                )
            obligation["configuration_digest"] = digest_of(path)

        export_path = ASSURANCE_DIR / INPUTS["PROOF-quire-static-export"][0]
        body["impact_snapshot"].update(self.impact_snapshot(export_path))
        return body

    def impact_snapshot(self, export_path: Path) -> dict[str, Any]:
        """Read the export and say what it actually contains.

        `completeness`, `truncated` and `gaps` used to be stated in the
        declaration as `complete` / `false` / `[]` about a file nothing ever
        opened — so an empty export sealed a record asserting a complete,
        ungapped snapshot over `{}`. They are read here instead.
        """
        export = json.loads(export_path.read_text(encoding="utf-8"))
        if not isinstance(export, dict):
            raise ChainError("the Quire static export is not a document")
        populated = {
            key: value
            for key, value in export.items()
            if isinstance(value, (list, dict)) and value
        }
        gaps = sorted(key for key, value in export.items() if not value)
        return {
            "revision": self.candidate_revision,
            "digest": digest_of(export_path),
            # Quoin's enum is complete|incomplete. An export with an empty
            # section is incomplete, and that is a fact worth sealing.
            "completeness": "complete" if populated and not gaps else "incomplete",
            "truncated": not populated,
            "gaps": gaps,
        }

    def seal_record(self, body: dict[str, Any]) -> str:
        completed = quoin(
            "change-assurance",
            "seal-record",
            "--repo",
            str(self.store),
            "--input",
            "-",
            "--json",
            stdin=json.dumps(body),
        )
        if completed.returncode != 0:
            raise ChainError(f"seal-record refused the declared record: {completed.stderr.strip()}")
        return json.loads(completed.stdout)["digest"]

    def attestation_body(
        self, record_digest: str, proof_id: str, result: str, obligation: dict[str, Any]
    ) -> dict[str, Any]:
        return {
            "schema_version": 1,
            "record_type": "proof_attestation",
            "attestation_id": f"{proof_id}-{self.candidate_revision[:12]}",
            "record_digest": record_digest,
            "candidate_revision": self.candidate_revision,
            "proof_id": proof_id,
            "command": obligation["command"],
            "tool": {
                "identity": obligation["tool_identity"],
                "version": tool_version(
                    obligation["tool_identity"], pinned_toolchain(obligation["command"]["argv"])
                ),
                "configuration_digest": obligation["configuration_digest"],
            },
            "environment": {
                "platform": sys.platform,
                "producer": "make assurance-inputs",
            },
            "observed_at": OBSERVED_AT,
            "result": result,
        }

    def seal_attestation(
        self, body: dict[str, Any], output: Path, media_type: str
    ) -> subprocess.CompletedProcess[str]:
        return quoin(
            "change-assurance",
            "seal-attestation",
            "--input",
            "-",
            "--output",
            str(output),
            "--media-type",
            media_type,
            "--json",
            stdin=json.dumps(body),
        )

    def intake(self, sealed: str, output: Path) -> subprocess.CompletedProcess[str]:
        return quoin(
            "change-assurance",
            "intake",
            "--repo",
            str(self.store),
            "--attestation",
            "-",
            "--output",
            str(output),
            "--json",
            stdin=sealed,
        )

    def receipt(
        self, record_digest: str, selections: dict[str, str], decisions: Path
    ) -> subprocess.CompletedProcess[str]:
        arguments = [
            "change-assurance",
            "receipt",
            "--repo",
            str(self.store),
            "--record",
            record_digest,
            "--candidate-revision",
            self.candidate_revision,
            "--decisions",
            str(decisions),
            "--json",
        ]
        for proof_id, digest in selections.items():
            arguments += ["--select", f"{proof_id}={digest}"]
        return quoin(*arguments)

    def verify_receipt(self, receipt: str) -> subprocess.CompletedProcess[str]:
        return quoin(
            "change-assurance", "verify-receipt", "--input", "-", "--json", stdin=receipt
        )


OBSERVED_AT = ""


def run(candidate_revision: str, keep_store: bool) -> dict[str, Any]:
    global OBSERVED_AT
    import datetime

    OBSERVED_AT = (
        datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat()
    )

    inputs = require_inputs()
    STORE_ROOT.mkdir(parents=True, exist_ok=True)
    store = Path(tempfile.mkdtemp(prefix="run-", dir=STORE_ROOT))
    chain = Chain(store, candidate_revision)

    scenarios: list[dict[str, Any]] = []
    controls: list[dict[str, Any]] = []

    body = chain.record_body()
    record_digest = chain.seal_record(body)
    # Read from the projected body for the sealed fields, and from the
    # declaration for this repository's own gate, which is never sealed.
    obligations = {
        obligation["proof_id"]: obligation
        for obligation in body["definition"]["proof_obligations"]
    }
    for obligation in chain.declaration["record"]["definition"]["proof_obligations"]:
        obligations[obligation["proof_id"]]["accepted_results"] = obligation[
            "accepted_results"
        ]

    # ------------------------------------------------------------------
    # The honest path. Every result is read out of the producer's bytes.
    # ------------------------------------------------------------------
    observed_results: dict[str, str] = {}
    selections: dict[str, str] = {}
    for proof_id, path in inputs.items():
        media_type = INPUTS[proof_id][1]
        observed = derive_result(proof_id, path)
        observed_results[proof_id] = observed
        accepted = obligations[proof_id]["accepted_results"]
        acceptable = observed in accepted

        attestation = chain.attestation_body(
            record_digest, proof_id, observed, obligations[proof_id]
        )
        sealed = chain.seal_attestation(attestation, path, media_type)
        if sealed.returncode != 0:
            raise ChainError(f"{proof_id}: seal-attestation refused: {sealed.stderr.strip()}")
        sealed_json = sealed.stdout
        taken = chain.intake(sealed_json, path)
        if taken.returncode != 0:
            raise ChainError(
                f"{proof_id}: intake refused an unmodified producer output: "
                f"{taken.stderr.strip()}"
            )
        retained_directory = Path(json.loads(taken.stdout)["directory"])
        retained = retained_directory / "output.bin"
        identical = retained.is_file() and retained.read_bytes() == path.read_bytes()
        selections[proof_id] = json.loads(sealed_json)["digest"]
        # Both halves are required. Byte identity says Quoin retained what the
        # producer wrote; acceptability says the producer established what this
        # proof exists to establish. Checking only the first is how a run in
        # which three proofs declared `inconclusive`, `not_computed` and
        # `unavailable` still reported `outcome: passed` and exit 0.
        scenarios.append(
            {
                "scenario": f"honest-{proof_id}",
                "proof": proof_id,
                "expected": (
                    f"intake retains the exact producer bytes and the result is one of "
                    f"{accepted}"
                ),
                "observedResult": observed,
                "acceptedResults": accepted,
                "resultAcceptable": acceptable,
                "retainedBytesIdentical": identical,
                "matched": identical and acceptable,
                "demonstrates": observed if identical and acceptable else None,
            }
        )

    # ------------------------------------------------------------------
    # Negatives. Each is a refusal that must happen, and each names the
    # positive control that proves the same path can accept something.
    # ------------------------------------------------------------------
    census_path = inputs["PROOF-solver-state-census"]
    proof_id = "PROOF-solver-state-census"

    # A sealed attestation whose retained output has been altered underneath it.
    altered = store / "altered-output.json"
    original = census_path.read_bytes()
    altered.write_bytes(original.replace(b'"outcome"', b'"0utcome"', 1))
    attestation = chain.attestation_body(
        record_digest, proof_id, "passed", obligations[proof_id]
    )
    sealed = chain.seal_attestation(attestation, census_path, "application/json")
    taken = chain.intake(sealed.stdout, altered)
    controls.append(
        {
            "scenario": "tampered-retained-bytes-refused",
            "pairs_with": f"honest-{proof_id}",
            "expected": "intake refuses bytes that are not the ones sealed",
            "matched": taken.returncode != 0,
            "demonstrates": "tampered" if taken.returncode != 0 else None,
        }
    )

    # A sealed attestation whose own JSON has been edited after sealing.
    tampered_attestation = json.loads(sealed.stdout)
    tampered_attestation["result"] = "passed"
    tampered_attestation["candidate_revision"] = "0" * 40
    taken = chain.intake(json.dumps(tampered_attestation), census_path)
    controls.append(
        {
            "scenario": "tampered-attestation-refused",
            "pairs_with": f"honest-{proof_id}",
            "expected": "intake refuses a sealed attestation whose digest no longer covers it",
            "matched": taken.returncode != 0,
            "demonstrates": "tampered" if taken.returncode != 0 else None,
        }
    )

    # A record body that states its own digest. Quoin computes the digest; a
    # caller-supplied one is refused rather than overwritten.
    presupplied = chain.record_body()
    presupplied["digest"] = "0" * 64
    completed = quoin(
        "change-assurance",
        "seal-record",
        "--repo",
        str(store),
        "--input",
        "-",
        "--json",
        stdin=json.dumps(presupplied),
    )
    controls.append(
        {
            "scenario": "presupplied-record-digest-refused",
            "pairs_with": f"honest-{proof_id}",
            "expected": "seal-record refuses a body that states its own digest",
            "matched": completed.returncode != 0,
            "demonstrates": "malformed" if completed.returncode != 0 else None,
        }
    )

    # An unlisted producer outcome must be refused by the adapter, not defaulted.
    unlisted = store / "unlisted-outcome.json"
    unlisted.write_text(json.dumps({"outcome": "probably-fine"}), encoding="utf-8")
    refused_unlisted = False
    try:
        derive_result("PROOF-shared-pins", unlisted)
    except ChainError:
        refused_unlisted = True
    controls.append(
        {
            "scenario": "unlisted-producer-outcome-refused",
            "pairs_with": f"honest-{proof_id}",
            "expected": "the adapter refuses an outcome its table does not name",
            "matched": refused_unlisted,
            "demonstrates": "unsupported" if refused_unlisted else None,
        }
    )

    # ------------------------------------------------------------------
    # The receipt. No ix-flow decision exists and none is synthesized, so the
    # expected answer is `incomplete` — which is not a failure and not a pass.
    # ------------------------------------------------------------------
    decisions = store / "decisions.json"
    decisions.write_text(
        json.dumps({"run_id": chain.declaration["record"]["review_workflow"]["run_id"],
                    "events": []}),
        encoding="utf-8",
    )
    receipt = chain.receipt(record_digest, selections, decisions)
    receipt_document = json.loads(receipt.stdout) if receipt.stdout.strip() else None
    receipt_outcome = receipt_document.get("outcome") if receipt_document else None
    scenarios.append(
        {
            "scenario": "receipt-without-a-human-decision",
            "proof": None,
            "expected": "incomplete, because nobody made the decision it needs",
            "observedResult": receipt_outcome,
            "matched": receipt_outcome == "incomplete",
            "demonstrates": "inconclusive" if receipt_outcome == "incomplete" else None,
        }
    )

    # The whole point of this repository's migration, checked at the far end of
    # the chain rather than only at the producer. An engine that was not there
    # must still be `unavailable` in the receipt: if it arrived as `failed` the
    # analysis would read as refuted, and if it arrived as `passed` it would read
    # as decided. Both are false, and both are one careless default away.
    engine_proof = None
    if receipt_document is not None:
        engine_proof = next(
            (
                proof
                for proof in receipt_document.get("proofs", [])
                if proof["proof_id"] == "PROOF-engine-availability"
            ),
            None,
        )
    carried = (
        engine_proof is not None
        and observed_results.get("PROOF-engine-availability") == "unavailable"
        and "result_unavailable" in engine_proof.get("reasons", [])
    )
    scenarios.append(
        {
            "scenario": "unavailable-survives-to-the-receipt",
            "proof": "PROOF-engine-availability",
            "expected": "the receipt states result_unavailable, not passed and not failed",
            "observedResult": engine_proof.get("reasons") if engine_proof else None,
            "matched": carried,
            "demonstrates": "unavailable" if carried else None,
        }
    )
    if receipt_document is not None:
        verified = chain.verify_receipt(receipt.stdout)
        scenarios.append(
            {
                "scenario": "receipt-re-verifies",
                "proof": None,
                "expected": "the emitted receipt re-verifies against its own seal",
                "observedResult": verified.returncode,
                "matched": verified.returncode in (0, 1),
                "demonstrates": "partial" if verified.returncode in (0, 1) else None,
            }
        )

    # A control naming a scenario that did not run pairs with nothing, and an
    # assertion over an empty pairing is vacuously true. Refuse it.
    names = {scenario["scenario"] for scenario in scenarios}
    for control in controls:
        if control["pairs_with"] not in names:
            raise ChainError(
                f"{control['scenario']} pairs with {control['pairs_with']}, "
                "which is not a scenario that ran"
            )

    if not keep_store:
        shutil.rmtree(store, ignore_errors=True)

    cases = scenarios + controls
    mismatches = [case["scenario"] for case in cases if not case["matched"]]
    demonstrated = sorted(
        {case["demonstrates"] for case in cases if case["matched"] and case["demonstrates"]}
    )
    return {
        "schema": "quire-analyze.assurance-chain-report/v1",
        "candidateRevision": candidate_revision,
        "recordDigest": record_digest,
        "observedResults": observed_results,
        "scenarios": scenarios,
        "controls": controls,
        "proofResultRollUp": worst(list(observed_results.values())),
        "casesMatched": len(cases) - len(mismatches),
        "casesTotal": len(cases),
        "statesDemonstrated": demonstrated,
        "mismatches": mismatches,
        "outcome": "passed" if not mismatches else "failed",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-revision", required=True)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--keep-store", action="store_true")
    args = parser.parse_args(argv)

    try:
        report = run(args.candidate_revision, args.keep_store)
    except ChainError as error:
        print(f"assurance chain: {error}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        for proof_id, result in report["observedResults"].items():
            print(f"{result:<14} {proof_id}")
        print()
        print(f"cases: {report['casesMatched']}/{report['casesTotal']} matched")
        print(f"states demonstrated: {', '.join(report['statesDemonstrated'])}")
        for mismatch in report["mismatches"]:
            print(f"  mismatch: {mismatch}")
        print(f"outcome: {report['outcome']}")

    return 0 if report["outcome"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
