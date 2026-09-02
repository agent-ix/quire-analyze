//! Native producer: is the pinned real-engine corpus runnable on this machine?
//!
//! FR-005 pins Z3 5.1.0 and cvc5 1.3.4 by release asset, and TC-006 pins each
//! executable by SHA-256 and locates it through `QUIRE_Z3` / `QUIRE_CVC5`. When
//! those binaries are not present, the differential corpus did not fail — it did
//! not run. Those are different facts, and this program keeps them apart so the
//! assurance lane can attest `unavailable` instead of inventing a pass or a
//! failure out of an absent tool.
//!
//! It installs nothing and never falls back to an engine found on `PATH`: an
//! engine nobody pinned is not the engine the conformance claim is about, and
//! substituting one would make the corpus agree about something else.
//!
//! Usage:
//!
//! ```text
//! cargo run --quiet --example engine_availability -- --json
//! ```

use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use serde_json::json;
use sha2::{Digest as _, Sha256};

const SCHEMA: &str = "quire-analyze.engine-availability/v1";

/// The engines TC-006 pins: the locating variable, the expected version prefix,
/// the version argument, and the executable digest the corpus is written
/// against. A binary that is present but hashes differently is not the pinned
/// engine and is reported as such rather than used.
const PINNED: [(&str, &str, &str, &str, &str); 2] = [
    (
        "z3",
        "QUIRE_Z3",
        "Z3 version 5.1.0",
        "-version",
        "54bae839dd54e262edac6f755fc99659ce2a121301faff20a3e3b94478dcead0",
    ),
    (
        "cvc5",
        "QUIRE_CVC5",
        "cvc5 1.3.4",
        "--version",
        "7562a8b0b835e3eaad5f1a7b4616cd762350cf567b6be03d7e8ee24fa5ced5ee",
    ),
];

fn main() -> ExitCode {
    let emit_json = std::env::args().any(|argument| argument == "--json");

    let mut engines = Vec::new();
    let mut pinned_present = 0_usize;
    for (name, variable, expected_version, version_argument, expected_digest) in PINNED {
        let located = std::env::var_os(variable).map(PathBuf::from);
        let readable = located.as_ref().filter(|path| path.is_file());
        let digest = readable.and_then(|path| digest_of(path.as_path()));
        let observed = readable.and_then(|path| observe_version(path.as_path(), version_argument));

        // Five states, none of which collapses into another. In particular
        // "declared but missing" is not "not declared", and "wrong bytes" is not
        // "wrong version": each names a different thing to go and fix.
        let state = match (&located, &readable, &digest, &observed) {
            (None, ..) => "not-declared",
            (Some(_), None, ..) => "declared-but-absent",
            (_, Some(_), None, _) => "unreadable",
            (_, Some(_), Some(actual), _) if actual != expected_digest => "digest-mismatch",
            (_, Some(_), Some(_), None) => "unreadable-version",
            (_, Some(_), Some(_), Some(version)) if !version.starts_with(expected_version) => {
                "version-mismatch"
            }
            _ => "pinned",
        };
        if state == "pinned" {
            pinned_present += 1;
        }
        engines.push(json!({
            "engine": name,
            "locatedBy": variable,
            "expectedVersion": expected_version,
            "expectedSha256": expected_digest,
            "path": located.as_ref().map(|path| path.display().to_string()),
            "observedSha256": digest,
            "observedVersion": observed,
            "state": state,
        }));
    }

    // `unavailable` is the honest answer and it is not `failed`. The corpus was
    // never executed, so nothing about it was decided. The `#[ignore]` on
    // `official_z3_cvc5_differential_corpus_agrees` is the same statement made in
    // cargo's vocabulary; this document is what the assurance lane can read.
    let outcome = if pinned_present == PINNED.len() {
        "passed"
    } else {
        "unavailable"
    };

    let document = json!({
        "schema": SCHEMA,
        "producer": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
        "outcome": outcome,
        "reason": if outcome == "passed" {
            "both pinned engines are present at their pinned digests and versions".to_owned()
        } else {
            format!(
                "{pinned_present}/{} pinned engines are present; the real-engine differential \
                 corpus did not run and nothing about it was decided",
                PINNED.len()
            )
        },
        "enginesPinned": PINNED.len(),
        "enginesPresent": pinned_present,
        "engines": engines,
    });

    if emit_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&document).expect("serializable availability")
        );
    } else {
        println!(
            "engine availability: {outcome} ({pinned_present}/{} pinned)",
            PINNED.len()
        );
        for engine in &engines {
            println!(
                "  {:<6} {}",
                engine["engine"].as_str().unwrap_or("?"),
                engine["state"].as_str().unwrap_or("?")
            );
        }
    }

    // Exit zero on every path. An absent engine is a reported state, not a broken
    // producer, and a non-zero exit would be indistinguishable from this program
    // itself failing. The assurance lane reads `outcome`, never this code.
    ExitCode::SUCCESS
}

fn digest_of(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(bytes)))
}

fn observe_version(path: &Path, argument: &str) -> Option<String> {
    let output = std::process::Command::new(path)
        .arg(argument)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}
