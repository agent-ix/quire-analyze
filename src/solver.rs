use std::{
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[cfg(target_os = "linux")]
use std::{
    ffi::CString,
    io::Write,
    process::{Child, Command, Stdio},
    thread,
    time::Instant,
};

#[cfg(target_os = "linux")]
use std::os::{
    fd::AsRawFd,
    unix::process::{CommandExt, ExitStatusExt},
};

use sha2::{Digest as _, Sha256};

use crate::QueryBundle;

pub const SOLVER_PROCESS_PROFILE: &str = "quire.solver-process/v1";
pub const MAX_SOLVER_WALL_TIME_MS: u64 = 5_000;
pub const MAX_SOLVER_CLEANUP_TIME_MS: u64 = 1_000;
pub const MAX_SOLVER_GRACEFUL_CLEANUP_MS: u64 = 100;
pub const MAX_SOLVER_MONITOR_INTERVAL_MS: u64 = 5;
pub const MAX_SOLVER_STDIN_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SOLVER_STDOUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SOLVER_STDERR_BYTES: usize = 1024 * 1024;
pub const MAX_SOLVER_MODEL_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SOLVER_VERSION_BYTES: usize = 64 * 1024;
pub const MAX_SOLVER_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_SOLVER_PATH_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SolverDigest([u8; 32]);

impl SolverDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for SolverDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SolverEngine {
    Z3,
    Cvc5,
}

impl SolverEngine {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Z3 => "z3",
            Self::Cvc5 => "cvc5",
        }
    }

    fn query_arguments(self) -> &'static [&'static str] {
        match self {
            Self::Z3 => &["-in", "-smt2"],
            Self::Cvc5 => &["--lang=smt2", "--no-incremental"],
        }
    }

    fn version_arguments(self) -> &'static [&'static str] {
        match self {
            Self::Z3 => &["-version"],
            Self::Cvc5 => &["--version"],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverPin {
    version: String,
    executable_sha256: SolverDigest,
}

impl SolverPin {
    pub fn new(version: impl Into<String>, executable_sha256: SolverDigest) -> Self {
        Self {
            version: version.into(),
            executable_sha256,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub const fn executable_sha256(&self) -> SolverDigest {
        self.executable_sha256
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterLimits {
    pub wall_time_ms: u64,
    pub cleanup_time_ms: u64,
    pub graceful_cleanup_ms: u64,
    pub monitor_interval_ms: u64,
    pub stdin_bytes: usize,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub model_bytes: usize,
    pub version_bytes: usize,
    pub executable_bytes: u64,
    pub path_bytes: usize,
}

impl Default for AdapterLimits {
    fn default() -> Self {
        Self {
            wall_time_ms: MAX_SOLVER_WALL_TIME_MS,
            cleanup_time_ms: MAX_SOLVER_CLEANUP_TIME_MS,
            graceful_cleanup_ms: MAX_SOLVER_GRACEFUL_CLEANUP_MS,
            monitor_interval_ms: MAX_SOLVER_MONITOR_INTERVAL_MS,
            stdin_bytes: MAX_SOLVER_STDIN_BYTES,
            stdout_bytes: MAX_SOLVER_STDOUT_BYTES,
            stderr_bytes: MAX_SOLVER_STDERR_BYTES,
            model_bytes: MAX_SOLVER_MODEL_BYTES,
            version_bytes: MAX_SOLVER_VERSION_BYTES,
            executable_bytes: MAX_SOLVER_EXECUTABLE_BYTES,
            path_bytes: MAX_SOLVER_PATH_BYTES,
        }
    }
}

impl AdapterLimits {
    pub fn validate(self) -> Result<Self, SolverConfigError> {
        validate_limit("wall_time_ms", self.wall_time_ms, MAX_SOLVER_WALL_TIME_MS)?;
        validate_limit(
            "cleanup_time_ms",
            self.cleanup_time_ms,
            MAX_SOLVER_CLEANUP_TIME_MS,
        )?;
        validate_limit(
            "graceful_cleanup_ms",
            self.graceful_cleanup_ms,
            MAX_SOLVER_GRACEFUL_CLEANUP_MS,
        )?;
        validate_limit(
            "monitor_interval_ms",
            self.monitor_interval_ms,
            MAX_SOLVER_MONITOR_INTERVAL_MS,
        )?;
        validate_limit("stdin_bytes", self.stdin_bytes, MAX_SOLVER_STDIN_BYTES)?;
        validate_limit("stdout_bytes", self.stdout_bytes, MAX_SOLVER_STDOUT_BYTES)?;
        validate_limit("stderr_bytes", self.stderr_bytes, MAX_SOLVER_STDERR_BYTES)?;
        validate_limit("model_bytes", self.model_bytes, MAX_SOLVER_MODEL_BYTES)?;
        validate_limit(
            "version_bytes",
            self.version_bytes,
            MAX_SOLVER_VERSION_BYTES,
        )?;
        validate_limit(
            "executable_bytes",
            self.executable_bytes,
            MAX_SOLVER_EXECUTABLE_BYTES,
        )?;
        validate_limit("path_bytes", self.path_bytes, MAX_SOLVER_PATH_BYTES)?;
        if self.graceful_cleanup_ms > self.cleanup_time_ms {
            return Err(SolverConfigError::new(
                "limits.graceful_cleanup_ms",
                "graceful cleanup exceeds total cleanup",
            ));
        }
        if self.model_bytes > self.stdout_bytes {
            return Err(SolverConfigError::new(
                "limits.model_bytes",
                "model capture exceeds stdout capture",
            ));
        }
        Ok(self)
    }
}

fn validate_limit<T>(path: &str, value: T, maximum: T) -> Result<(), SolverConfigError>
where
    T: Copy + Ord + Default,
{
    if value == T::default() || value > maximum {
        Err(SolverConfigError::new(
            format!("limits.{path}"),
            "limit must be positive and no greater than the v1 profile ceiling",
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverConfigError {
    path: String,
    message: String,
}

impl SolverConfigError {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SolverConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid solver configuration at {}: {}",
            self.path, self.message
        )
    }
}

impl std::error::Error for SolverConfigError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverConfig {
    engine: SolverEngine,
    executable: PathBuf,
    pin: SolverPin,
    limits: AdapterLimits,
}

impl SolverConfig {
    pub fn new(
        engine: SolverEngine,
        executable: impl Into<PathBuf>,
        pin: SolverPin,
        limits: AdapterLimits,
    ) -> Result<Self, SolverConfigError> {
        let executable = executable.into();
        let limits = limits.validate()?;
        validate_path(&executable, limits.path_bytes)?;
        if pin.version.trim().is_empty()
            || pin.version.as_bytes().contains(&0)
            || pin.version.len() > limits.version_bytes
        {
            return Err(SolverConfigError::new(
                "pin.version",
                "expected version must be non-empty and within the version-output bound",
            ));
        }
        Ok(Self {
            engine,
            executable,
            pin,
            limits,
        })
    }

    pub const fn engine(&self) -> SolverEngine {
        self.engine
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn pin(&self) -> &SolverPin {
        &self.pin
    }

    pub const fn limits(&self) -> AdapterLimits {
        self.limits
    }
}

fn validate_path(path: &Path, maximum_bytes: usize) -> Result<&str, SolverConfigError> {
    if !path.is_absolute() {
        return Err(SolverConfigError::new(
            "executable",
            "solver executable path must be absolute",
        ));
    }
    let text = path.to_str().ok_or_else(|| {
        SolverConfigError::new("executable", "solver executable path must be valid UTF-8")
    })?;
    if text.as_bytes().contains(&0) {
        return Err(SolverConfigError::new(
            "executable",
            "solver executable path contains NUL",
        ));
    }
    if text.len() > maximum_bytes {
        return Err(SolverConfigError::new(
            "executable",
            "solver executable path exceeds the configured byte bound",
        ));
    }
    Ok(text)
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SolverOutcome {
    Sat,
    Unsat,
    Unknown,
    TimedOut,
    Cancelled,
    MissingExecutable,
    SpawnError,
    IdentityError,
    ExecutableDigestMismatch,
    ExecutableChanged,
    VersionMismatch,
    SolverError,
    MalformedOutput,
    ContradictoryOutput,
    NonzeroExit,
    Signaled,
    StdinLimit,
    StdoutLimit,
    StderrLimit,
    DiagnosticOutput,
    ModelLimit,
    CleanupFailed,
    UnsupportedPlatform,
    IoError,
}

impl SolverOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sat => "sat",
            Self::Unsat => "unsat",
            Self::Unknown => "unknown",
            Self::TimedOut => "timed-out",
            Self::Cancelled => "cancelled",
            Self::MissingExecutable => "missing-executable",
            Self::SpawnError => "spawn-error",
            Self::IdentityError => "identity-error",
            Self::ExecutableDigestMismatch => "executable-digest-mismatch",
            Self::ExecutableChanged => "executable-changed",
            Self::VersionMismatch => "version-mismatch",
            Self::SolverError => "solver-error",
            Self::MalformedOutput => "malformed-output",
            Self::ContradictoryOutput => "contradictory-output",
            Self::NonzeroExit => "nonzero-exit",
            Self::Signaled => "signaled",
            Self::StdinLimit => "stdin-limit",
            Self::StdoutLimit => "stdout-limit",
            Self::StderrLimit => "stderr-limit",
            Self::DiagnosticOutput => "diagnostic-output",
            Self::ModelLimit => "model-limit",
            Self::CleanupFailed => "cleanup-failed",
            Self::UnsupportedPlatform => "unsupported-platform",
            Self::IoError => "io-error",
        }
    }

    pub const fn is_conclusive_candidate(self) -> bool {
        matches!(self, Self::Sat | Self::Unsat)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableIdentity {
    pub canonical_path: PathBuf,
    pub byte_length: u64,
    pub sha256: SolverDigest,
    pub version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessExit {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverRecord {
    profile: &'static str,
    engine: SolverEngine,
    outcome: SolverOutcome,
    identity: Option<ExecutableIdentity>,
    argv: Vec<String>,
    configuration_digest: SolverDigest,
    query_digest: String,
    limits: AdapterLimits,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    model: Vec<u8>,
    exit: Option<ProcessExit>,
    elapsed_ms: u64,
    cleanup_ms: u64,
    diagnostic: Option<String>,
}

impl SolverRecord {
    pub const fn profile(&self) -> &'static str {
        self.profile
    }

    pub const fn engine(&self) -> SolverEngine {
        self.engine
    }

    pub const fn outcome(&self) -> SolverOutcome {
        self.outcome
    }

    pub fn identity(&self) -> Option<&ExecutableIdentity> {
        self.identity.as_ref()
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    pub const fn configuration_digest(&self) -> SolverDigest {
        self.configuration_digest
    }

    pub fn query_digest(&self) -> &str {
        &self.query_digest
    }

    pub const fn limits(&self) -> AdapterLimits {
        self.limits
    }

    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    pub fn model(&self) -> &[u8] {
        &self.model
    }

    pub const fn exit(&self) -> Option<ProcessExit> {
        self.exit
    }

    pub const fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }

    pub const fn cleanup_ms(&self) -> u64 {
        self.cleanup_ms
    }

    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    pub const fn is_conclusive_candidate(&self) -> bool {
        self.outcome.is_conclusive_candidate()
    }

    pub fn normalized_outcome_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_field(&mut bytes, SOLVER_PROCESS_PROFILE.as_bytes());
        push_field(&mut bytes, self.engine.as_str().as_bytes());
        push_field(&mut bytes, self.outcome.as_str().as_bytes());
        push_field(&mut bytes, self.configuration_digest.as_bytes());
        push_field(&mut bytes, self.query_digest.as_bytes());
        push_field(&mut bytes, &self.model);
        bytes
    }
}

pub fn execute_solver(
    query: &QueryBundle,
    config: &SolverConfig,
    cancellation: &CancellationToken,
) -> SolverRecord {
    let query_digest = query.query_digest().to_string();
    let argv = config
        .engine
        .query_arguments()
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect::<Vec<_>>();
    let config_digest = configuration_digest(config, &argv);
    let mut record = SolverRecord {
        profile: SOLVER_PROCESS_PROFILE,
        engine: config.engine,
        outcome: SolverOutcome::IdentityError,
        identity: None,
        argv: argv.clone(),
        configuration_digest: config_digest,
        query_digest,
        limits: config.limits,
        stdout: Vec::new(),
        stderr: Vec::new(),
        model: Vec::new(),
        exit: None,
        elapsed_ms: 0,
        cleanup_ms: 0,
        diagnostic: None,
    };

    #[cfg(not(target_os = "linux"))]
    {
        record.outcome = SolverOutcome::UnsupportedPlatform;
        record.diagnostic =
            Some("quire.solver-process/v1 process containment is implemented for Linux".to_owned());
        return record;
    }

    if query.query().len() > config.limits.stdin_bytes {
        record.outcome = SolverOutcome::StdinLimit;
        return record;
    }
    if cancellation.is_cancelled() {
        record.outcome = SolverOutcome::Cancelled;
        return record;
    }

    let (identity, executable_file) = match inspect_executable(config, cancellation) {
        Ok(prepared) => prepared,
        Err(failure) => {
            record.outcome = failure.outcome;
            record.diagnostic = Some(failure.message);
            return record;
        }
    };
    if identity.sha256 != config.pin.executable_sha256 {
        record.outcome = SolverOutcome::ExecutableDigestMismatch;
        record.identity = Some(identity);
        return record;
    }
    if identity.version != config.pin.version {
        record.outcome = SolverOutcome::VersionMismatch;
        record.identity = Some(identity);
        return record;
    }
    let canonical_path = identity.canonical_path.clone();
    record.identity = Some(identity);

    let raw = match run_process(
        &canonical_path,
        &executable_file,
        &argv,
        query.query().as_bytes(),
        config.limits.stdout_bytes,
        config.limits.stderr_bytes,
        config.limits,
        cancellation,
    ) {
        Ok(raw) => raw,
        Err(error) => {
            record.outcome = if error.kind() == io::ErrorKind::NotFound {
                SolverOutcome::MissingExecutable
            } else {
                SolverOutcome::SpawnError
            };
            record.diagnostic = Some(error.to_string());
            return record;
        }
    };
    let executable_integrity = match digest_file(&executable_file) {
        Ok(digest) if digest == record.identity.as_ref().expect("identity").sha256 => None,
        Ok(_) => {
            record.diagnostic = Some("solver executable changed during execution".to_owned());
            Some(SolverOutcome::ExecutableChanged)
        }
        Err(error) => {
            record.diagnostic = Some(error.to_string());
            Some(SolverOutcome::IdentityError)
        }
    };
    let stdout_exceeded = raw.stdout.exceeded;
    let stderr_exceeded = raw.stderr.exceeded;
    record.stdout = raw.stdout.bytes;
    record.stderr = raw.stderr.bytes;
    record.exit = raw.exit.map(process_exit);
    record.elapsed_ms = millis(raw.elapsed);
    record.cleanup_ms = millis(raw.cleanup);

    record.outcome = if !raw.cleanup_ok {
        SolverOutcome::CleanupFailed
    } else if let Some(outcome) = executable_integrity {
        outcome
    } else if raw.cancelled {
        SolverOutcome::Cancelled
    } else if raw.timed_out {
        SolverOutcome::TimedOut
    } else if raw.io_error {
        SolverOutcome::IoError
    } else if stdout_exceeded {
        SolverOutcome::StdoutLimit
    } else if stderr_exceeded {
        SolverOutcome::StderrLimit
    } else if raw.exit.as_ref().is_some_and(|status| !status.success()) {
        if raw.exit.as_ref().and_then(exit_signal).is_some() {
            SolverOutcome::Signaled
        } else {
            SolverOutcome::NonzeroExit
        }
    } else if raw.exit.is_none() {
        SolverOutcome::IoError
    } else if !record.stderr.is_empty() {
        SolverOutcome::DiagnosticOutput
    } else {
        match parse_response(&record.stdout, config.limits.model_bytes) {
            Ok((outcome, model)) => {
                record.model = model;
                outcome
            }
            Err(outcome) => outcome,
        }
    };
    record
}

fn inspect_executable(
    config: &SolverConfig,
    cancellation: &CancellationToken,
) -> Result<(ExecutableIdentity, File), PreflightFailure> {
    let canonical_path = std::fs::canonicalize(&config.executable).map_err(|error| {
        PreflightFailure::new(
            if error.kind() == io::ErrorKind::NotFound {
                SolverOutcome::MissingExecutable
            } else {
                SolverOutcome::IdentityError
            },
            error.to_string(),
        )
    })?;
    validate_path(&canonical_path, config.limits.path_bytes)
        .map_err(|error| PreflightFailure::new(SolverOutcome::IdentityError, error.to_string()))?;
    let executable_file = File::open(&canonical_path)
        .map_err(|error| PreflightFailure::new(SolverOutcome::IdentityError, error.to_string()))?;
    let metadata = executable_file
        .metadata()
        .map_err(|error| PreflightFailure::new(SolverOutcome::IdentityError, error.to_string()))?;
    if !metadata.is_file() {
        return Err(PreflightFailure::new(
            SolverOutcome::IdentityError,
            "solver executable is not a regular file",
        ));
    }
    if metadata.len() > config.limits.executable_bytes {
        return Err(PreflightFailure::new(
            SolverOutcome::IdentityError,
            "solver executable exceeds the configured identity-input bound",
        ));
    }
    let sha256 = digest_file(&executable_file)
        .map_err(|error| PreflightFailure::new(SolverOutcome::IdentityError, error.to_string()))?;

    let version_argv = config
        .engine
        .version_arguments()
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect::<Vec<_>>();
    let raw = run_process(
        &canonical_path,
        &executable_file,
        &version_argv,
        &[],
        config.limits.version_bytes,
        config.limits.stderr_bytes,
        config.limits,
        cancellation,
    )
    .map_err(|error| {
        PreflightFailure::new(
            if error.kind() == io::ErrorKind::NotFound {
                SolverOutcome::MissingExecutable
            } else {
                SolverOutcome::SpawnError
            },
            error.to_string(),
        )
    })?;
    let failure = if !raw.cleanup_ok {
        Some(SolverOutcome::CleanupFailed)
    } else if raw.cancelled {
        Some(SolverOutcome::Cancelled)
    } else if raw.timed_out {
        Some(SolverOutcome::TimedOut)
    } else if raw.io_error {
        Some(SolverOutcome::IoError)
    } else if raw.stdout.exceeded
        || raw.stderr.exceeded
        || raw.exit.as_ref().map_or(true, |status| !status.success())
    {
        Some(SolverOutcome::IdentityError)
    } else {
        None
    };
    if let Some(outcome) = failure {
        return Err(PreflightFailure::new(
            outcome,
            "bounded solver version probe failed",
        ));
    }
    let version = std::str::from_utf8(&raw.stdout.bytes)
        .map_err(|_| {
            PreflightFailure::new(
                SolverOutcome::IdentityError,
                "solver version output is not UTF-8",
            )
        })?
        .trim()
        .to_owned();
    if version.is_empty() || version.as_bytes().contains(&0) {
        return Err(PreflightFailure::new(
            SolverOutcome::IdentityError,
            "solver version output is empty or contains NUL",
        ));
    }
    let post_version_digest = digest_file(&executable_file)
        .map_err(|error| PreflightFailure::new(SolverOutcome::IdentityError, error.to_string()))?;
    if post_version_digest != sha256 {
        return Err(PreflightFailure::new(
            SolverOutcome::ExecutableChanged,
            "solver executable changed during version probing",
        ));
    }
    Ok((
        ExecutableIdentity {
            canonical_path,
            byte_length: metadata.len(),
            sha256,
            version,
        },
        executable_file,
    ))
}

struct PreflightFailure {
    outcome: SolverOutcome,
    message: String,
}

impl PreflightFailure {
    fn new(outcome: SolverOutcome, message: impl Into<String>) -> Self {
        Self {
            outcome,
            message: message.into(),
        }
    }
}

fn digest_file(file: &File) -> io::Result<SolverDigest> {
    let mut file = file.try_clone()?;
    file.seek(SeekFrom::Start(0))?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut hasher = Sha256::new();
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(SolverDigest(hasher.finalize().into()))
}

#[derive(Default)]
struct Capture {
    bytes: Vec<u8>,
    exceeded: bool,
    eof: bool,
}

struct RawProcess {
    stdout: Capture,
    stderr: Capture,
    exit: Option<ExitStatus>,
    elapsed: Duration,
    cleanup: Duration,
    cleanup_ok: bool,
    timed_out: bool,
    cancelled: bool,
    io_error: bool,
}

#[cfg(target_os = "linux")]
fn run_process(
    executable: &Path,
    executable_file: &File,
    arguments: &[String],
    input: &[u8],
    stdout_limit: usize,
    stderr_limit: usize,
    limits: AdapterLimits,
    cancellation: &CancellationToken,
) -> io::Result<RawProcess> {
    let argv = std::iter::once(executable.to_str().expect("validated UTF-8 path"))
        .chain(arguments.iter().map(String::as_str))
        .map(|argument| {
            CString::new(argument)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "argv contains NUL"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let environment = [
        CString::new("LANG=C").expect("static environment"),
        CString::new("LC_ALL=C").expect("static environment"),
    ];
    let argv_pointers = argv
        .iter()
        .map(|argument| argument.as_ptr() as usize)
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let environment_pointers = environment
        .iter()
        .map(|variable| variable.as_ptr() as usize)
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let executable_file = executable_file.try_clone()?;
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .env_clear()
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: the closure uses only async-signal-safe libc operations, points argv/environment at
    // immutable captured C strings, creates an isolated process group, and either replaces the child
    // with the already-hashed open executable or returns the exact OS error to Command::spawn.
    unsafe {
        command.pre_exec(move || {
            if libc::setpgid(0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            let descriptor = executable_file.as_raw_fd();
            let flags = libc::fcntl(descriptor, libc::F_GETFD);
            if flags < 0 || libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                return Err(io::Error::last_os_error());
            }
            libc::fexecve(
                descriptor,
                argv_pointers.as_ptr().cast(),
                environment_pointers.as_ptr().cast(),
            );
            Err(io::Error::last_os_error())
        });
    }
    let mut child = command.spawn()?;
    let process_group = match i32::try_from(child.id()) {
        Ok(process_group) => process_group,
        Err(_) => {
            abort_spawned_child(&mut child, None);
            return Err(io::Error::other(
                "child pid does not fit a POSIX process-group id",
            ));
        }
    };
    let mut stdin = Some(child.stdin.take().expect("piped stdin"));
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let nonblocking = set_nonblocking(stdin.as_ref().expect("piped stdin").as_raw_fd())
        .and_then(|()| set_nonblocking(stdout.as_raw_fd()))
        .and_then(|()| set_nonblocking(stderr.as_raw_fd()));
    if let Err(error) = nonblocking {
        abort_spawned_child(&mut child, Some(process_group));
        return Err(error);
    }

    let started = Instant::now();
    let wall = Duration::from_millis(limits.wall_time_ms);
    let interval = Duration::from_millis(limits.monitor_interval_ms);
    let mut input_offset = 0;
    let mut stdout_capture = Capture::default();
    let mut stderr_capture = Capture::default();
    let mut exit = None;
    let mut timed_out = false;
    let mut cancelled = false;
    let mut io_error = false;

    loop {
        if input_offset < input.len() {
            match stdin
                .as_mut()
                .expect("stdin remains open while input is pending")
                .write(&input[input_offset..])
            {
                Ok(0) => {
                    stdin.take();
                    input_offset = input.len();
                }
                Ok(written) => input_offset += written,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                    stdin.take();
                    input_offset = input.len();
                }
                Err(_) => {
                    io_error = true;
                    break;
                }
            }
        } else {
            stdin.take();
        }
        drain(
            &mut stdout,
            &mut stdout_capture,
            stdout_limit,
            &mut io_error,
        );
        drain(
            &mut stderr,
            &mut stderr_capture,
            stderr_limit,
            &mut io_error,
        );
        if io_error {
            break;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                exit = Some(status);
                break;
            }
            Ok(None) => {}
            Err(_) => {
                io_error = true;
                break;
            }
        }
        if cancellation.is_cancelled() {
            cancelled = true;
            break;
        }
        if started.elapsed() >= wall {
            timed_out = true;
            break;
        }
        thread::sleep(interval);
    }
    stdin.take();

    let cleanup_started = Instant::now();
    let cleanup_deadline = cleanup_started + Duration::from_millis(limits.cleanup_time_ms);
    let graceful_deadline = cleanup_started + Duration::from_millis(limits.graceful_cleanup_ms);
    if signal_group(process_group, libc::SIGTERM).is_err() {
        io_error = true;
    }
    while group_alive(process_group) && Instant::now() < graceful_deadline {
        if exit.is_none() {
            match child.try_wait() {
                Ok(status) => exit = status,
                Err(_) => io_error = true,
            }
        }
        drain(
            &mut stdout,
            &mut stdout_capture,
            stdout_limit,
            &mut io_error,
        );
        drain(
            &mut stderr,
            &mut stderr_capture,
            stderr_limit,
            &mut io_error,
        );
        thread::sleep(interval);
    }
    if group_alive(process_group) && signal_group(process_group, libc::SIGKILL).is_err() {
        io_error = true;
    }
    while group_alive(process_group) && Instant::now() < cleanup_deadline {
        if exit.is_none() {
            match child.try_wait() {
                Ok(status) => exit = status,
                Err(_) => io_error = true,
            }
        }
        drain(
            &mut stdout,
            &mut stdout_capture,
            stdout_limit,
            &mut io_error,
        );
        drain(
            &mut stderr,
            &mut stderr_capture,
            stderr_limit,
            &mut io_error,
        );
        thread::sleep(interval);
    }
    if exit.is_none() {
        match child.try_wait() {
            Ok(status) => exit = status,
            Err(_) => io_error = true,
        }
    }
    if exit.is_none() && !group_alive(process_group) {
        match child.wait() {
            Ok(status) => exit = Some(status),
            Err(_) => io_error = true,
        }
    }
    drain(
        &mut stdout,
        &mut stdout_capture,
        stdout_limit,
        &mut io_error,
    );
    drain(
        &mut stderr,
        &mut stderr_capture,
        stderr_limit,
        &mut io_error,
    );
    let cleanup = cleanup_started.elapsed();
    Ok(RawProcess {
        stdout: stdout_capture,
        stderr: stderr_capture,
        exit,
        elapsed: started.elapsed(),
        cleanup,
        cleanup_ok: !group_alive(process_group)
            && cleanup <= Duration::from_millis(limits.cleanup_time_ms),
        timed_out,
        cancelled,
        io_error,
    })
}

#[cfg(not(target_os = "linux"))]
fn run_process(
    _executable: &Path,
    _executable_file: &File,
    _arguments: &[String],
    _input: &[u8],
    _stdout_limit: usize,
    _stderr_limit: usize,
    _limits: AdapterLimits,
    _cancellation: &CancellationToken,
) -> io::Result<RawProcess> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "quire.solver-process/v1 process containment is implemented for Linux",
    ))
}

#[cfg(target_os = "linux")]
fn abort_spawned_child(child: &mut Child, process_group: Option<i32>) {
    if let Some(process_group) = process_group {
        let _ = signal_group(process_group, libc::SIGKILL);
    } else {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(target_os = "linux")]
fn set_nonblocking(file_descriptor: i32) -> io::Result<()> {
    // SAFETY: fcntl is called with a valid owned pipe descriptor and commands that do not outlive it.
    let flags = unsafe { libc::fcntl(file_descriptor, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the same valid descriptor is updated by preserving its flags and adding O_NONBLOCK.
    if unsafe { libc::fcntl(file_descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn drain<R: Read>(reader: &mut R, capture: &mut Capture, limit: usize, io_error: &mut bool) {
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                capture.eof = true;
                return;
            }
            Ok(read) => {
                let remaining = limit.saturating_sub(capture.bytes.len());
                let retained = remaining.min(read);
                capture.bytes.extend_from_slice(&buffer[..retained]);
                capture.exceeded |= retained < read;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
            Err(_) => {
                *io_error = true;
                return;
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn signal_group(process_group: i32, signal: i32) -> io::Result<()> {
    // SAFETY: negative process_group intentionally addresses the isolated child process group.
    let result = unsafe { libc::kill(-process_group, signal) };
    if result == 0 {
        Ok(())
    } else {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(())
        } else {
            Err(error)
        }
    }
}

#[cfg(target_os = "linux")]
fn group_alive(process_group: i32) -> bool {
    // SAFETY: signal 0 performs a non-mutating existence/permission probe on the isolated group.
    let result = unsafe { libc::kill(-process_group, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn parse_response(
    bytes: &[u8],
    model_limit: usize,
) -> Result<(SolverOutcome, Vec<u8>), SolverOutcome> {
    let text = std::str::from_utf8(bytes).map_err(|_| SolverOutcome::MalformedOutput)?;
    let text = text.trim_start();
    if text.is_empty() || text.as_bytes().contains(&0) {
        return Err(SolverOutcome::MalformedOutput);
    }
    if text.starts_with("(error") && valid_s_expression(text.trim().as_bytes()) {
        return Err(SolverOutcome::SolverError);
    }
    let (head, raw_tail) = text
        .find(char::is_whitespace)
        .map_or((text, ""), |index| (&text[..index], &text[index..]));
    let tail = raw_tail.trim();
    let outcome = match head {
        "sat" => SolverOutcome::Sat,
        "unsat" => SolverOutcome::Unsat,
        "unknown" => SolverOutcome::Unknown,
        _ => return Err(SolverOutcome::MalformedOutput),
    };
    if tail
        .split_whitespace()
        .any(|token| matches!(token, "sat" | "unsat" | "unknown"))
    {
        return Err(SolverOutcome::ContradictoryOutput);
    }
    if tail.is_empty() {
        return Ok((outcome, Vec::new()));
    }
    if tail.starts_with("(error") && valid_s_expression(tail.as_bytes()) {
        return Err(SolverOutcome::SolverError);
    }
    if outcome != SolverOutcome::Sat {
        return Err(SolverOutcome::MalformedOutput);
    }
    if raw_tail.len() > model_limit {
        return Err(SolverOutcome::ModelLimit);
    }
    if !valid_s_expression(tail.as_bytes()) {
        return Err(SolverOutcome::MalformedOutput);
    }
    Ok((outcome, tail.as_bytes().to_vec()))
}

fn valid_s_expression(bytes: &[u8]) -> bool {
    let mut depth = 0_u64;
    let mut saw_list = false;
    let mut string = false;
    let mut quoted = false;
    let mut comment = false;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if comment {
            comment = byte != b'\n';
        } else if string {
            if byte == b'"' {
                if bytes.get(index + 1) == Some(&b'"') {
                    index += 1;
                } else {
                    string = false;
                }
            }
        } else if quoted {
            if byte == b'|' {
                quoted = false;
            } else if byte == b'\\' {
                return false;
            }
        } else {
            match byte {
                b';' => comment = true,
                b'"' => string = true,
                b'|' => quoted = true,
                b'(' => {
                    if depth == 0 && saw_list {
                        return false;
                    }
                    depth = depth.saturating_add(1);
                    saw_list = true;
                }
                b')' if depth == 0 => return false,
                b')' => depth -= 1,
                _ if depth == 0 && !byte.is_ascii_whitespace() => return false,
                _ => {}
            }
        }
        index += 1;
    }
    saw_list && depth == 0 && !string && !quoted
}

fn configuration_digest(config: &SolverConfig, argv: &[String]) -> SolverDigest {
    let mut fields = Vec::new();
    push_field(&mut fields, config.engine.as_str().as_bytes());
    push_field(&mut fields, config.executable.to_string_lossy().as_bytes());
    push_field(&mut fields, config.pin.version.as_bytes());
    push_field(&mut fields, config.pin.executable_sha256.as_bytes());
    for argument in argv {
        push_field(&mut fields, argument.as_bytes());
    }
    for value in [
        config.limits.wall_time_ms,
        config.limits.cleanup_time_ms,
        config.limits.graceful_cleanup_ms,
        config.limits.monitor_interval_ms,
        config.limits.stdin_bytes as u64,
        config.limits.stdout_bytes as u64,
        config.limits.stderr_bytes as u64,
        config.limits.model_bytes as u64,
        config.limits.version_bytes as u64,
        config.limits.executable_bytes,
        config.limits.path_bytes as u64,
    ] {
        push_field(&mut fields, &value.to_be_bytes());
    }
    hash_fields(
        "configuration",
        &[SOLVER_PROCESS_PROFILE.as_bytes(), &fields],
    )
}

fn hash_fields(domain: &str, fields: &[&[u8]]) -> SolverDigest {
    let mut hasher = Sha256::new();
    hasher.update(b"quire-analyze\0");
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    SolverDigest(hasher.finalize().into())
}

fn push_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u64).to_be_bytes());
    output.extend_from_slice(field);
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn process_exit(status: ExitStatus) -> ProcessExit {
    ProcessExit {
        code: status.code(),
        signal: exit_signal(&status),
    }
}

#[cfg(target_os = "linux")]
fn exit_signal(status: &ExitStatus) -> Option<i32> {
    status.signal()
}

#[cfg(not(target_os = "linux"))]
fn exit_signal(_status: &ExitStatus) -> Option<i32> {
    None
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::{
        ffi::OsString,
        io::{Cursor, Error, ErrorKind, Read},
        os::unix::{ffi::OsStringExt, process::CommandExt},
        process::Command,
    };

    use super::*;

    #[test]
    fn response_parser_rejects_every_non_exact_shape() {
        assert_eq!(parse_response(b"sat", 0), Ok((SolverOutcome::Sat, vec![])));
        assert_eq!(
            parse_response(b"unsat", 0),
            Ok((SolverOutcome::Unsat, vec![]))
        );
        assert_eq!(
            parse_response(b"unknown", 0),
            Ok((SolverOutcome::Unknown, vec![]))
        );
        assert_eq!(parse_response(b"", 1), Err(SolverOutcome::MalformedOutput));
        assert_eq!(
            parse_response(b"sat\0", 1),
            Err(SolverOutcome::MalformedOutput)
        );
        assert_eq!(
            parse_response(&[0xff], 1),
            Err(SolverOutcome::MalformedOutput)
        );
        assert_eq!(
            parse_response(b"unsat (model)", 100),
            Err(SolverOutcome::MalformedOutput)
        );
        assert_eq!(
            parse_response(b"sat unknown", 100),
            Err(SolverOutcome::ContradictoryOutput)
        );
        let model = b"(model (define-fun |x y| () String \"a\"\"b\")) ; comment";
        assert_eq!(
            parse_response(&[b"sat\n".as_slice(), model].concat(), model.len() + 1),
            Ok((SolverOutcome::Sat, model.to_vec()))
        );
        assert_eq!(
            parse_response(&[b"sat\n".as_slice(), model].concat(), model.len()),
            Err(SolverOutcome::ModelLimit)
        );
        for malformed in [
            b"sat\n(model".as_slice(),
            b"sat\n(model))".as_slice(),
            b"sat\n(model)(other)".as_slice(),
            b"sat\natom".as_slice(),
            b"sat\n(|bad\\symbol|)".as_slice(),
            b"sat\n(\"unterminated)".as_slice(),
            b"sat\n(|unterminated)".as_slice(),
        ] {
            assert_eq!(
                parse_response(malformed, 1_000),
                Err(SolverOutcome::MalformedOutput)
            );
        }
    }

    #[test]
    fn public_outcome_and_configuration_censuses_are_closed() {
        let outcomes = [
            SolverOutcome::Sat,
            SolverOutcome::Unsat,
            SolverOutcome::Unknown,
            SolverOutcome::TimedOut,
            SolverOutcome::Cancelled,
            SolverOutcome::MissingExecutable,
            SolverOutcome::SpawnError,
            SolverOutcome::IdentityError,
            SolverOutcome::ExecutableDigestMismatch,
            SolverOutcome::ExecutableChanged,
            SolverOutcome::VersionMismatch,
            SolverOutcome::SolverError,
            SolverOutcome::MalformedOutput,
            SolverOutcome::ContradictoryOutput,
            SolverOutcome::NonzeroExit,
            SolverOutcome::Signaled,
            SolverOutcome::StdinLimit,
            SolverOutcome::StdoutLimit,
            SolverOutcome::StderrLimit,
            SolverOutcome::DiagnosticOutput,
            SolverOutcome::ModelLimit,
            SolverOutcome::CleanupFailed,
            SolverOutcome::UnsupportedPlatform,
            SolverOutcome::IoError,
        ];
        let names = outcomes.map(SolverOutcome::as_str);
        assert_eq!(
            names
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            outcomes.len()
        );

        let digest = SolverDigest::from_bytes([0x5a; 32]);
        let pin = SolverPin::new("solver 1", digest);
        let config = SolverConfig::new(
            SolverEngine::Z3,
            "/tmp/solver",
            pin.clone(),
            AdapterLimits::default(),
        )
        .expect("syntactically valid config");
        assert_eq!(config.engine(), SolverEngine::Z3);
        assert_eq!(config.executable(), Path::new("/tmp/solver"));
        assert_eq!(config.pin(), &pin);
        assert_eq!(config.limits(), AdapterLimits::default());
        assert_eq!(pin.version(), "solver 1");
        assert_eq!(pin.executable_sha256(), digest);
        assert_eq!(digest.as_bytes(), &[0x5a; 32]);

        for limits in [
            AdapterLimits {
                cleanup_time_ms: 1,
                graceful_cleanup_ms: 2,
                ..AdapterLimits::default()
            },
            AdapterLimits {
                stdout_bytes: 1,
                model_bytes: 2,
                ..AdapterLimits::default()
            },
        ] {
            let error = limits.validate().expect_err("relational limit");
            assert!(error.path().starts_with("limits."));
            assert!(!error.message().is_empty());
            assert!(error.to_string().contains(error.path()));
        }
        for version in [String::new(), "x".repeat(MAX_SOLVER_VERSION_BYTES + 1)] {
            let error = SolverConfig::new(
                SolverEngine::Z3,
                "/tmp/solver",
                SolverPin::new(version, digest),
                AdapterLimits::default(),
            )
            .expect_err("invalid version pin");
            assert_eq!(error.path(), "pin.version");
        }
        let non_utf8 = PathBuf::from(OsString::from_vec(b"/tmp/solver-\xff".to_vec()));
        assert_eq!(
            SolverConfig::new(SolverEngine::Z3, non_utf8, pin, AdapterLimits::default(),)
                .expect_err("non-UTF-8 path")
                .path(),
            "executable"
        );
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(Error::new(ErrorKind::InvalidData, "injected read failure"))
        }
    }

    #[test]
    fn low_level_io_and_cleanup_helpers_fail_closed() {
        let mut capture = Capture::default();
        let mut io_error = false;
        drain(&mut Cursor::new(b"abcdef"), &mut capture, 3, &mut io_error);
        assert_eq!(capture.bytes, b"abc");
        assert!(capture.exceeded);
        assert!(capture.eof);
        assert!(!io_error);

        drain(&mut FailingReader, &mut capture, 3, &mut io_error);
        assert!(io_error);
        assert!(set_nonblocking(-1).is_err());
        assert!(!group_alive(i32::MAX));
        assert!(signal_group(i32::MAX, libc::SIGTERM).is_ok());

        let mut direct = Command::new("/bin/sleep").arg("30").spawn().expect("sleep");
        abort_spawned_child(&mut direct, None);
        assert!(direct.try_wait().expect("wait").is_some());

        let mut grouped = Command::new("/bin/sleep");
        grouped.arg("30").process_group(0);
        let mut grouped = grouped.spawn().expect("grouped sleep");
        let process_group = i32::try_from(grouped.id()).expect("pid");
        abort_spawned_child(&mut grouped, Some(process_group));
        assert!(!group_alive(process_group));
    }
}
