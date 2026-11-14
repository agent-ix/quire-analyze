//! The unix execution half of the `run-package` runner (FR-009): capability root, staged
//! query, `producer-execute` invocation and publication.

use std::{
    fmt, fs,
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quire_contract_model::{
    CheckedPackageReadLimits, ContractPackageReadResult, ContractPackageWireV1,
};

use super::{
    publish_with, RunPackageArguments, RunnerError, EXIT_NON_CONCLUSIVE, EXIT_REFUTED,
    EXIT_SATISFIED,
};
use crate::{
    compare_solver_records, invoke_producer_execute, lower_analysis_request, publish_report_new,
    query_request, render_differential_report, render_differential_summary,
    solver_record_from_evidence, translate_analysis_claims, validate_differential_report,
    version_request, version_text, AdapterLimits, AnalysisStatus, DifferentialDisposition,
    ProducerExecutionRequest, ProducerExecutionResult, QueryBundle, SolverConfig, SolverEngine,
    SolverRecord, MAX_SOLVER_PATH_BYTES, MAX_SOLVER_WALL_TIME_MS,
};

/// The staged query file inside the capability root.
const QUERY_FILE: &str = "query.smt2";
/// The most bytes of the package file the runner reads, one past the reader's ceiling so that
/// an oversized document is refused by the reader as `Incomplete`.
const MAX_PACKAGE_READ_BYTES: u64 = CheckedPackageReadLimits::bounded().bytes + 1;

static ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    invoke_producer_execute(
        producer,
        root,
        &request.to_json_bytes(),
        Duration::from_millis(MAX_SOLVER_WALL_TIME_MS + 5_000),
        Duration::from_millis(1_000),
    )
    .map_err(|error| failure(error.to_string()))
}
