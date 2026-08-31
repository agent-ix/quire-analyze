//! Requirement-tagged tests for bounded external solver adapters (issue #3).

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use quire_analyze::{
    execute_solver, lower_boolean_statements, AdapterLimits, CancellationToken, SolverConfig,
    SolverDigest, SolverEngine, SolverOutcome, SolverPin, MAX_SOLVER_CLEANUP_TIME_MS,
    MAX_SOLVER_EXECUTABLE_BYTES, MAX_SOLVER_GRACEFUL_CLEANUP_MS, MAX_SOLVER_MODEL_BYTES,
    MAX_SOLVER_MONITOR_INTERVAL_MS, MAX_SOLVER_PATH_BYTES, MAX_SOLVER_STDERR_BYTES,
    MAX_SOLVER_STDIN_BYTES, MAX_SOLVER_STDOUT_BYTES, MAX_SOLVER_VERSION_BYTES,
    MAX_SOLVER_WALL_TIME_MS,
};
use quire_contract_ir::{
    AnchorName, Clause, ClauseId, ClauseKind, ContractPackage, DeclarationEnvironment,
    ExecutionPoint, Expression, ExpressionKind, PackageId, Requirement, RequirementId,
    RequirementRef, RequirementRevision, SchemaVersion, SourceDocumentId, SourceIdentity,
    SourceLocation, SourceRevision, SourceSpan, TypedExpression, ValueType,
};
use sha2::{Digest as _, Sha256};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const VERSION: &str = "fake-solver 1.0";

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quire-analyze-{label}-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn span() -> SourceSpan {
    let source = SourceIdentity::new(
        SourceDocumentId::new("adapter-test").expect("source id"),
        SourceRevision::new(1).expect("source revision"),
    );
    SourceSpan::new(
        SourceLocation::new(source.clone(), 1, 1, 0).expect("start"),
        SourceLocation::new(source, 1, 2, 1).expect("end"),
    )
    .expect("span")
}

fn statement(index: usize) -> quire_analyze::StatementInput {
    let package_id = PackageId::new("agent-ix/adapter-test").expect("package");
    let requirement_id = RequirementId::new(format!("REQ-adapter-{index}")).expect("requirement");
    let revision = RequirementRevision::new(1).expect("revision");
    let owner = RequirementRef::new(package_id.clone(), requirement_id.clone(), revision);
    let environment =
        DeclarationEnvironment::new(owner, vec![], vec![], vec![]).expect("environment");
    let point = ExecutionPoint::Pre {
        operation: AnchorName::new("solve").expect("anchor"),
    };
    let expression = Expression::new(ExpressionKind::BooleanLiteral { value: true }, span());
    let checked = environment
        .check_expression(&expression, &ValueType::Boolean, &point, true)
        .expect("typed expression");
    let clause = Clause::new(
        ClauseId::new(format!("C-adapter-{index}")).expect("clause"),
        ClauseKind::Assertion,
        Some(point),
        span(),
        checked,
    )
    .expect("clause");
    let requirement = Requirement::<TypedExpression>::new(
        &package_id,
        requirement_id,
        revision,
        span(),
        vec![clause],
    )
    .expect("requirement");
    let source = span().source().clone();
    let package = ContractPackage::new(package_id, SchemaVersion::V1_0, source, vec![requirement])
        .expect("package");
    quire_analyze::StatementInput::from_clause(
        &package,
        &package.requirements()[0],
        &package.requirements()[0].clauses()[0],
        environment,
    )
    .expect("statement")
}

fn query() -> quire_analyze::QueryBundle {
    lower_boolean_statements(&[statement(0)], &[]).expect("query")
}

fn large_query() -> quire_analyze::QueryBundle {
    let statements = (0..512).map(statement).collect::<Vec<_>>();
    let query = lower_boolean_statements(&statements, &[]).expect("large query");
    assert!(query.query().len() > 64 * 1024);
    query
}

fn script_body(mode: &str, pid_file: Option<&Path>) -> String {
    let version_action = match mode {
        "version_empty" => "exit 0".to_owned(),
        "version_nonzero" => "exit 9".to_owned(),
        "version_invalid" => "printf '\\377'".to_owned(),
        "version_slow" => "/bin/sleep 30".to_owned(),
        "version_mutates" => format!("printf '{VERSION}\\n'; printf '# changed\\n' >> \"$0\""),
        _ => format!("printf '{VERSION}\\n'"),
    };
    let action = match mode {
        "sat" => "printf 'sat\\n'".to_owned(),
        "unsat" => "printf 'unsat\\n'".to_owned(),
        "unknown" => "printf 'unknown\\n'".to_owned(),
        "malformed" => "printf 'not-a-status\\n'".to_owned(),
        "solver_error" => "printf '(error \"injected\")\\n'".to_owned(),
        "contradictory" => "printf 'sat\\nunsat\\n'".to_owned(),
        "nonzero" => "printf 'sat\\n'; exit 7".to_owned(),
        "nonzero_diagnostic" => "printf 'failure' >&2; exit 7".to_owned(),
        "signaled" => "kill -TERM $$".to_owned(),
        "diagnostic" => "printf 'warning' >&2; printf 'sat\\n'".to_owned(),
        "large_stdout" => "i=0; while [ $i -lt 65 ]; do printf x; i=$((i + 1)); done".to_owned(),
        "large_stderr" => {
            "i=0; while [ $i -lt 65 ]; do printf x >&2; i=$((i + 1)); done; printf 'sat\\n'"
                .to_owned()
        }
        "model" => "printf 'sat\\n(aaaaaa)\\n'".to_owned(),
        "flood" => "/bin/dd if=/dev/zero bs=131072 count=1 2>/dev/null & /bin/dd if=/dev/zero bs=131072 count=1 1>&2 2>/dev/null; wait".to_owned(),
        "slow" => "/bin/sleep 30".to_owned(),
        "mutate_query" => "printf '# changed\\n' >> \"$0\"; printf 'sat\\n'".to_owned(),
        "version_empty"
        | "version_nonzero"
        | "version_invalid"
        | "version_slow"
        | "version_mutates" => {
            "printf 'sat\\n'".to_owned()
        }
        "slow_tree" => {
            let pid_file = pid_file.expect("pid file").display();
            format!("/bin/sleep 30 & child=$!; printf '%s\\n%s\\n' $$ $child > '{pid_file}'; wait")
        }
        other => panic!("unknown fake mode {other}"),
    };
    format!(
        "#!/bin/sh\nif [ \"$1\" = \"-version\" ] || [ \"$1\" = \"--version\" ]; then\n  {version_action}\n  exit $?\nfi\n{action}\n"
    )
}

fn make_script(directory: &Path, name: &str, mode: &str, pid_file: Option<&Path>) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, script_body(mode, pid_file)).expect("write fake solver");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions).expect("make fake solver executable");
    path
}

fn digest(path: &Path) -> SolverDigest {
    SolverDigest::from_bytes(Sha256::digest(fs::read(path).expect("read script")).into())
}

fn config(engine: SolverEngine, executable: &Path, limits: AdapterLimits) -> SolverConfig {
    SolverConfig::new(
        engine,
        executable,
        SolverPin::new(VERSION, digest(executable)),
        limits,
    )
    .expect("solver config")
}

fn execute_mode(mode: &str, limits: AdapterLimits) -> quire_analyze::SolverRecord {
    let directory = TempDirectory::new(mode);
    let executable = make_script(directory.path(), "solver", mode, None);
    execute_solver(
        &query(),
        &config(SolverEngine::Z3, &executable, limits),
        &CancellationToken::default(),
    )
}

/// FR-003-AC-2/3: both engines share the exact normalized sat/unsat/unknown contract.
/// Trace: TC-005, TC-006, FR-003-AC-2, FR-003-AC-3
#[test]
fn exact_engine_contract_and_protocol_states() {
    for (mode, expected) in [
        ("sat", SolverOutcome::Sat),
        ("unsat", SolverOutcome::Unsat),
        ("unknown", SolverOutcome::Unknown),
        ("malformed", SolverOutcome::MalformedOutput),
        ("solver_error", SolverOutcome::SolverError),
        ("contradictory", SolverOutcome::ContradictoryOutput),
        ("nonzero", SolverOutcome::NonzeroExit),
        ("nonzero_diagnostic", SolverOutcome::NonzeroExit),
        ("signaled", SolverOutcome::Signaled),
        ("diagnostic", SolverOutcome::DiagnosticOutput),
    ] {
        let record = execute_mode(mode, AdapterLimits::default());
        assert_eq!(record.outcome(), expected, "mode {mode}");
        assert_eq!(
            record.is_conclusive_candidate(),
            matches!(mode, "sat" | "unsat")
        );
    }

    let directory = TempDirectory::new("engines");
    let z3 = make_script(directory.path(), "z3-airgap", "sat", None);
    let cvc5 = make_script(directory.path(), "cvc5-airgap", "sat", None);
    assert_ne!(z3, cvc5);
    for (engine, executable) in [(SolverEngine::Z3, z3), (SolverEngine::Cvc5, cvc5)] {
        let solver_config = config(engine, &executable, AdapterLimits::default());
        let record = execute_solver(&query(), &solver_config, &CancellationToken::default());
        assert_eq!(record.outcome(), SolverOutcome::Sat);
        assert_eq!(record.engine(), engine);
        assert_eq!(record.profile(), "quire.solver-process/v1");
        assert_eq!(record.limits(), AdapterLimits::default());
        assert_eq!(record.stdout(), b"sat\n");
        assert!(record.stderr().is_empty());
        assert!(record.model().is_empty());
        assert_eq!(record.exit().expect("exit").code, Some(0));
        assert!(record.elapsed_ms() <= MAX_SOLVER_WALL_TIME_MS + MAX_SOLVER_CLEANUP_TIME_MS);
        assert!(record.cleanup_ms() <= MAX_SOLVER_CLEANUP_TIME_MS);
        assert_eq!(
            record.argv(),
            match engine {
                SolverEngine::Z3 => vec!["-in".to_owned(), "-smt2".to_owned()],
                SolverEngine::Cvc5 => {
                    vec!["--lang=smt2".to_owned(), "--no-incremental".to_owned()]
                }
            }
        );
        assert_eq!(record.identity().expect("identity").version, VERSION);
        assert_eq!(
            record.identity().expect("identity").sha256,
            digest(&executable)
        );
        assert!(!record.configuration_digest().to_string().is_empty());
        assert!(!record.query_digest().is_empty());
        let repeated = execute_solver(&query(), &solver_config, &CancellationToken::default());
        assert_eq!(
            record.normalized_outcome_bytes(),
            repeated.normalized_outcome_bytes()
        );
    }
}

/// FR-003-AC-5: absolute metacharacter paths are direct argv inputs, never shell programs.
/// Trace: TC-005, FR-003-AC-5
#[test]
fn paths_are_absolute_pinned_and_not_shell_interpreted() {
    let directory = TempDirectory::new("path;metacharacters");
    let marker = directory.path().join("must-not-exist");
    let executable = make_script(directory.path(), "solver;touch must-not-exist", "sat", None);
    let record = execute_solver(
        &query(),
        &config(SolverEngine::Z3, &executable, AdapterLimits::default()),
        &CancellationToken::default(),
    );
    assert_eq!(record.outcome(), SolverOutcome::Sat);
    assert!(!marker.exists());
    assert!(SolverConfig::new(
        SolverEngine::Z3,
        PathBuf::from("relative-solver"),
        SolverPin::new(VERSION, digest(&executable)),
        AdapterLimits::default(),
    )
    .is_err());

    let missing = directory.path().join("missing");
    let missing_config = SolverConfig::new(
        SolverEngine::Z3,
        &missing,
        SolverPin::new(VERSION, SolverDigest::from_bytes([0; 32])),
        AdapterLimits::default(),
    )
    .expect("missing path is syntactically valid");
    assert_eq!(
        execute_solver(&query(), &missing_config, &CancellationToken::default()).outcome(),
        SolverOutcome::MissingExecutable
    );

    let wrong_pin = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        SolverPin::new(VERSION, SolverDigest::from_bytes([0; 32])),
        AdapterLimits::default(),
    )
    .expect("wrong pin config");
    assert_eq!(
        execute_solver(&query(), &wrong_pin, &CancellationToken::default()).outcome(),
        SolverOutcome::ExecutableDigestMismatch
    );
    let wrong_version = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        SolverPin::new("fake-solver 9.9", digest(&executable)),
        AdapterLimits::default(),
    )
    .expect("wrong version config");
    assert_eq!(
        execute_solver(&query(), &wrong_version, &CancellationToken::default()).outcome(),
        SolverOutcome::VersionMismatch
    );
}

/// FR-003-AC-2/4: identity-probe failures stay distinct from solver answers.
/// Trace: TC-005, FR-003-AC-2, FR-003-AC-4
#[test]
fn executable_identity_probe_fails_closed() {
    for (mode, expected) in [
        ("version_empty", SolverOutcome::IdentityError),
        ("version_nonzero", SolverOutcome::IdentityError),
        ("version_invalid", SolverOutcome::IdentityError),
        ("version_mutates", SolverOutcome::ExecutableChanged),
    ] {
        assert_eq!(
            execute_mode(mode, AdapterLimits::default()).outcome(),
            expected,
            "mode {mode}"
        );
    }
    assert_eq!(
        execute_mode("mutate_query", AdapterLimits::default()).outcome(),
        SolverOutcome::ExecutableChanged
    );

    let directory = TempDirectory::new("identity-probe");
    let directory_config = SolverConfig::new(
        SolverEngine::Z3,
        directory.path(),
        SolverPin::new(VERSION, SolverDigest::from_bytes([0; 32])),
        AdapterLimits::default(),
    )
    .expect("directory is syntactically valid");
    assert_eq!(
        execute_solver(&query(), &directory_config, &CancellationToken::default()).outcome(),
        SolverOutcome::IdentityError
    );

    let invalid = directory.path().join("invalid-executable");
    fs::write(&invalid, b"not an executable format").expect("invalid executable");
    let mut permissions = fs::metadata(&invalid).expect("metadata").permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&invalid, permissions).expect("permissions");
    assert_eq!(
        execute_solver(
            &query(),
            &config(SolverEngine::Z3, &invalid, AdapterLimits::default()),
            &CancellationToken::default(),
        )
        .outcome(),
        SolverOutcome::SpawnError
    );

    let slow = make_script(directory.path(), "slow-version", "version_slow", None);
    let limits = AdapterLimits {
        wall_time_ms: 50,
        cleanup_time_ms: 1_000,
        graceful_cleanup_ms: 20,
        monitor_interval_ms: 2,
        ..AdapterLimits::default()
    };
    assert_eq!(
        execute_solver(
            &query(),
            &config(SolverEngine::Z3, &slow, limits),
            &CancellationToken::default(),
        )
        .outcome(),
        SolverOutcome::TimedOut
    );

    let cancellation = CancellationToken::default();
    let worker_token = cancellation.clone();
    let solver_config = config(SolverEngine::Z3, &slow, AdapterLimits::default());
    let worker = thread::spawn(move || execute_solver(&query(), &solver_config, &worker_token));
    thread::sleep(Duration::from_millis(20));
    cancellation.cancel();
    assert_eq!(
        worker.join().expect("version cancellation").outcome(),
        SolverOutcome::Cancelled
    );
}

/// NFR-001-AC-2/3: the closed numeric profile rejects every one-over boundary.
/// Trace: TC-005, NFR-001-AC-2, NFR-001-AC-3
#[test]
fn every_adapter_owned_limit_is_finite_and_enforced() {
    let defaults = AdapterLimits::default();
    assert_eq!(defaults.wall_time_ms, MAX_SOLVER_WALL_TIME_MS);
    assert_eq!(defaults.cleanup_time_ms, MAX_SOLVER_CLEANUP_TIME_MS);
    assert_eq!(defaults.graceful_cleanup_ms, MAX_SOLVER_GRACEFUL_CLEANUP_MS);
    assert_eq!(defaults.monitor_interval_ms, MAX_SOLVER_MONITOR_INTERVAL_MS);
    assert_eq!(defaults.stdin_bytes, MAX_SOLVER_STDIN_BYTES);
    assert_eq!(defaults.stdout_bytes, MAX_SOLVER_STDOUT_BYTES);
    assert_eq!(defaults.stderr_bytes, MAX_SOLVER_STDERR_BYTES);
    assert_eq!(defaults.model_bytes, MAX_SOLVER_MODEL_BYTES);
    assert_eq!(defaults.version_bytes, MAX_SOLVER_VERSION_BYTES);
    assert_eq!(defaults.executable_bytes, MAX_SOLVER_EXECUTABLE_BYTES);
    assert_eq!(defaults.path_bytes, MAX_SOLVER_PATH_BYTES);

    let directory = TempDirectory::new("limits");
    let executable = make_script(directory.path(), "solver", "sat", None);
    let pin = SolverPin::new(VERSION, digest(&executable));
    for invalid in [
        AdapterLimits {
            wall_time_ms: 0,
            ..defaults
        },
        AdapterLimits {
            cleanup_time_ms: MAX_SOLVER_CLEANUP_TIME_MS + 1,
            ..defaults
        },
        AdapterLimits {
            graceful_cleanup_ms: 0,
            ..defaults
        },
        AdapterLimits {
            monitor_interval_ms: 0,
            ..defaults
        },
        AdapterLimits {
            stdin_bytes: MAX_SOLVER_STDIN_BYTES + 1,
            ..defaults
        },
        AdapterLimits {
            stdout_bytes: MAX_SOLVER_STDOUT_BYTES + 1,
            ..defaults
        },
        AdapterLimits {
            stderr_bytes: MAX_SOLVER_STDERR_BYTES + 1,
            ..defaults
        },
        AdapterLimits {
            model_bytes: MAX_SOLVER_MODEL_BYTES + 1,
            ..defaults
        },
        AdapterLimits {
            version_bytes: MAX_SOLVER_VERSION_BYTES + 1,
            ..defaults
        },
        AdapterLimits {
            executable_bytes: MAX_SOLVER_EXECUTABLE_BYTES + 1,
            ..defaults
        },
        AdapterLimits {
            path_bytes: MAX_SOLVER_PATH_BYTES + 1,
            ..defaults
        },
    ] {
        assert!(SolverConfig::new(SolverEngine::Z3, &executable, pin.clone(), invalid).is_err());
    }

    let query = query();
    let exact_input = AdapterLimits {
        stdin_bytes: query.query().len(),
        ..defaults
    };
    assert_eq!(
        execute_solver(
            &query,
            &config(SolverEngine::Z3, &executable, exact_input),
            &CancellationToken::default(),
        )
        .outcome(),
        SolverOutcome::Sat
    );
    let over_input = AdapterLimits {
        stdin_bytes: query.query().len() - 1,
        ..defaults
    };
    assert_eq!(
        execute_solver(
            &query,
            &config(SolverEngine::Z3, &executable, over_input),
            &CancellationToken::default(),
        )
        .outcome(),
        SolverOutcome::StdinLimit
    );

    assert_eq!(
        execute_mode(
            "sat",
            AdapterLimits {
                stdout_bytes: 4,
                model_bytes: 4,
                ..defaults
            }
        )
        .outcome(),
        SolverOutcome::Sat
    );
    let stdout_limits = AdapterLimits {
        stdout_bytes: 3,
        model_bytes: 3,
        ..defaults
    };
    assert_eq!(
        execute_mode("sat", stdout_limits).outcome(),
        SolverOutcome::StdoutLimit
    );
    assert_eq!(
        execute_mode(
            "large_stderr",
            AdapterLimits {
                stderr_bytes: 64,
                ..defaults
            },
        )
        .outcome(),
        SolverOutcome::StderrLimit
    );
    assert_eq!(
        execute_mode(
            "large_stderr",
            AdapterLimits {
                stderr_bytes: 65,
                ..defaults
            }
        )
        .outcome(),
        SolverOutcome::DiagnosticOutput
    );
    assert_eq!(
        execute_mode(
            "model",
            AdapterLimits {
                model_bytes: 10,
                ..defaults
            }
        )
        .outcome(),
        SolverOutcome::Sat
    );
    assert_eq!(
        execute_mode(
            "model",
            AdapterLimits {
                model_bytes: 9,
                ..defaults
            }
        )
        .outcome(),
        SolverOutcome::ModelLimit
    );

    let file_size = fs::metadata(&executable).expect("metadata").len();
    let exact_executable = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        pin.clone(),
        AdapterLimits {
            executable_bytes: file_size,
            ..defaults
        },
    )
    .expect("exact executable bound");
    assert_eq!(
        execute_solver(&query, &exact_executable, &CancellationToken::default()).outcome(),
        SolverOutcome::Sat
    );
    let identity_limited = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        pin.clone(),
        AdapterLimits {
            executable_bytes: file_size - 1,
            ..defaults
        },
    )
    .expect("syntactically valid identity limit");
    assert_eq!(
        execute_solver(&query, &identity_limited, &CancellationToken::default()).outcome(),
        SolverOutcome::IdentityError
    );

    let path_length = executable.to_str().expect("UTF-8 path").len();
    let exact_path = SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        pin.clone(),
        AdapterLimits {
            path_bytes: path_length,
            ..defaults
        },
    )
    .expect("exact path bound");
    assert_eq!(
        execute_solver(&query, &exact_path, &CancellationToken::default()).outcome(),
        SolverOutcome::Sat
    );
    assert!(SolverConfig::new(
        SolverEngine::Z3,
        &executable,
        pin,
        AdapterLimits {
            path_bytes: path_length - 1,
            ..defaults
        },
    )
    .is_err());

    let version_output_bytes = VERSION.len() + 1;
    let exact_version = config(
        SolverEngine::Z3,
        &executable,
        AdapterLimits {
            version_bytes: version_output_bytes,
            ..defaults
        },
    );
    assert_eq!(
        execute_solver(&query, &exact_version, &CancellationToken::default()).outcome(),
        SolverOutcome::Sat
    );
    let short_version = config(
        SolverEngine::Z3,
        &executable,
        AdapterLimits {
            version_bytes: VERSION.len(),
            ..defaults
        },
    );
    assert_eq!(
        execute_solver(&query, &short_version, &CancellationToken::default()).outcome(),
        SolverOutcome::IdentityError
    );
}

/// NFR-001-AC-2/3: full stdin and output pipes cannot block the wall-time monitor.
/// Trace: TC-005, NFR-001-AC-2, NFR-001-AC-3
#[test]
fn hostile_pipe_pressure_remains_bounded() {
    let flooded = execute_mode(
        "flood",
        AdapterLimits {
            stdout_bytes: 64,
            stderr_bytes: 64,
            model_bytes: 64,
            ..AdapterLimits::default()
        },
    );
    assert_eq!(flooded.outcome(), SolverOutcome::StdoutLimit);
    assert_eq!(flooded.stdout().len(), 64);
    assert_eq!(flooded.stderr().len(), 64);

    let directory = TempDirectory::new("blocked-stdin");
    let executable = make_script(directory.path(), "solver", "slow", None);
    let limits = AdapterLimits {
        wall_time_ms: 50,
        cleanup_time_ms: 1_000,
        graceful_cleanup_ms: 20,
        monitor_interval_ms: 2,
        ..AdapterLimits::default()
    };
    let record = execute_solver(
        &large_query(),
        &config(SolverEngine::Z3, &executable, limits),
        &CancellationToken::default(),
    );
    assert_eq!(record.outcome(), SolverOutcome::TimedOut);
    assert!(record.elapsed_ms() <= 1_050);

    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let cancelled = execute_solver(
        &query(),
        &config(SolverEngine::Z3, &executable, AdapterLimits::default()),
        &cancellation,
    );
    assert_eq!(cancelled.outcome(), SolverOutcome::Cancelled);
    assert!(cancelled.identity().is_none());
}

fn wait_for_pid_file(path: &Path) -> Vec<i32> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if let Ok(contents) = fs::read_to_string(path) {
            let pids: Vec<_> = contents
                .lines()
                .map(|line| line.parse().expect("pid"))
                .collect();
            if pids.len() == 2 {
                return pids;
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("fake solver did not publish process ids");
}

fn process_exists(pid: i32) -> bool {
    // SAFETY: signal 0 is a non-mutating existence probe for a pid supplied by the test fixture.
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// FR-003-AC-1: timeout and cancellation reap the whole isolated group in the numeric bound.
/// Trace: TC-005, FR-003-AC-1, NFR-001-AC-2
#[test]
fn timeout_and_cancellation_cleanup_are_measured() {
    let mut durations = Vec::new();
    for cancelled in [false, true] {
        for repetition in 0..3 {
            let directory = TempDirectory::new(&format!("cleanup-{cancelled}-{repetition}"));
            let pid_file = directory.path().join("pids");
            let executable = make_script(directory.path(), "solver", "slow_tree", Some(&pid_file));
            let limits = AdapterLimits {
                wall_time_ms: if cancelled { 1_000 } else { 50 },
                cleanup_time_ms: 1_000,
                graceful_cleanup_ms: 20,
                monitor_interval_ms: 2,
                ..AdapterLimits::default()
            };
            let config = config(SolverEngine::Z3, &executable, limits);
            let query = query();
            let cancellation = CancellationToken::default();
            let worker_token = cancellation.clone();
            let worker = thread::spawn(move || execute_solver(&query, &config, &worker_token));
            let pids = wait_for_pid_file(&pid_file);
            if cancelled {
                cancellation.cancel();
            }
            let record = worker.join().expect("adapter worker");
            assert_eq!(
                record.outcome(),
                if cancelled {
                    SolverOutcome::Cancelled
                } else {
                    SolverOutcome::TimedOut
                }
            );
            assert!(
                record.cleanup_ms() <= 1_000,
                "cleanup {} ms",
                record.cleanup_ms()
            );
            for pid in pids {
                assert!(!process_exists(pid), "surviving process {pid}");
            }
            durations.push(record.cleanup_ms());
        }
    }
    assert_eq!(durations.len(), 6);
    let maximum = *durations.iter().max().expect("measurement");
    eprintln!("TC-005 cleanup_ms={durations:?} maximum_ms={maximum}");
    assert!(maximum <= 1_000);
}
