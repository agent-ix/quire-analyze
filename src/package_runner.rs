//! The `run-package` runner (FR-009, NFR-003).
//!
//! The runner reads one `quire.contract-package/v1` document, takes one consistency
//! request from its `analysis_claim` nodes, runs Z3 and cvc5 over the lowered query
//! only through `producer-execute`, and publishes one `quire.differential-report/v2`
//! report. The exit status follows the report: `0` satisfied, `1` refuted, `2` usage or
//! input error, `3` non-conclusive, `4` publication or executor failure.

use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
};

use crate::{PublicationError, PublicationState};

#[cfg(unix)]
mod execution;
#[cfg(unix)]
pub use execution::run_package;

/// Exit status of a satisfied analysis.
pub const EXIT_SATISFIED: u8 = 0;
/// Exit status of a refuted analysis.
pub const EXIT_REFUTED: u8 = 1;
/// Exit status of a usage or input error.
pub const EXIT_USAGE: u8 = 2;
/// Exit status of a non-conclusive differential run.
pub const EXIT_NON_CONCLUSIVE: u8 = 3;
/// Exit status of a publication or executor failure.
pub const EXIT_FAILURE: u8 = 4;

const USAGE: &str = "usage: quire-analyze run-package --package FILE --output NEW_REPORT.json \
--z3 PATH --cvc5 PATH --producer-execute PATH";

/// The help text of `run-package`: the five arguments and the exit classes.
pub fn run_package_help() -> String {
    [
        USAGE,
        "",
        "Run one differential consistency analysis over a quire.contract-package/v1 document",
        "and publish one quire.differential-report/v2 report.",
        "",
        "Arguments:",
        "  --package FILE            the quire.contract-package/v1 document",
        "  --output NEW_REPORT.json  the report destination, which must not exist beforehand",
        "  --z3 PATH                 the absolute path of the Z3 executable",
        "  --cvc5 PATH               the absolute path of the cvc5 executable",
        "  --producer-execute PATH   the absolute path of the engineering-assurance executable",
        "",
        "Exit status:",
        "  0  satisfied: both solvers agree the clauses are consistent",
        "  1  refuted: both solvers agree the clauses are contradictory",
        "  2  usage or input error: no report is written",
        "  3  non-conclusive: the report is written and retains both engine records",
        "  4  publication or executor failure: no report is written, or its durability is unknown",
        "",
    ]
    .join("\n")
}

/// A failed run: the exit status and the human diagnostic written to stderr.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunnerError {
    class: u8,
    message: String,
}

impl RunnerError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            class: EXIT_USAGE,
            message: message.into(),
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self {
            class: EXIT_FAILURE,
            message: message.into(),
        }
    }

    /// The exit status of the failure.
    pub const fn class(&self) -> u8 {
        self.class
    }

    /// The human-readable cause.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RunnerError {}

/// The five arguments of `run-package`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunPackageArguments {
    package: PathBuf,
    output: PathBuf,
    z3: PathBuf,
    cvc5: PathBuf,
    producer_execute: PathBuf,
}

/// What the command line of `run-package` asks for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunPackageCommand {
    /// Print the help text.
    Help,
    /// Run the analysis.
    Run(RunPackageArguments),
}

/// Parse the arguments that follow `run-package`.
///
/// An unknown, repeated or missing argument and an option without a value are exit `2`
/// errors whose message is the usage.
pub fn parse_run_package_arguments(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<RunPackageCommand, RunnerError> {
    let arguments: Vec<OsString> = arguments.into_iter().collect();
    if let [only] = arguments.as_slice() {
        if only.to_str() == Some("--help") {
            return Ok(RunPackageCommand::Help);
        }
    }
    let mut arguments = arguments.into_iter();
    let mut package = None;
    let mut output = None;
    let mut z3 = None;
    let mut cvc5 = None;
    let mut producer_execute = None;
    while let Some(argument) = arguments.next() {
        let slot = match argument.to_str() {
            Some("--package") => &mut package,
            Some("--output") => &mut output,
            Some("--z3") => &mut z3,
            Some("--cvc5") => &mut cvc5,
            Some("--producer-execute") => &mut producer_execute,
            _ => return Err(RunnerError::usage(USAGE)),
        };
        match (slot.is_none(), arguments.next()) {
            (true, Some(value)) => *slot = Some(PathBuf::from(value)),
            _ => return Err(RunnerError::usage(USAGE)),
        }
    }
    match (package, output, z3, cvc5, producer_execute) {
        (Some(package), Some(output), Some(z3), Some(cvc5), Some(producer_execute)) => {
            Ok(RunPackageCommand::Run(RunPackageArguments {
                package,
                output,
                z3,
                cvc5,
                producer_execute,
            }))
        }
        _ => Err(RunnerError::usage(USAGE)),
    }
}

/// Publish the report through `publish` and map a publication error to its exit `4` failure.
pub(crate) fn publish_with(
    output: &Path,
    bytes: &[u8],
    publish: impl FnOnce(&Path, &[u8]) -> Result<(), PublicationError>,
) -> Result<(), RunnerError> {
    publish(output, bytes).map_err(|error| publication_failure(&error))
}

/// The exit `4` failure of a publication error, naming `published-durability-unknown` when the
/// report was renamed into place but its directory could not be synced.
fn publication_failure(error: &PublicationError) -> RunnerError {
    let state = match error.state() {
        PublicationState::PublishedDurabilityUnknown => "published-durability-unknown",
        PublicationState::DestinationUnmodified => "destination-unmodified",
    };
    RunnerError::failure(format!("{error}; publication state: {state}"))
}

/// Report that the runner cannot run on this target and return exit `4`.
///
/// The runner stages a `0700` capability root and drives `producer-execute` under POSIX
/// process semantics, so it exists only on unix targets. As FR-009 treats a non-Linux host
/// as `unsupported-platform` with no report to publish, the caller gets the exit `4` executor
/// failure.
///
/// # Errors
///
/// Always returns the exit `4` failure.
#[cfg(not(unix))]
pub fn run_package(_arguments: &RunPackageArguments) -> Result<u8, RunnerError> {
    Err(RunnerError::failure(
        "unsupported-platform: run-package requires a unix host",
    ))
}
