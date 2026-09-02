use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use crate::{
    classify_analysis, execute_solver, AnalysisConclusion, AnalysisStatus, CancellationToken,
    ExplanationState, QueryBundle, SolverConfig, SolverEngine, SolverOutcome, SolverRecord,
    CONTRACT_IR_REVISION,
};

pub const DIFFERENTIAL_REPORT_SCHEMA: &str = "quire.differential-report/v1";
pub const PGM01_ENVELOPE_DEPENDENCY: &str = "agent-ix/quire-contract-ir#20";
pub const MAX_REPORT_BYTES: usize = 64 * 1024 * 1024;
static PUBLICATION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationStage {
    DestinationValidation,
    StagingCreate,
    StagingWrite,
    StagingSync,
    AtomicRename,
    DirectorySync,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationState {
    DestinationUnmodified,
    PublishedDurabilityUnknown,
}

#[derive(Debug)]
pub struct PublicationError {
    stage: PublicationStage,
    state: PublicationState,
    source: io::Error,
    cleanup_error: Option<io::Error>,
}

impl PublicationError {
    fn new(stage: PublicationStage, state: PublicationState, source: io::Error) -> Self {
        Self {
            stage,
            state,
            source,
            cleanup_error: None,
        }
    }

    pub const fn stage(&self) -> PublicationStage {
        self.stage
    }

    pub const fn state(&self) -> PublicationState {
        self.state
    }

    pub fn kind(&self) -> io::ErrorKind {
        self.source.kind()
    }

    pub fn cleanup_error(&self) -> Option<&io::Error> {
        self.cleanup_error.as_ref()
    }
}

impl std::fmt::Display for PublicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "report publication failed at {:?} ({:?}): {}",
            self.stage, self.state, self.source
        )?;
        if let Some(cleanup_error) = &self.cleanup_error {
            write!(formatter, "; staging cleanup also failed: {cleanup_error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PublicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DifferentialDisposition {
    Agreement,
    Disagreement,
    Unavailable,
    Inconclusive,
}

impl DifferentialDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agreement => "agreement",
            Self::Disagreement => "disagreement",
            Self::Unavailable => "unavailable",
            Self::Inconclusive => "inconclusive",
        }
    }

    pub const fn is_conclusive(self) -> bool {
        matches!(self, Self::Agreement)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DifferentialRun {
    z3: AnalysisConclusion,
    cvc5: AnalysisConclusion,
    disposition: DifferentialDisposition,
    agreed_status: Option<AnalysisStatus>,
}

impl DifferentialRun {
    pub const fn z3(&self) -> &AnalysisConclusion {
        &self.z3
    }

    pub const fn cvc5(&self) -> &AnalysisConclusion {
        &self.cvc5
    }

    pub const fn disposition(&self) -> DifferentialDisposition {
        self.disposition
    }

    pub const fn agreed_status(&self) -> Option<AnalysisStatus> {
        self.agreed_status
    }

    pub const fn is_conclusive(&self) -> bool {
        self.disposition.is_conclusive()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DifferentialConfigError(String);

impl std::fmt::Display for DifferentialConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DifferentialConfigError {}

pub fn execute_differential(
    query: &QueryBundle,
    z3: &SolverConfig,
    cvc5: &SolverConfig,
    cancellation: &CancellationToken,
) -> Result<DifferentialRun, DifferentialConfigError> {
    if z3.engine() != SolverEngine::Z3 || cvc5.engine() != SolverEngine::Cvc5 {
        return Err(DifferentialConfigError(
            "differential configuration requires Z3 first and cvc5 second".to_owned(),
        ));
    }
    let z3_record = execute_solver(query, z3, cancellation);
    let cvc5_record = execute_solver(query, cvc5, cancellation);
    Ok(compare_solver_records(query, &z3_record, &cvc5_record))
}

pub fn compare_solver_records(
    query: &QueryBundle,
    z3: &SolverRecord,
    cvc5: &SolverRecord,
) -> DifferentialRun {
    let z3_conclusion = classify_analysis(query, z3);
    let cvc5_conclusion = classify_analysis(query, cvc5);
    let unavailable = [z3.outcome(), cvc5.outcome()].into_iter().any(|outcome| {
        matches!(
            outcome,
            SolverOutcome::MissingExecutable
                | SolverOutcome::UnsupportedPlatform
                | SolverOutcome::VersionMismatch
                | SolverOutcome::ExecutableDigestMismatch
        )
    });
    let verified_if_required = |conclusion: &AnalysisConclusion| {
        conclusion.solver().outcome() != SolverOutcome::Sat
            || conclusion.explanation() == ExplanationState::Verified
    };
    let same_conclusive = z3_conclusion.status() == cvc5_conclusion.status()
        && z3_conclusion.is_conclusive()
        && cvc5_conclusion.is_conclusive()
        && verified_if_required(&z3_conclusion)
        && verified_if_required(&cvc5_conclusion);
    let opposite_conclusive = z3_conclusion.is_conclusive()
        && cvc5_conclusion.is_conclusive()
        && z3_conclusion.status() != cvc5_conclusion.status();
    let disposition = if unavailable {
        DifferentialDisposition::Unavailable
    } else if same_conclusive {
        DifferentialDisposition::Agreement
    } else if opposite_conclusive {
        DifferentialDisposition::Disagreement
    } else {
        DifferentialDisposition::Inconclusive
    };
    let agreed_status =
        (disposition == DifferentialDisposition::Agreement).then_some(z3_conclusion.status());
    DifferentialRun {
        z3: z3_conclusion,
        cvc5: cvc5_conclusion,
        disposition,
        agreed_status,
    }
}

pub fn render_differential_report(
    query: &QueryBundle,
    run: &DifferentialRun,
) -> Result<Vec<u8>, String> {
    let analysis_kind = query
        .analysis_kind()
        .ok_or_else(|| "differential reports require an analysis query".to_owned())?;
    let mut payload = json!({
        "analysisKind": analysis_kind.as_str(),
        "analysisModelProfile": run.z3.analysis_model_profile(),
        "assertions": query.assertions().iter().map(|assertion| json!({
            "clause": format!("{}:{}@{}:{}", assertion.clause.requirement().package(), assertion.clause.requirement().requirement(), assertion.clause.requirement().revision().get(), assertion.clause.clause()),
            "clauseDigest": assertion.clause_digest.to_string(),
            "name": assertion.name,
            "polarity": assertion.polarity.as_str(),
            "role": assertion.role.as_str(),
            "source": source_value(&assertion.source),
        })).collect::<Vec<_>>(),
        "bindingSetDigest": query.binding_set_digest().to_string(),
        "contractIrRevision": CONTRACT_IR_REVISION,
        "differential": {
            "agreedStatus": run.agreed_status.map(AnalysisStatus::as_str),
            "disposition": run.disposition.as_str(),
        },
        "encodingProfile": query.profile(),
        "engines": [engine_value(run.z3()), engine_value(run.cvc5())],
        "logic": query.logic(),
        "pgm01Envelope": {
            "dependency": PGM01_ENVELOPE_DEPENDENCY,
            "status": "unavailable",
        },
        "producer": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
        "queryDigest": query.query_digest().to_string(),
        "queryHex": hex(query.query().as_bytes()),
        "querySha256": sha256(query.query().as_bytes()),
        "requestDigest": query.analysis_request_digest().to_string(),
        "schema": DIFFERENTIAL_REPORT_SCHEMA,
    });
    let digest = digest_json(&payload);
    payload
        .as_object_mut()
        .expect("report object")
        .insert("reportDigest".to_owned(), Value::String(digest));
    Ok(serde_json::to_vec(&payload).expect("serializable report"))
}

pub fn validate_differential_report(
    bytes: &[u8],
    query: &QueryBundle,
    run: &DifferentialRun,
) -> Result<(), String> {
    validate_report_document(bytes)?;
    let value: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "report root is not an object".to_owned())?;
    if object.get("schema").and_then(Value::as_str) != Some(DIFFERENTIAL_REPORT_SCHEMA) {
        return Err("report schema is absent or unsupported".to_owned());
    }
    if object
        .get("pgm01Envelope")
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        != Some("unavailable")
    {
        return Err(
            "PGM-01 envelope status must remain unavailable until the shared component lands"
                .to_owned(),
        );
    }
    let expected = render_differential_report(query, run)?;
    if bytes != expected {
        return Err(
            "report bytes differ from the authoritative canonical reconstruction".to_owned(),
        );
    }
    Ok(())
}

pub fn validate_report_document(bytes: &[u8]) -> Result<DifferentialDisposition, String> {
    if bytes.len() > MAX_REPORT_BYTES {
        return Err("report exceeds the v1 byte bound".to_owned());
    }
    let mut value: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let schema: Value = serde_json::from_str(include_str!(
        "../schemas/differential-report-v1.schema.json"
    ))
    .map_err(|error| format!("embedded report schema is invalid: {error}"))?;
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .map_err(|error| format!("embedded report schema cannot compile: {error}"))?;
    if let Err(mut errors) = validator.validate(&value) {
        let error = errors
            .next()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "unknown validation error".to_owned());
        return Err(format!("report does not satisfy its schema: {error}"));
    }
    {
        let object = require_object(&value, "report root")?;
        require_exact_keys(
            object,
            &[
                "analysisModelProfile",
                "analysisKind",
                "assertions",
                "bindingSetDigest",
                "contractIrRevision",
                "differential",
                "encodingProfile",
                "engines",
                "logic",
                "pgm01Envelope",
                "producer",
                "queryDigest",
                "queryHex",
                "querySha256",
                "reportDigest",
                "requestDigest",
                "schema",
            ],
            "report root",
        )?;
        if object.get("schema").and_then(Value::as_str) != Some(DIFFERENTIAL_REPORT_SCHEMA) {
            return Err("report schema is absent or unsupported".to_owned());
        }
        for (field, expected) in [
            ("analysisModelProfile", "quire.analysis-model/v1"),
            ("encodingProfile", "quire.smtlib2/v1"),
            ("logic", "QF_UF"),
            // Pinned like the profiles above. A report produced under a
            // different contract-IR revision was produced under different
            // clause-digest semantics, and a bare 40-hex shape check does not
            // notice that.
            ("contractIrRevision", CONTRACT_IR_REVISION),
        ] {
            if object.get(field).and_then(Value::as_str) != Some(expected) {
                return Err(format!("report {field} is absent or unsupported"));
            }
        }
        let envelope = require_object(
            object
                .get("pgm01Envelope")
                .ok_or_else(|| "PGM-01 envelope is absent".to_owned())?,
            "PGM-01 envelope",
        )?;
        require_exact_keys(envelope, &["dependency", "status"], "PGM-01 envelope")?;
        if envelope.get("status").and_then(Value::as_str) != Some("unavailable")
            || envelope.get("dependency").and_then(Value::as_str) != Some(PGM01_ENVELOPE_DEPENDENCY)
        {
            return Err("PGM-01 envelope is not the required unavailable state".to_owned());
        }
    }
    let expected_digest = value
        .as_object_mut()
        .expect("object checked")
        .remove("reportDigest")
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| "report digest is absent".to_owned())?;
    if digest_json(&value) != expected_digest {
        return Err("report digest does not match canonical payload bytes".to_owned());
    }
    value
        .as_object_mut()
        .expect("object checked")
        .insert("reportDigest".to_owned(), Value::String(expected_digest));
    if serde_json::to_vec(&value).map_err(|error| error.to_string())? != bytes {
        return Err("report JSON is not in canonical compact form".to_owned());
    }

    let object = value.as_object().expect("object checked");
    let query_bytes = decode_hex(
        object
            .get("queryHex")
            .and_then(Value::as_str)
            .ok_or_else(|| "query bytes are absent".to_owned())?,
    )?;
    if object.get("querySha256").and_then(Value::as_str) != Some(&sha256(&query_bytes)) {
        return Err("query byte digest does not match retained query bytes".to_owned());
    }
    let engines = object
        .get("engines")
        .and_then(Value::as_array)
        .filter(|engines| engines.len() == 2)
        .ok_or_else(|| "report must contain exactly two engines".to_owned())?;
    for (index, engine) in engines.iter().enumerate() {
        let engine_object = require_object(engine, "engine record")?;
        require_exact_keys(
            engine_object,
            &[
                "argv",
                "cleanupMs",
                "configurationDigest",
                "configuredExecutable",
                "diagnostic",
                "elapsedMs",
                "engine",
                "expectedExecutableSha256",
                "expectedVersion",
                "explanation",
                "identity",
                "limits",
                "modelHex",
                "modelSha256",
                "outcome",
                "processExit",
                "profile",
                "queryDigest",
                "status",
                "stderrHex",
                "stderrSha256",
                "stdoutHex",
                "stdoutSha256",
            ],
            "engine record",
        )?;
        let expected_engine = if index == 0 { "z3" } else { "cvc5" };
        if engine.get("engine").and_then(Value::as_str) != Some(expected_engine) {
            return Err(format!("engine record {index} is not {expected_engine}"));
        }
        if engine.get("queryDigest") != object.get("queryDigest") {
            return Err(format!("engine {expected_engine} query digest mismatch"));
        }
        let kind = object
            .get("analysisKind")
            .and_then(Value::as_str)
            .expect("schema checked analysis kind");
        let outcome = engine
            .get("outcome")
            .and_then(Value::as_str)
            .expect("schema checked outcome");
        let expected_status = status_for_values(kind, outcome);
        if engine.get("status").and_then(Value::as_str) != Some(expected_status) {
            return Err(format!(
                "engine {expected_engine} status contradicts its outcome"
            ));
        }
        if let Some(identity) = engine.get("identity").filter(|value| !value.is_null()) {
            require_exact_keys(
                require_object(identity, "engine identity")?,
                &["bytes", "path", "sha256", "version"],
                "engine identity",
            )?;
        }
        let mut streams = std::collections::BTreeMap::new();
        for field in ["stdout", "stderr", "model"] {
            let encoded = engine
                .get(format!("{field}Hex"))
                .and_then(Value::as_str)
                .ok_or_else(|| format!("engine {field} bytes are absent"))?;
            let bytes = decode_hex(encoded)?;
            let digest = engine
                .get(format!("{field}Sha256"))
                .and_then(Value::as_str)
                .ok_or_else(|| format!("engine {field} digest is absent"))?;
            if sha256(&bytes) != digest {
                return Err(format!("engine {field} digest mismatch"));
            }
            streams.insert(field, bytes);
        }

        // Re-derive the conclusion from the raw response the report retains.
        //
        // Checking `stdoutHex` against `stdoutSha256` only proves the record is
        // self-consistent: an author who edits both fields together passes it.
        // The retained bytes are the evidence for the claim, so for the three
        // outcomes where stdout *is* that evidence the claim is re-parsed from
        // it. Without this, a report whose stdout says `unsat` can declare `sat`
        // and validate, and the CLI turns that into exit 0.
        //
        // Only the conclusive-and-unknown outcomes are re-derived. The failure
        // outcomes are decided by process state the report also records — an
        // exit code, a signal, a capture bound — which stdout alone cannot
        // witness, so requiring stdout to agree with them would reject honest
        // records.
        if matches!(outcome, "sat" | "unsat" | "unknown") {
            let model_bytes = engine
                .get("limits")
                .and_then(|limits| limits.get("modelBytes"))
                .and_then(Value::as_u64)
                .ok_or_else(|| "engine model byte bound is absent".to_owned())?;
            let stdout = streams.get("stdout").expect("stdout retained above");
            let reparsed = crate::reparse_solver_status(
                stdout,
                usize::try_from(model_bytes).unwrap_or(usize::MAX),
            );
            if reparsed != Some(outcome) {
                return Err(format!(
                    "engine {expected_engine} outcome {outcome} is not what its retained \
                     stdout says"
                ));
            }
        }
    }

    // Re-derive the query identity from the two inputs the report carries.
    //
    // `querySha256` covers the retained query bytes and `reportDigest` covers
    // the whole payload, but neither ties `queryDigest` to what it is supposed
    // to be a digest *of*. Both of its inputs are fields of this document, so
    // the binding is checkable and is checked.
    let request_digest = object
        .get("requestDigest")
        .and_then(Value::as_str)
        .ok_or_else(|| "request digest is absent".to_owned())?;
    let declared_query_digest = object
        .get("queryDigest")
        .and_then(Value::as_str)
        .ok_or_else(|| "query digest is absent".to_owned())?;
    if crate::derive_query_digest(request_digest, &query_bytes)?.as_str() != declared_query_digest {
        return Err(
            "query digest does not match the retained request identity and query bytes".to_owned(),
        );
    }
    let derived = disposition_from_values(&engines[0], &engines[1])?;
    let differential = object
        .get("differential")
        .and_then(Value::as_object)
        .ok_or_else(|| "differential result is absent".to_owned())?;
    require_exact_keys(
        differential,
        &["agreedStatus", "disposition"],
        "differential result",
    )?;
    if differential.get("disposition").and_then(Value::as_str) != Some(derived.as_str()) {
        return Err("differential disposition contradicts engine records".to_owned());
    }
    let derived_status = if derived == DifferentialDisposition::Agreement {
        engines[0].get("status").and_then(Value::as_str)
    } else {
        None
    };
    if differential.get("agreedStatus").and_then(Value::as_str) != derived_status {
        return Err("agreed status contradicts engine records".to_owned());
    }
    Ok(derived)
}

pub fn render_differential_summary(run: &DifferentialRun) -> String {
    format!(
        "Differential result: {}\nZ3: {}\ncvc5: {}\nPGM-01 envelope: unavailable ({})\n",
        run.disposition.as_str(),
        run.z3.status().as_str(),
        run.cvc5.status().as_str(),
        PGM01_ENVELOPE_DEPENDENCY,
    )
}

#[cfg(target_os = "linux")]
pub fn publish_report_new(destination: &Path, bytes: &[u8]) -> Result<(), PublicationError> {
    publish_report_with(destination, bytes, &mut RealPublicationIo::default())
}

#[cfg(target_os = "linux")]
fn publish_report_with(
    destination: &Path,
    bytes: &[u8],
    publication: &mut impl PublicationIo,
) -> Result<(), PublicationError> {
    if destination.file_name().is_none() {
        return Err(PublicationError::new(
            PublicationStage::DestinationValidation,
            PublicationState::DestinationUnmodified,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "report destination requires a file name",
            ),
        ));
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let counter = PUBLICATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staging = parent.join(format!(
        ".quire-analyze-report-{}-{counter}.tmp",
        std::process::id()
    ));

    publication.create_staging(&staging).map_err(|error| {
        PublicationError::new(
            PublicationStage::StagingCreate,
            PublicationState::DestinationUnmodified,
            error,
        )
    })?;
    publication_test_termination("after-create");

    let before_rename: Result<(), PublicationError> = (|| {
        publication.write_staging(bytes).map_err(|error| {
            PublicationError::new(
                PublicationStage::StagingWrite,
                PublicationState::DestinationUnmodified,
                error,
            )
        })?;
        publication_test_termination("after-write");
        publication.sync_staging().map_err(|error| {
            PublicationError::new(
                PublicationStage::StagingSync,
                PublicationState::DestinationUnmodified,
                error,
            )
        })?;
        publication_test_termination("after-file-sync");
        publication.close_staging();
        publication
            .rename_no_replace(&staging, destination)
            .map_err(|error| {
                PublicationError::new(
                    PublicationStage::AtomicRename,
                    PublicationState::DestinationUnmodified,
                    error,
                )
            })?;
        publication_test_termination("after-rename");
        Ok(())
    })();

    if let Err(mut error) = before_rename {
        publication.close_staging();
        match publication.remove_staging(&staging) {
            Ok(()) => {
                if let Err(cleanup_error) = publication.sync_parent(parent) {
                    error.cleanup_error = Some(cleanup_error);
                }
            }
            Err(cleanup_error) => {
                error.cleanup_error = Some(cleanup_error);
            }
        }
        return Err(error);
    }

    publication.sync_parent(parent).map_err(|error| {
        PublicationError::new(
            PublicationStage::DirectorySync,
            PublicationState::PublishedDurabilityUnknown,
            error,
        )
    })?;
    publication_test_termination("after-directory-sync");
    Ok(())
}

#[cfg(test)]
fn publication_test_termination(point: &str) {
    if std::env::var("QUIRE_ANALYZE_TEST_PUBLICATION_TERMINATE").as_deref() == Ok(point) {
        std::process::exit(86);
    }
}

#[cfg(not(test))]
const fn publication_test_termination(_point: &str) {}

#[cfg(not(target_os = "linux"))]
pub fn publish_report_new(_destination: &Path, _bytes: &[u8]) -> Result<(), PublicationError> {
    Err(PublicationError::new(
        PublicationStage::DestinationValidation,
        PublicationState::DestinationUnmodified,
        io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic no-replace report publication is implemented for Linux",
        ),
    ))
}

#[cfg(target_os = "linux")]
trait PublicationIo {
    fn create_staging(&mut self, path: &Path) -> io::Result<()>;
    fn write_staging(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn sync_staging(&mut self) -> io::Result<()>;
    fn close_staging(&mut self);
    fn rename_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()>;
    fn sync_parent(&mut self, parent: &Path) -> io::Result<()>;
    fn remove_staging(&mut self, path: &Path) -> io::Result<()>;
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct RealPublicationIo {
    staging: Option<File>,
}

#[cfg(target_os = "linux")]
impl PublicationIo for RealPublicationIo {
    fn create_staging(&mut self, path: &Path) -> io::Result<()> {
        self.staging = Some(OpenOptions::new().write(true).create_new(true).open(path)?);
        Ok(())
    }

    fn write_staging(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.staging
            .as_mut()
            .expect("staging file exists after successful creation")
            .write_all(bytes)
    }

    fn sync_staging(&mut self) -> io::Result<()> {
        self.staging
            .as_ref()
            .expect("staging file exists after successful creation")
            .sync_all()
    }

    fn close_staging(&mut self) {
        self.staging = None;
    }

    fn rename_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
        rename_no_replace(source, destination)
    }

    fn sync_parent(&mut self, parent: &Path) -> io::Result<()> {
        File::open(parent)?.sync_all()
    }

    fn remove_staging(&mut self, path: &Path) -> io::Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(target_os = "linux")]
fn rename_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "staging path contains NUL"))?;
    let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination path contains NUL")
    })?;
    // SAFETY: both C strings are live for the call, AT_FDCWD resolves the validated paths, and
    // RENAME_NOREPLACE atomically refuses an existing destination instead of overwriting it.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn source_value(source: &quire_contract_ir::SourceSpan) -> Value {
    json!({
        "document": source.source().document().as_str(),
        "end": {"byte": source.end().byte_offset(), "column": source.end().column(), "line": source.end().line()},
        "revision": source.source().revision().get(),
        "start": {"byte": source.start().byte_offset(), "column": source.start().column(), "line": source.start().line()},
    })
}

fn engine_value(conclusion: &AnalysisConclusion) -> Value {
    let record = conclusion.solver();
    let limits = record.limits();
    json!({
        "argv": record.argv(),
        "cleanupMs": record.cleanup_ms(),
        "configurationDigest": record.configuration_digest().to_string(),
        "configuredExecutable": record.configured_executable().to_str().expect("validated UTF-8 path"),
        "diagnostic": record.diagnostic(),
        "elapsedMs": record.elapsed_ms(),
        "engine": record.engine().as_str(),
        "explanation": match conclusion.explanation() { ExplanationState::NotApplicable => "not-applicable", ExplanationState::Incomplete => "incomplete", ExplanationState::Verified => "verified" },
        "identity": record.identity().map(|identity| json!({"bytes": identity.byte_length, "path": identity.canonical_path, "sha256": identity.sha256.to_string(), "version": identity.version})),
        "limits": {
            "cleanupTimeMs": limits.cleanup_time_ms,
            "executableBytes": limits.executable_bytes,
            "gracefulCleanupMs": limits.graceful_cleanup_ms,
            "modelBytes": limits.model_bytes,
            "monitorIntervalMs": limits.monitor_interval_ms,
            "pathBytes": limits.path_bytes,
            "stderrBytes": limits.stderr_bytes,
            "stdinBytes": limits.stdin_bytes,
            "stdoutBytes": limits.stdout_bytes,
            "versionBytes": limits.version_bytes,
            "wallTimeMs": limits.wall_time_ms,
        },
        "modelHex": hex(record.model()),
        "modelSha256": sha256(record.model()),
        "outcome": record.outcome().as_str(),
        "processExit": record.exit().map(|exit| json!({"code": exit.code, "signal": exit.signal})),
        "profile": record.profile(),
        "queryDigest": record.query_digest(),
        "status": conclusion.status().as_str(),
        "stderrHex": hex(record.stderr()),
        "stderrSha256": sha256(record.stderr()),
        "stdoutHex": hex(record.stdout()),
        "stdoutSha256": sha256(record.stdout()),
        "expectedExecutableSha256": record.expected_executable_sha256().to_string(),
        "expectedVersion": record.expected_version(),
    })
}

fn digest_json(value: &Value) -> String {
    sha256(&serde_json::to_vec(value).expect("serializable JSON"))
}

fn disposition_from_values(z3: &Value, cvc5: &Value) -> Result<DifferentialDisposition, String> {
    let z3_outcome = required_string(z3, "outcome")?;
    let cvc5_outcome = required_string(cvc5, "outcome")?;
    let unavailable = [z3_outcome, cvc5_outcome].into_iter().any(|outcome| {
        matches!(
            outcome,
            "missing-executable"
                | "unsupported-platform"
                | "version-mismatch"
                | "executable-digest-mismatch"
        )
    });
    if unavailable {
        return Ok(DifferentialDisposition::Unavailable);
    }
    let z3_status = required_string(z3, "status")?;
    let cvc5_status = required_string(cvc5, "status")?;
    let conclusive = |status: &str| matches!(status, "satisfied" | "refuted");
    let verified = |value: &Value, outcome: &str| {
        outcome != "sat" || value.get("explanation").and_then(Value::as_str) == Some("verified")
    };
    if z3_status == cvc5_status
        && conclusive(z3_status)
        && verified(z3, z3_outcome)
        && verified(cvc5, cvc5_outcome)
    {
        Ok(DifferentialDisposition::Agreement)
    } else if conclusive(z3_status) && conclusive(cvc5_status) && z3_status != cvc5_status {
        Ok(DifferentialDisposition::Disagreement)
    } else {
        Ok(DifferentialDisposition::Inconclusive)
    }
}

fn status_for_values(kind: &str, outcome: &str) -> &'static str {
    match (kind, outcome) {
        ("consistency", "sat") => "satisfied",
        (_, "sat") => "refuted",
        ("consistency", "unsat") => "refuted",
        (_, "unsat") => "satisfied",
        (_, "unknown") => "unknown",
        (_, "timed-out" | "cancelled") => "timeout",
        (_, "unsupported-platform") => "unsupported",
        _ => "tool-error",
    }
}

fn required_string<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("engine {name} is absent"))
}

fn require_object<'a>(
    value: &'a Value,
    label: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} is not an object"))
}

fn require_exact_keys(
    object: &serde_json::Map<String, Value>,
    expected: &[&str],
    label: &str,
) -> Result<(), String> {
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} has missing or unknown fields"))
    }
}

fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    if text.len() % 2 != 0 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("raw evidence hex is invalid".to_owned());
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII checked");
            u8::from_str_radix(pair, 16).map_err(|_| "raw evidence hex is invalid".to_owned())
        })
        .collect()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("string write");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, process::Command};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum InjectedFault {
        Create,
        PartialWrite,
        FileSync,
        Rename,
        DirectorySync,
    }

    #[derive(Default)]
    struct FakePublicationIo {
        fault: Option<InjectedFault>,
        cleanup_fails: bool,
        staging: Option<Vec<u8>>,
        destination: Option<Vec<u8>>,
        staging_open: bool,
        parent_synced: bool,
        synced_parent: Option<PathBuf>,
    }

    impl FakePublicationIo {
        fn with_fault(fault: InjectedFault, destination: Option<Vec<u8>>) -> Self {
            Self {
                fault: Some(fault),
                destination,
                ..Self::default()
            }
        }

        fn fault(&self, expected: InjectedFault) -> io::Result<()> {
            if self.fault == Some(expected) {
                Err(io::Error::other(format!("injected {expected:?} failure")))
            } else {
                Ok(())
            }
        }
    }

    impl PublicationIo for FakePublicationIo {
        fn create_staging(&mut self, _path: &Path) -> io::Result<()> {
            self.fault(InjectedFault::Create)?;
            if self.staging.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "stale staging file",
                ));
            }
            self.staging = Some(Vec::new());
            self.staging_open = true;
            Ok(())
        }

        fn write_staging(&mut self, bytes: &[u8]) -> io::Result<()> {
            if self.fault == Some(InjectedFault::PartialWrite) {
                self.staging
                    .as_mut()
                    .expect("created staging")
                    .extend_from_slice(&bytes[..bytes.len() / 2]);
                return self.fault(InjectedFault::PartialWrite);
            }
            self.staging
                .as_mut()
                .expect("created staging")
                .extend_from_slice(bytes);
            Ok(())
        }

        fn sync_staging(&mut self) -> io::Result<()> {
            self.fault(InjectedFault::FileSync)
        }

        fn close_staging(&mut self) {
            self.staging_open = false;
        }

        fn rename_no_replace(&mut self, _source: &Path, _destination: &Path) -> io::Result<()> {
            self.fault(InjectedFault::Rename)?;
            if self.destination.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "destination exists",
                ));
            }
            self.destination = self.staging.take();
            Ok(())
        }

        fn sync_parent(&mut self, parent: &Path) -> io::Result<()> {
            self.fault(InjectedFault::DirectorySync)?;
            self.parent_synced = true;
            self.synced_parent = Some(parent.to_owned());
            Ok(())
        }

        fn remove_staging(&mut self, _path: &Path) -> io::Result<()> {
            if self.cleanup_fails {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected cleanup failure",
                ));
            }
            self.staging = None;
            Ok(())
        }
    }

    #[test]
    fn differential_disposition_census_is_closed() {
        let values = [
            DifferentialDisposition::Agreement,
            DifferentialDisposition::Disagreement,
            DifferentialDisposition::Unavailable,
            DifferentialDisposition::Inconclusive,
        ];
        assert_eq!(
            values.iter().filter(|value| value.is_conclusive()).count(),
            1
        );
        assert_eq!(
            values
                .map(DifferentialDisposition::as_str)
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            values.len()
        );
    }

    #[test]
    fn invalid_publication_destination_fails_before_write() {
        assert_eq!(
            publish_report_new(Path::new("/"), b"report")
                .expect_err("directory destination")
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }

    /// TC-008: every recoverable pre-rename fault preserves the destination and removes staging.
    #[test]
    fn publication_pre_rename_faults_are_atomic_and_clean() {
        let existing = b"developer-owned".to_vec();
        for (fault, stage) in [
            (InjectedFault::Create, PublicationStage::StagingCreate),
            (InjectedFault::PartialWrite, PublicationStage::StagingWrite),
            (InjectedFault::FileSync, PublicationStage::StagingSync),
            (InjectedFault::Rename, PublicationStage::AtomicRename),
        ] {
            let mut publication = FakePublicationIo::with_fault(fault, Some(existing.clone()));
            let error = publish_report_with(Path::new("report.json"), b"new", &mut publication)
                .expect_err("injected failure");
            assert_eq!(error.stage(), stage);
            assert_eq!(error.state(), PublicationState::DestinationUnmodified);
            assert_eq!(publication.destination, Some(existing.clone()));
            assert!(publication.staging.is_none());
            assert!(!publication.staging_open);
            assert!(error.cleanup_error().is_none());
        }
    }

    /// TC-008: a cleanup failure is never hidden behind the primary publication failure.
    #[test]
    fn publication_cleanup_failure_is_explicit() {
        let mut publication =
            FakePublicationIo::with_fault(InjectedFault::PartialWrite, Some(b"old".to_vec()));
        publication.cleanup_fails = true;
        let error = publish_report_with(Path::new("report.json"), b"new", &mut publication)
            .expect_err("write and cleanup failure");
        assert_eq!(error.stage(), PublicationStage::StagingWrite);
        assert_eq!(error.state(), PublicationState::DestinationUnmodified);
        assert!(error.cleanup_error().is_some());
        assert_eq!(publication.destination, Some(b"old".to_vec()));
        assert_eq!(publication.staging, Some(b"n".to_vec()));
        assert!(!publication.staging_open);
    }

    /// TC-008: after atomic rename, a directory-sync error reports a complete but uncertain result.
    #[test]
    fn publication_directory_sync_failure_reports_post_rename_state() {
        let mut publication = FakePublicationIo::with_fault(InjectedFault::DirectorySync, None);
        let error = publish_report_with(Path::new("report.json"), b"new", &mut publication)
            .expect_err("directory sync failure");
        assert_eq!(error.stage(), PublicationStage::DirectorySync);
        assert_eq!(error.state(), PublicationState::PublishedDurabilityUnknown);
        assert_eq!(publication.destination, Some(b"new".to_vec()));
        assert!(publication.staging.is_none());
        assert!(!publication.parent_synced);
    }

    /// TC-008: an unknown stale staging path is never deleted or mistaken for this attempt's file.
    #[test]
    fn publication_refuses_stale_staging_without_claiming_ownership() {
        let mut publication = FakePublicationIo {
            staging: Some(b"stale-owner".to_vec()),
            ..FakePublicationIo::default()
        };
        let error = publish_report_with(Path::new("report.json"), b"new", &mut publication)
            .expect_err("stale staging collision");
        assert_eq!(error.stage(), PublicationStage::StagingCreate);
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(publication.destination, None);
        assert_eq!(publication.staging, Some(b"stale-owner".to_vec()));
    }

    /// TC-008: success is not reported until the parent directory has been synchronized.
    #[test]
    fn publication_success_syncs_complete_bytes_and_parent() {
        let mut publication = FakePublicationIo::default();
        publish_report_with(Path::new("report.json"), b"new", &mut publication)
            .expect("publication");
        assert_eq!(publication.destination, Some(b"new".to_vec()));
        assert!(publication.staging.is_none());
        assert!(publication.parent_synced);
        assert_eq!(publication.synced_parent, Some(PathBuf::from(".")));
    }

    #[test]
    fn publication_termination_helper() {
        let Some(destination) = std::env::var_os("QUIRE_ANALYZE_TEST_PUBLICATION_DESTINATION")
        else {
            return;
        };
        publish_report_new(Path::new(&destination), b"complete-report")
            .expect("termination point must exit before this returns");
        panic!("configured termination point was not reached");
    }

    /// TC-008: abrupt termination never exposes partial destination bytes; each residue is explicit.
    #[test]
    fn publication_process_termination_boundaries_have_defined_state() {
        let root = std::env::temp_dir().join(format!(
            "quire-publication-crash-{}-{}",
            std::process::id(),
            PUBLICATION_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("crash test root");
        for point in [
            "after-create",
            "after-write",
            "after-file-sync",
            "after-rename",
            "after-directory-sync",
        ] {
            let directory = root.join(point);
            fs::create_dir(&directory).expect("crash boundary directory");
            let destination = directory.join("report.json");
            let status = Command::new(std::env::current_exe().expect("test executable"))
                .args([
                    "--exact",
                    "report::tests::publication_termination_helper",
                    "--nocapture",
                ])
                .env("QUIRE_ANALYZE_TEST_PUBLICATION_TERMINATE", point)
                .env("QUIRE_ANALYZE_TEST_PUBLICATION_DESTINATION", &destination)
                .status()
                .expect("termination subprocess");
            assert_eq!(status.code(), Some(86), "termination point {point}");

            let staging = fs::read_dir(&directory)
                .expect("crash directory")
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".quire-analyze-report-")
                })
                .collect::<Vec<_>>();
            if matches!(point, "after-create" | "after-write" | "after-file-sync") {
                assert!(!destination.exists());
                assert_eq!(staging.len(), 1, "private crash residue at {point}");
            } else {
                assert_eq!(
                    fs::read(&destination).expect("complete destination"),
                    b"complete-report"
                );
                assert!(staging.is_empty());
            }
        }
        fs::remove_dir_all(root).expect("remove crash test root");
    }
}
