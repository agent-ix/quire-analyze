//! `producer-execute` wire documents and the `SolverRecord` they evidence.
//!
//! `producer-execute` is a separate executable reached over a process boundary
//! (FR-009). This module holds the minimal typed wire documents for its
//! `producer-execution-request/v1` and `producer-execution-result/v1`
//! protocols, builds the version and query requests, parses the single result
//! line, and turns a result into a [`SolverRecord`] by the FR-009 result-state
//! table.

use std::{collections::BTreeMap, fmt, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    solver::classify_exit, ExecutableIdentity, ProcessExit, QueryBundle, SolverConfig,
    SolverOutcome, SolverRecord, SOLVER_PROCESS_PROFILE,
};

/// Protocol constant of a `producer-execute` request document.
pub const PRODUCER_REQUEST_PROTOCOL: &str = "engineering-assurance.producer-execution-request/v1";
/// Protocol constant of a `producer-execute` result document.
pub const PRODUCER_RESULT_PROTOCOL: &str = "engineering-assurance.producer-execution-result/v1";
/// `producer.version` of a version invocation, whose version is not yet known.
pub const UNPROBED_VERSION: &str = "unprobed";
/// Role of the staged query artifact bound to the solver's stdin.
pub const QUERY_ROLE: &str = "query";
/// Stdout and stderr ceiling of `producer-execute` (`MAX_STREAM_BYTES`).
pub const MAX_PRODUCER_STREAM_BYTES: usize = 8_388_608;

const CALLER_KIND: &str = "quire-analyze.run-package";
const CONTAINMENT_KIND: &str = "quire.solver-process";
const RESPONSE_PROTOCOL_KIND: &str = "engineering-assurance.process-evidence";
const RESPONSE_ADAPTER_KIND: &str = "engineering-assurance.cli-process-evidence-adapter";
const MAX_DESCENDANTS: u64 = 16;
const MAX_CONCURRENCY: u64 = 1;

/// Why building a request or reading a result line failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceErrorKind {
    /// A path or text field cannot be carried in a request.
    Request,
    /// The producer-execute stdout is not exactly one result document.
    ResultLine,
    /// The result line is not a valid `producer-execution-result/v1` document.
    ResultDocument,
}

/// A request that cannot be built or a result line that cannot be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceError {
    kind: EvidenceErrorKind,
    message: String,
}

impl EvidenceError {
    fn new(kind: EvidenceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// The stable failure code.
    pub const fn kind(&self) -> EvidenceErrorKind {
        self.kind
    }

    /// The human-readable cause.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for EvidenceError {}

#[derive(Clone, Debug, Serialize)]
struct Contract {
    kind: &'static str,
    version: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct RequestProducer {
    name: &'static str,
    version: String,
    executable: String,
}

#[derive(Clone, Debug, Serialize)]
struct LiteralArgument {
    kind: &'static str,
    value: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct RequestInput {
    role: &'static str,
    path: String,
}

#[derive(Clone, Debug, Serialize)]
struct RequestStdin {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
struct Containment {
    profile: &'static str,
    contract: Contract,
}

#[derive(Clone, Debug, Serialize)]
struct Cancellation {
    kind: &'static str,
}

/// An output description; uninhabited because the budget allows no output artifacts.
#[derive(Clone, Debug, Serialize)]
enum NoOutput {}

#[derive(Clone, Debug, Serialize)]
struct ExitCodes {
    kind: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Budget {
    timeout_millis: u64,
    max_stdout_bytes: u64,
    max_stderr_bytes: u64,
    max_input_bytes: u64,
    max_output_artifacts: u64,
    max_output_bytes: u64,
    max_descendants: u64,
    max_concurrency: u64,
}

#[derive(Clone, Debug, Serialize)]
struct RequestResponse {
    protocol: Contract,
    adapter: Contract,
    #[serde(rename = "exitCodes")]
    exit_codes: ExitCodes,
}

/// One `engineering-assurance.producer-execution-request/v1` document.
///
/// Built only by [`version_request`] and [`query_request`]; serialize it with
/// [`ProducerExecutionRequest::to_json_bytes`] and pass it to
/// `producer-execute --request -`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerExecutionRequest {
    protocol: &'static str,
    producer: RequestProducer,
    caller: Contract,
    procedure: &'static str,
    capability_root: String,
    arguments: Vec<LiteralArgument>,
    environment: BTreeMap<String, String>,
    inputs: Vec<RequestInput>,
    stdin: RequestStdin,
    /// Always empty: the budget allows no output artifacts, so no output is describable.
    outputs: Vec<NoOutput>,
    containment: Containment,
    cancellation: Cancellation,
    budget: Budget,
    response: RequestResponse,
}

impl ProducerExecutionRequest {
    /// Serialize the request as the JSON document `producer-execute` reads.
    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a request of strings and integers serializes")
    }
}

fn path_text<'a>(path: &'a Path, what: &str) -> Result<&'a str, EvidenceError> {
    match path.to_str() {
        Some(text) if path.is_absolute() && !text.is_empty() => Ok(text),
        _ => Err(EvidenceError::new(
            EvidenceErrorKind::Request,
            format!("{what} must be an absolute UTF-8 path"),
        )),
    }
}

fn relative_path_text<'a>(path: &'a Path, what: &str) -> Result<&'a str, EvidenceError> {
    let normal = path
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_)));
    match path.to_str() {
        Some(text) if normal && !text.is_empty() => Ok(text),
        _ => Err(EvidenceError::new(
            EvidenceErrorKind::Request,
            format!("{what} must be a normal relative UTF-8 path"),
        )),
    }
}

fn budget(config: &SolverConfig) -> Budget {
    let limits = config.limits();
    let capped = |bytes: usize| -> u64 {
        u64::try_from(bytes.min(MAX_PRODUCER_STREAM_BYTES))
            .unwrap_or(MAX_PRODUCER_STREAM_BYTES as u64)
    };
    Budget {
        timeout_millis: limits.wall_time_ms,
        max_stdout_bytes: capped(limits.stdout_bytes),
        max_stderr_bytes: capped(limits.stderr_bytes),
        max_input_bytes: u64::try_from(limits.stdin_bytes).unwrap_or(u64::MAX),
        max_output_artifacts: 0,
        max_output_bytes: 0,
        max_descendants: MAX_DESCENDANTS,
        max_concurrency: MAX_CONCURRENCY,
    }
}

fn build_request(
    config: &SolverConfig,
    capability_root: &Path,
    version: &str,
    arguments: &[&'static str],
    query_path: Option<&Path>,
) -> Result<ProducerExecutionRequest, EvidenceError> {
    let executable = path_text(config.executable(), "solver executable")?.to_owned();
    let capability_root = path_text(capability_root, "capability root")?.to_owned();
    let (inputs, stdin) = match query_path {
        Some(path) => (
            vec![RequestInput {
                role: QUERY_ROLE,
                path: relative_path_text(path, "staged query")?.to_owned(),
            }],
            RequestStdin {
                kind: "input_artifact",
                role: Some(QUERY_ROLE),
            },
        ),
        None => (
            Vec::new(),
            RequestStdin {
                kind: "null",
                role: None,
            },
        ),
    };
    Ok(ProducerExecutionRequest {
        protocol: PRODUCER_REQUEST_PROTOCOL,
        producer: RequestProducer {
            name: config.engine().as_str(),
            version: version.to_owned(),
            executable,
        },
        caller: Contract {
            kind: CALLER_KIND,
            version: "1",
        },
        procedure: "direct",
        capability_root,
        arguments: arguments
            .iter()
            .map(|value| LiteralArgument {
                kind: "literal",
                value,
            })
            .collect(),
        environment: BTreeMap::new(),
        inputs,
        stdin,
        outputs: Vec::new(),
        containment: Containment {
            profile: "process-group-v1",
            contract: Contract {
                kind: CONTAINMENT_KIND,
                version: "v1",
            },
        },
        cancellation: Cancellation { kind: "disabled" },
        budget: budget(config),
        response: RequestResponse {
            protocol: Contract {
                kind: RESPONSE_PROTOCOL_KIND,
                version: "1",
            },
            adapter: Contract {
                kind: RESPONSE_ADAPTER_KIND,
                version: "1",
            },
            exit_codes: ExitCodes { kind: "any" },
        },
    })
}

/// The version invocation of an engine: `producer.version` `unprobed`, stdin `null`, no inputs.
pub fn version_request(
    config: &SolverConfig,
    capability_root: &Path,
) -> Result<ProducerExecutionRequest, EvidenceError> {
    build_request(
        config,
        capability_root,
        UNPROBED_VERSION,
        config.engine().version_arguments(),
        None,
    )
}

/// The query invocation of an engine, carrying the trimmed `version` text of its version
/// invocation and the lowered query staged at `query_file`, a normal path relative to the capability
/// root, as the `query` input bound to stdin.
pub fn query_request(
    config: &SolverConfig,
    capability_root: &Path,
    version: &str,
    query_file: &Path,
) -> Result<ProducerExecutionRequest, EvidenceError> {
    if version.is_empty() {
        return Err(EvidenceError::new(
            EvidenceErrorKind::Request,
            "the version text of a query request must not be empty",
        ));
    }
    build_request(
        config,
        capability_root,
        version,
        config.engine().query_arguments(),
        Some(query_file),
    )
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultProducer {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: String,
    executable: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum StatusKind {
    ExitCode,
    Signal,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminalStatus {
    kind: StatusKind,
    value: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stream {
    bytes: Vec<u8>,
    truncated: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ProcessEvidence {
    #[serde(deserialize_with = "Option::deserialize")]
    terminal_status: Option<TerminalStatus>,
    stdout: Stream,
    stderr: Stream,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Timing {
    #[allow(dead_code)]
    preflight_nanos: u64,
    #[serde(deserialize_with = "Option::deserialize")]
    execution_nanos: Option<u64>,
    #[allow(dead_code)]
    total_nanos: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ResultArtifact {
    #[allow(dead_code)]
    role: String,
    #[allow(dead_code)]
    path: String,
    #[allow(dead_code)]
    byte_length: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RefusalReason {
    Executable,
    CapabilityRoot,
    Input,
    Output,
    AdapterBinding,
    CancellationBinding,
    Concurrency,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum FailureReason {
    ExitCode { value: i64 },
    Signal { value: i64 },
    StdoutTooLarge {},
    StderrTooLarge {},
    Observation {},
    OutputArtifact {},
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Observation {
    exit_code: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum State {
    Unavailable {},
    TimedOut {},
    MalformedResponse {},
    ContainmentFailure {},
    Cancelled {},
    Refused {
        #[allow(dead_code)]
        reason: RefusalReason,
    },
    Failed {
        reason: FailureReason,
    },
    Completed {
        observation: Observation,
    },
}

/// One `engineering-assurance.producer-execution-result/v1` document.
///
/// Obtain it with [`parse_result_line`].
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProducerExecutionResult {
    protocol: String,
    producer: ResultProducer,
    #[allow(dead_code)]
    cancellation_event: Value,
    timing: Timing,
    #[allow(dead_code)]
    observed_host: Value,
    #[serde(deserialize_with = "Option::deserialize")]
    process: Option<ProcessEvidence>,
    #[allow(dead_code)]
    artifacts: Vec<ResultArtifact>,
    state: State,
}

/// Read the single result line `producer-execute` writes to stdout.
///
/// The bytes are exactly one JSON document and one line terminator; anything else, an unknown
/// field, a wrong protocol, or a `completed` state without retained process evidence is an error.
pub fn parse_result_line(stdout: &[u8]) -> Result<ProducerExecutionResult, EvidenceError> {
    let Some(line) = stdout.strip_suffix(b"\n") else {
        return Err(EvidenceError::new(
            EvidenceErrorKind::ResultLine,
            "producer-execute stdout must end with a newline-terminated result line",
        ));
    };
    if line.is_empty() || line.contains(&b'\n') {
        return Err(EvidenceError::new(
            EvidenceErrorKind::ResultLine,
            "producer-execute stdout must be exactly one result line",
        ));
    }
    let result: ProducerExecutionResult = serde_json::from_slice(line).map_err(|error| {
        EvidenceError::new(
            EvidenceErrorKind::ResultDocument,
            format!("not a producer-execution-result/v1 document: {error}"),
        )
    })?;
    if result.protocol != PRODUCER_RESULT_PROTOCOL {
        return Err(EvidenceError::new(
            EvidenceErrorKind::ResultDocument,
            "result protocol is not producer-execution-result/v1",
        ));
    }
    if matches!(result.state, State::Completed { .. }) && result.process.is_none() {
        return Err(EvidenceError::new(
            EvidenceErrorKind::ResultDocument,
            "a completed result must retain process evidence",
        ));
    }
    Ok(result)
}

impl ProducerExecutionResult {
    /// Whether the state is `completed`.
    pub const fn is_completed(&self) -> bool {
        matches!(self.state, State::Completed { .. })
    }
}

/// The engine outcome of a result state that is not `completed`, by the FR-009 table.
fn state_outcome(state: State, host_is_linux: bool) -> Option<SolverOutcome> {
    Some(match state {
        State::Completed { .. } => return None,
        State::Unavailable {} => SolverOutcome::MissingExecutable,
        State::TimedOut {} => SolverOutcome::TimedOut,
        State::Failed { reason } => match reason {
            FailureReason::ExitCode { .. } => SolverOutcome::NonzeroExit,
            FailureReason::Signal { .. } => SolverOutcome::Signaled,
            FailureReason::StdoutTooLarge {} => SolverOutcome::StdoutLimit,
            FailureReason::StderrTooLarge {} => SolverOutcome::StderrLimit,
            FailureReason::Observation {} | FailureReason::OutputArtifact {} => {
                SolverOutcome::IoError
            }
        },
        State::Refused { .. } => SolverOutcome::SpawnError,
        State::MalformedResponse {} => SolverOutcome::MalformedOutput,
        State::ContainmentFailure {} if host_is_linux => SolverOutcome::CleanupFailed,
        State::ContainmentFailure {} => SolverOutcome::UnsupportedPlatform,
        State::Cancelled {} => SolverOutcome::Cancelled,
    })
}

/// The trimmed version text of a version invocation, or the engine outcome that replaces the
/// query invocation when the version run is not usable.
///
/// A state other than `completed` gives its table outcome. A completed run whose exit is not `0`,
/// whose stdout is truncated, over `limits.version_bytes`, not UTF-8, contains NUL, or is empty
/// after trimming gives `identity-error`.
pub fn version_text(
    config: &SolverConfig,
    version: &ProducerExecutionResult,
) -> Result<String, SolverOutcome> {
    version_text_on_host(config, version, cfg!(target_os = "linux"))
}

fn version_text_on_host(
    config: &SolverConfig,
    version: &ProducerExecutionResult,
    host_is_linux: bool,
) -> Result<String, SolverOutcome> {
    if let Some(outcome) = state_outcome(version.state, host_is_linux) {
        return Err(outcome);
    }
    let exited_zero =
        matches!(version.state, State::Completed { observation } if observation.exit_code == 0);
    let process = version
        .process
        .as_ref()
        .ok_or(SolverOutcome::IdentityError)?;
    if !exited_zero
        || process.stdout.truncated
        || process.stderr.truncated
        || process.stdout.bytes.len() > config.limits().version_bytes
    {
        return Err(SolverOutcome::IdentityError);
    }
    let text = std::str::from_utf8(&process.stdout.bytes)
        .map_err(|_| SolverOutcome::IdentityError)?
        .trim();
    if text.is_empty() || text.as_bytes().contains(&0) {
        return Err(SolverOutcome::IdentityError);
    }
    Ok(text.to_owned())
}

/// Build the [`SolverRecord`] of one engine from its `producer-execute` results.
///
/// `version` is the result of the version invocation. `query_result` is the result of the query
/// invocation, absent when the version run was not usable and the query invocation was skipped.
/// Every result state maps to a closed engine outcome by the FR-009 table, and `sat` or `unsat`
/// arises only from a `completed` query result, classified from its exit code, stdout and stderr
/// exactly as a direct run of the same bytes.
pub fn solver_record_from_evidence(
    query: &QueryBundle,
    config: &SolverConfig,
    version: &ProducerExecutionResult,
    query_result: Option<&ProducerExecutionResult>,
) -> SolverRecord {
    record_on_host(
        query,
        config,
        version,
        query_result,
        cfg!(target_os = "linux"),
    )
}

fn record_on_host(
    query: &QueryBundle,
    config: &SolverConfig,
    version: &ProducerExecutionResult,
    query_result: Option<&ProducerExecutionResult>,
    host_is_linux: bool,
) -> SolverRecord {
    let mut record = SolverRecord {
        profile: SOLVER_PROCESS_PROFILE,
        engine: config.engine(),
        outcome: SolverOutcome::IdentityError,
        identity: None,
        configured_executable: config.executable().to_path_buf(),
        argv: config
            .engine()
            .query_arguments()
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect(),
        query_digest: query.query_digest().to_string(),
        limits: config.limits(),
        stdout: Vec::new(),
        stderr: Vec::new(),
        model: Vec::new(),
        exit: None,
        elapsed_ms: 0,
        cleanup_ms: 0,
        diagnostic: None,
    };
    let version_text = match version_text_on_host(config, version, host_is_linux) {
        Ok(text) => text,
        Err(outcome) => {
            record.outcome = outcome;
            record.diagnostic = Some("the solver version invocation was not usable".to_owned());
            return record;
        }
    };
    record.identity = Some(ExecutableIdentity {
        canonical_path: PathBuf::from(&version.producer.executable),
        version: version_text,
    });
    let Some(result) = query_result else {
        record.outcome = SolverOutcome::IoError;
        record.diagnostic = Some("no query invocation result was supplied".to_owned());
        return record;
    };
    if let Some(process) = &result.process {
        record.stdout.clone_from(&process.stdout.bytes);
        record.stderr.clone_from(&process.stderr.bytes);
        record.exit = process.terminal_status.map(process_exit);
    }
    if let Some(nanos) = result.timing.execution_nanos {
        record.elapsed_ms = nanos / 1_000_000;
    }
    if let Some(outcome) = state_outcome(result.state, host_is_linux) {
        record.outcome = outcome;
        return record;
    }
    let State::Completed { observation } = result.state else {
        return record;
    };
    let Some(process) = &result.process else {
        record.outcome = SolverOutcome::IoError;
        return record;
    };
    let Ok(code) = i32::try_from(observation.exit_code) else {
        record.outcome = SolverOutcome::IoError;
        return record;
    };
    record.exit = Some(ProcessExit {
        code: Some(code),
        signal: None,
    });
    if process.stdout.truncated {
        record.outcome = SolverOutcome::StdoutLimit;
        return record;
    }
    if process.stderr.truncated {
        record.outcome = SolverOutcome::StderrLimit;
        return record;
    }
    let (outcome, model) = classify_exit(
        config.engine(),
        record.exit,
        &record.stdout,
        &record.stderr,
        config.limits().model_bytes,
        query.requests_model(),
    );
    record.outcome = outcome;
    record.model = model;
    record
}

fn process_exit(status: TerminalStatus) -> ProcessExit {
    match status.kind {
        StatusKind::ExitCode => ProcessExit {
            code: i32::try_from(status.value).ok(),
            signal: None,
        },
        StatusKind::Signal => ProcessExit {
            code: None,
            signal: i32::try_from(status.value).ok(),
        },
    }
}

#[cfg(test)]
mod tests {
    use quire_contract_model::{
        AnchorName, Clause, ClauseId, ClauseKind, ContractPackage, DeclarationEnvironment,
        ExecutionPoint, Expression, ExpressionKind, PackageId, Requirement, RequirementId,
        RequirementRef, RequirementRevision, SchemaVersion, SourceDocumentId, SourceIdentity,
        SourceLocation, SourceRevision, SourceSpan, TypedExpression, ValueType,
    };
    use serde_json::json;

    use super::*;
    use crate::DifferentialDisposition;
    use crate::{compare_solver_records, lower_boolean_statements, AdapterLimits, SolverEngine};

    fn parsed(state: &Value) -> ProducerExecutionResult {
        let mut line = serde_json::to_vec(&json!({
            "protocol": PRODUCER_RESULT_PROTOCOL,
            "producer": {"name": "z3", "version": "Z3", "executable": "/opt/z3"},
            "cancellationEvent": null,
            "timing": {"preflightNanos": 1, "executionNanos": null, "totalNanos": 2},
            "observedHost": null,
            "process": null,
            "artifacts": [],
            "state": state,
        }))
        .expect("serialize");
        line.push(b'\n');
        parse_result_line(&line).expect("result document")
    }

    fn completed_version() -> ProducerExecutionResult {
        let mut line = serde_json::to_vec(&json!({
            "protocol": PRODUCER_RESULT_PROTOCOL,
            "producer": {"name": "z3", "version": "Z3", "executable": "/opt/z3"},
            "cancellationEvent": null,
            "timing": {"preflightNanos": 1, "executionNanos": 3, "totalNanos": 4},
            "observedHost": null,
            "process": {
                "terminalStatus": {"kind": "exit_code", "value": 0},
                "stdout": {"bytes": b"Z3 version 4.13\n", "truncated": false},
                "stderr": {"bytes": [], "truncated": false},
            },
            "artifacts": [],
            "state": {"kind": "completed", "observation": {"exitCode": 0}},
        }))
        .expect("serialize");
        line.push(b'\n');
        parse_result_line(&line).expect("result document")
    }

    fn one_statement_query() -> QueryBundle {
        let span = {
            let source = SourceIdentity::new(
                SourceDocumentId::new("evidence-unit").expect("source id"),
                SourceRevision::new(1).expect("source revision"),
            );
            SourceSpan::new(
                SourceLocation::new(source.clone(), 1, 1, 0).expect("start"),
                SourceLocation::new(source, 1, 2, 1).expect("end"),
            )
            .expect("span")
        };
        let package_id = PackageId::new("agent-ix/evidence-unit").expect("package");
        let requirement_id = RequirementId::new("REQ-evidence-0").expect("requirement");
        let revision = RequirementRevision::new(1).expect("revision");
        let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
        let environment =
            DeclarationEnvironment::new(owner, vec![], vec![], vec![]).expect("environment");
        let point = ExecutionPoint::Pre {
            operation: AnchorName::new("solve").expect("anchor").into(),
        };
        let expression =
            Expression::new(ExpressionKind::BooleanLiteral { value: true }, span.clone());
        let checked = environment
            .check_expression(&expression, &ValueType::Boolean, &point, true)
            .expect("typed expression");
        let clause = Clause::new(
            ClauseId::new("C-evidence-0").expect("clause"),
            ClauseKind::Assertion,
            Some(point),
            span.clone(),
            checked,
        )
        .expect("clause");
        let requirement = Requirement::<TypedExpression>::new(
            &package_id,
            requirement_id,
            revision,
            span.clone(),
            vec![clause],
        )
        .expect("requirement");
        let package = ContractPackage::new(
            package_id,
            SchemaVersion::V1_1,
            span.source().clone(),
            vec![requirement],
        )
        .expect("package");
        let statement = crate::StatementInput::from_clause(
            &package,
            &package.requirements()[0],
            &package.requirements()[0].clauses()[0],
            environment,
        )
        .expect("statement");
        lower_boolean_statements(&[statement], &[]).expect("query")
    }

    /// Off Linux a `containment_failure` is `unsupported-platform`, whose disposition is
    /// `unavailable`; on Linux it is `cleanup-failed`, which is inconclusive.
    #[test]
    fn containment_failure_disposition_follows_the_host() {
        let query = one_statement_query();
        let config = SolverConfig::new(SolverEngine::Z3, "/opt/z3", AdapterLimits::default())
            .expect("solver config");
        let version = completed_version();
        let failure = parsed(&json!({"kind": "containment_failure"}));
        for (host_is_linux, outcome, disposition) in [
            (
                false,
                SolverOutcome::UnsupportedPlatform,
                DifferentialDisposition::Unavailable,
            ),
            (
                true,
                SolverOutcome::CleanupFailed,
                DifferentialDisposition::Inconclusive,
            ),
        ] {
            let record = record_on_host(&query, &config, &version, Some(&failure), host_is_linux);
            assert_eq!(record.outcome(), outcome);
            assert_eq!(
                compare_solver_records(&query, &record, &record).disposition(),
                disposition
            );
        }
    }
}
