//! The `run-package` runner (FR-009, NFR-003).
//!
//! The runner reads one `quire.contract-package/v1` document, takes one consistency
//! request from its `analysis_claim` nodes, runs Z3 and cvc5 over the lowered query
//! only through `producer-execute`, and publishes one `quire.differential-report/v2`
//! report. The exit status follows the report: `0` satisfied, `1` refuted, `2` usage or
//! input error, `3` non-conclusive, `4` publication or executor failure.

use std::{
    ffi::OsString,
    fmt, fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, OpenOptionsExt},
        process::ExitStatusExt,
    },
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quire_contract_model::{
    CheckedPackageReadLimits, ContractPackageReadResult, ContractPackageWireV1,
};

use crate::{
    compare_solver_records, lower_analysis_request, parse_result_line, publish_report_new,
    query_request, render_differential_report, render_differential_summary,
    solver_record_from_evidence, translate_analysis_claims, validate_differential_report,
    version_request, version_text, AdapterLimits, AnalysisStatus, DifferentialDisposition,
    ProducerExecutionRequest, ProducerExecutionResult, PublicationError, PublicationState,
    QueryBundle, SolverConfig, SolverEngine, SolverRecord, MAX_PRODUCER_STREAM_BYTES,
    MAX_SOLVER_PATH_BYTES,
};

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

/// The staged query file inside the capability root.
const QUERY_FILE: &str = "query.smt2";
/// The most bytes of one `producer-execute` result line the runner reads.
///
/// A result line carries up to [`MAX_PRODUCER_STREAM_BYTES`] stdout bytes and as many stderr
/// bytes, each rendered as a JSON number of at most three digits and a comma (four characters),
/// plus an envelope of timing, state and host fields that stays far below one MiB.
const MAX_RESULT_LINE_BYTES: u64 =
    RESULT_STREAMS * MAX_PRODUCER_STREAM_BYTES as u64 * RESULT_CHARACTERS_PER_STREAM_BYTE
        + RESULT_ENVELOPE_BYTES;
/// The captured streams of one result: stdout and stderr.
const RESULT_STREAMS: u64 = 2;
/// The most JSON characters one captured byte takes: up to three digits and a comma.
const RESULT_CHARACTERS_PER_STREAM_BYTE: u64 = 4;
/// The room for everything in a result line other than the captured bytes.
const RESULT_ENVELOPE_BYTES: u64 = 1024 * 1024;
/// The most bytes of the package file the runner reads, one past the reader's ceiling so that
/// an oversized document is refused by the reader as `Incomplete`.
const MAX_PACKAGE_READ_BYTES: u64 = CheckedPackageReadLimits::bounded().bytes + 1;

static ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// Run the analysis and return the exit status of a published report.
///
/// # Errors
///
/// Returns the exit status and diagnostic of a usage, input, executor or publication failure;
/// no report is written for such a failure, except that a publication that fails with
/// `published-durability-unknown` leaves the renamed report in place.
pub fn run_package(arguments: &RunPackageArguments) -> Result<u8, RunnerError> {
    let producer = checked_executable(&arguments.producer_execute, "--producer-execute")?;
    let z3 = solver_config(SolverEngine::Z3, &arguments.z3, "--z3")?;
    let cvc5 = solver_config(SolverEngine::Cvc5, &arguments.cvc5, "--cvc5")?;
    let output = std::path::absolute(&arguments.output).map_err(|error| {
        RunnerError::usage(format!(
            "--output {}: cannot be resolved: {error}",
            arguments.output.display()
        ))
    })?;
    let parent = output.parent().ok_or_else(|| {
        RunnerError::usage(format!("--output {}: has no parent", output.display()))
    })?;
    reject_symlinks(parent, "--output parent")?;

    let query = read_query(&arguments.package)?;

    if fs::symlink_metadata(&output).is_ok() {
        return Err(RunnerError::failure(format!(
            "--output {}: the destination exists",
            output.display()
        )));
    }
    let root = CapabilityRoot::create(parent).map_err(|error| {
        RunnerError::failure(format!(
            "cannot create the capability root under {}: {error}",
            parent.display()
        ))
    })?;
    root.stage_query(query.query().as_bytes())
        .map_err(|error| RunnerError::failure(format!("cannot stage the query: {error}")))?;

    let z3_record = run_engine(&producer, root.path(), &query, &z3)?;
    let cvc5_record = run_engine(&producer, root.path(), &query, &cvc5)?;
    let run = compare_solver_records(&query, &z3_record, &cvc5_record);
    let bytes = render_differential_report(&query, &run)
        .and_then(|bytes| validate_differential_report(&bytes, &query, &run).map(|()| bytes))
        .map_err(|error| RunnerError::failure(format!("the report is not valid: {error}")))?;
    publish_with(&output, &bytes, publish_report_new)?;
    drop(root);
    eprint!("{}", render_differential_summary(&run));
    eprintln!("quire-analyze: published {}", output.display());
    Ok(exit_status(run.disposition(), run.agreed_status()))
}

fn exit_status(disposition: DifferentialDisposition, agreed: Option<AnalysisStatus>) -> u8 {
    match (disposition, agreed) {
        (DifferentialDisposition::Agreement, Some(AnalysisStatus::Satisfied)) => EXIT_SATISFIED,
        (DifferentialDisposition::Agreement, Some(AnalysisStatus::Refuted)) => EXIT_REFUTED,
        _ => EXIT_NON_CONCLUSIVE,
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

/// An absolute, UTF-8, bounded path without a NUL and without a symbolic link in any component.
fn checked_executable(path: &Path, label: &str) -> Result<PathBuf, RunnerError> {
    let text = path
        .to_str()
        .ok_or_else(|| RunnerError::usage(format!("{label}: the path is not UTF-8")))?;
    if text.is_empty() || !path.is_absolute() {
        return Err(RunnerError::usage(format!(
            "{label} {text:?}: the path must be absolute and non-empty"
        )));
    }
    if text.contains('\0') || text.len() > MAX_SOLVER_PATH_BYTES {
        return Err(RunnerError::usage(format!(
            "{label}: the path contains NUL or exceeds {MAX_SOLVER_PATH_BYTES} bytes"
        )));
    }
    reject_symlinks(path, label)?;
    Ok(path.to_path_buf())
}

fn solver_config(
    engine: SolverEngine,
    path: &Path,
    label: &str,
) -> Result<SolverConfig, RunnerError> {
    let path = checked_executable(path, label)?;
    SolverConfig::new(engine, path, AdapterLimits::default())
        .map_err(|error| RunnerError::usage(format!("{label}: {}", error.message())))
}

/// Refuse a path with a symbolic link in any component. A component that does not exist is not
/// a link, and nothing below it exists.
fn reject_symlinks(path: &Path, label: &str) -> Result<(), RunnerError> {
    let mut prefix = PathBuf::new();
    for component in path.components() {
        prefix.push(component);
        match fs::symlink_metadata(&prefix) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(RunnerError::usage(format!(
                    "{label} {}: {} is a symbolic link",
                    path.display(),
                    prefix.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(RunnerError::usage(format!(
                    "{label} {}: cannot inspect {}: {error}",
                    path.display(),
                    prefix.display()
                )));
            }
        }
    }
    Ok(())
}

/// Read and admit the package, translate it and lower the one consistency request.
fn read_query(package: &Path) -> Result<QueryBundle, RunnerError> {
    let unreadable = |error: &dyn fmt::Display| {
        RunnerError::usage(format!("--package {}: {error}", package.display()))
    };
    let metadata = fs::metadata(package).map_err(|error| unreadable(&error))?;
    if !metadata.is_file() {
        return Err(unreadable(&"not a regular file"));
    }
    let mut bytes = Vec::new();
    fs::File::open(package)
        .and_then(|file| file.take(MAX_PACKAGE_READ_BYTES).read_to_end(&mut bytes))
        .map_err(|error| unreadable(&error))?;
    let admitted = match ContractPackageWireV1::read(
        &bytes,
        CheckedPackageReadLimits::bounded(),
        &Default::default(),
    ) {
        ContractPackageReadResult::Admitted(admitted) => admitted,
        ContractPackageReadResult::Refused(refusal) => {
            return Err(unreadable(&format!("the package is refused: {refusal}")));
        }
        ContractPackageReadResult::Incomplete(incomplete) => {
            return Err(unreadable(&format!(
                "the package exhausts a read limit: {incomplete:?}"
            )));
        }
        ContractPackageReadResult::CatalogUnavailable(error) => {
            return Err(RunnerError::failure(format!(
                "the package reader's operation catalog is unavailable: {error}"
            )));
        }
    };
    let translation = translate_analysis_claims(&admitted)
        .map_err(|error| RunnerError::usage(format!("--package: {error}")))?;
    lower_analysis_request(translation.request()).map_err(|errors| {
        let cause = errors.first().map_or_else(
            || "the request is refused".to_owned(),
            |error| format!("{}: {}", error.path(), error.message()),
        );
        RunnerError::usage(format!("--package: the request cannot be lowered: {cause}"))
    })
}

/// The runner-owned capability root, a `0700` directory removed when the guard drops.
struct CapabilityRoot {
    path: PathBuf,
}

impl CapabilityRoot {
    fn create(parent: &Path) -> io::Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let counter = ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".quire-analyze-run-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn stage_query(&self, query: &[u8]) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.path.join(QUERY_FILE))?;
        file.write_all(query)?;
        file.sync_all()
    }
}

/// Removal runs on every exit path. A removal that fails is named on stderr and does not change
/// the exit status (FR-009-AC-28).
impl Drop for CapabilityRoot {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path) {
            if error.kind() != io::ErrorKind::NotFound {
                eprintln!(
                    "quire-analyze: cannot remove the capability root {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}

/// Run the version then the query invocation of one engine and build its record.
fn run_engine(
    producer: &Path,
    root: &Path,
    query: &QueryBundle,
    config: &SolverConfig,
) -> Result<SolverRecord, RunnerError> {
    let cause = |error: &dyn fmt::Display| {
        RunnerError::failure(format!("{} request: {error}", config.engine().as_str()))
    };
    let version_request = version_request(config, root).map_err(|error| cause(&error))?;
    let version = invoke(producer, root, &version_request)?;
    let query_result = match version_text(config, &version) {
        Ok(text) => {
            let request = query_request(config, root, &text, Path::new(QUERY_FILE))
                .map_err(|error| cause(&error))?;
            Some(invoke(producer, root, &request)?)
        }
        Err(_) => None,
    };
    Ok(solver_record_from_evidence(
        query,
        config,
        &version,
        query_result.as_ref(),
    ))
}

/// One `producer-execute --request -` process: the request on stdin, exactly one result line on
/// stdout, and a zero exit status.
fn invoke(
    producer: &Path,
    root: &Path,
    request: &ProducerExecutionRequest,
) -> Result<ProducerExecutionResult, RunnerError> {
    let failure = |message: String| RunnerError::failure(format!("producer-execute: {message}"));
    let mut child = Command::new(producer)
        .args(["producer-execute", "--request", "-"])
        .env_clear()
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| failure(format!("cannot start {}: {error}", producer.display())))?;
    let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        return Err(failure("the process has no piped streams".to_owned()));
    };
    let written = stdin.write_all(&request.to_json_bytes());
    drop(stdin);
    let mut line = Vec::new();
    let read = stdout
        .take(MAX_RESULT_LINE_BYTES + 1)
        .read_to_end(&mut line);
    let oversized = line.len() as u64 > MAX_RESULT_LINE_BYTES;
    if oversized {
        // The process may still be writing; end it so that waiting cannot block.
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|error| failure(format!("cannot wait for the process: {error}")))?;
    if oversized {
        return Err(failure(format!(
            "the result exceeds {MAX_RESULT_LINE_BYTES} bytes"
        )));
    }
    if !status.success() {
        return Err(failure(describe_status(status)));
    }
    written.map_err(|error| failure(format!("cannot write the request: {error}")))?;
    read.map_err(|error| failure(format!("cannot read the result: {error}")))?;
    parse_result_line(&line).map_err(|error| failure(error.to_string()))
}

fn describe_status(status: ExitStatus) -> String {
    match (status.code(), status.signal()) {
        (Some(code), _) => format!("exited with status {code}"),
        (None, Some(signal)) => format!("ended by signal {signal}"),
        (None, None) => "ended without a status".to_owned(),
    }
}
