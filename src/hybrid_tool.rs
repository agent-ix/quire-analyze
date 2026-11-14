// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PHAVer adapter for bounded hybrid reachability (FR-042).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use quire_analyze_method_contract::{AnalyzeMethod, AnalyzeOutcome};
use serde::{Deserialize, Serialize};

use crate::{
    external_tool_request, invoke_producer_execute, EvidenceErrorKind, ExternalExecutionState,
    MAX_SOLVER_STDERR_BYTES, MAX_SOLVER_STDIN_BYTES, MAX_SOLVER_STDOUT_BYTES,
    MAX_SOLVER_WALL_TIME_MS,
};

const RESULT_SCHEMA: &str = "quire-analyze.hybrid-tool-result/v1";
const MAX_MODE_BYTES: usize = 128;
const MAX_TRANSITIONS: usize = 100_000;
const MAX_RESULT_ENTRIES: usize = 4_000_000;
const MODEL_FILE: &str = "model.pha";
const PRODUCER_EXECUTE_CLEANUP_MS: u64 = 1_000;
// 20 CLI launches with invalid requests measured at most 31.6 ms end to end.
const PRODUCER_EXECUTE_STARTUP_MARGIN_MS: u64 = 500;
const PRODUCER_EXECUTE_FALLBACK_CLEANUP_MS: u64 = 500;
static ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A closed integer interval used by the finite hybrid request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interval {
    /// Inclusive lower bound.
    pub lower: i64,
    /// Inclusive upper bound.
    pub upper: i64,
}

/// One guarded additive reset in a finite hybrid model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridTransition {
    /// Source mode identifier.
    pub from_mode: String,
    /// Target mode identifier.
    pub to_mode: String,
    /// Closed guard interval.
    pub guard: Interval,
    /// Reset offset added to the flowed state.
    pub reset_offset: i64,
}

/// Finite hybrid model submitted to the external reachability tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridRequest {
    /// Initial mode identifier.
    pub initial_mode: String,
    /// Closed initial state interval.
    pub initial_set: Interval,
    /// Closed flow delta interval per step.
    pub flow_delta: Interval,
    /// Authored guarded/reset transitions.
    pub transitions: Vec<HybridTransition>,
    /// Requested transition targets whose reachable set is explicitly expected to be empty.
    /// These modes are included in every result step and may have no intervals.
    pub empty_targets: BTreeSet<String>,
    /// Number of discrete flow/transition steps to analyze.
    pub horizon: u64,
    /// Widening applied to reported tool intervals.
    pub error_bound: u64,
}

/// Tool paths and process environment for delegated reachability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridToolConfig {
    /// PHAVer executable. `None` selects the recorded tool-none disposition.
    pub executable: Option<PathBuf>,
    /// engineering-assurance executable that provides producer-execute.
    pub producer_execute: PathBuf,
    /// Explicit environment for the PHAVer process.
    pub environment: BTreeMap<String, String>,
}

/// Interval returned for one requested mode and step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridEnclosure {
    /// Mode identity.
    pub mode: String,
    /// Closed integer hull; `None` is an explicitly empty mode/step enclosure.
    pub interval: Option<IntegerInterval>,
}

/// Closed integer hull returned from a rational PHAVer enclosure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegerInterval {
    /// Inclusive lower integer bound.
    pub lower: i128,
    /// Inclusive upper integer bound.
    pub upper: i128,
}

/// Result of delegated hybrid reachability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridOutcome {
    /// Complete tool enclosure for every step from zero through the horizon.
    Enclosure { steps: Vec<Vec<HybridEnclosure>> },
    /// A valid request could not be completed within its declared process limits.
    Incomplete { reason: &'static str },
    /// The request or tool result was refused.
    Refused { reason: &'static str },
}

impl HybridOutcome {
    /// Map the delegated result to the existing Analyze method outcome contract.
    pub const fn analyze_classification(&self) -> (AnalyzeMethod, AnalyzeOutcome) {
        let outcome = match self {
            Self::Enclosure { .. } => AnalyzeOutcome::Enclosure,
            Self::Incomplete { .. } => AnalyzeOutcome::Incomplete,
            Self::Refused { .. } => AnalyzeOutcome::Refused,
        };
        (AnalyzeMethod::HybridReachability, outcome)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolResult {
    schema: String,
    requested_horizon: u64,
    steps: Vec<ToolStep>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolStep {
    step: u64,
    modes: Vec<ToolMode>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolMode {
    mode: String,
    intervals: Vec<ToolInterval>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolInterval {
    lower: String,
    upper: String,
}

/// Execute one valid hybrid request through producer-execute.
#[must_use]
pub fn reach_hybrid(request: &HybridRequest, config: &HybridToolConfig) -> HybridOutcome {
    if validate_request(request).is_err() {
        return HybridOutcome::Refused {
            reason: "malformed_request",
        };
    }
    let Some(executable) = &config.executable else {
        return HybridOutcome::Refused {
            reason: "tool_not_configured",
        };
    };
    let model = match render_model(request) {
        Ok(model) => model,
        Err(()) => {
            return HybridOutcome::Refused {
                reason: "malformed_request",
            };
        }
    };
    if model.len() > MAX_SOLVER_STDIN_BYTES {
        return HybridOutcome::Incomplete {
            reason: "tool_output_limit_exceeded",
        };
    }
    let root = match CapabilityRoot::create() {
        Ok(root) => root,
        Err(()) => {
            return HybridOutcome::Incomplete {
                reason: "tool_unavailable",
            };
        }
    };
    if fs::write(root.path().join(MODEL_FILE), model.as_bytes()).is_err() {
        return HybridOutcome::Incomplete {
            reason: "tool_unavailable",
        };
    }
    let request_doc = match external_tool_request(
        executable,
        root.path(),
        Path::new(MODEL_FILE),
        config.environment.clone(),
        MAX_SOLVER_WALL_TIME_MS,
        MAX_SOLVER_STDIN_BYTES as u64,
        MAX_SOLVER_STDOUT_BYTES as u64,
        MAX_SOLVER_STDERR_BYTES as u64,
    ) {
        Ok(request_doc) => request_doc,
        Err(_) => {
            return HybridOutcome::Refused {
                reason: "malformed_request",
            };
        }
    };
    let result = match invoke_producer_execute(
        &config.producer_execute,
        root.path(),
        &request_doc.to_json_bytes(),
        Duration::from_millis(
            MAX_SOLVER_WALL_TIME_MS
                + PRODUCER_EXECUTE_CLEANUP_MS
                + PRODUCER_EXECUTE_STARTUP_MARGIN_MS,
        ),
        Duration::from_millis(PRODUCER_EXECUTE_FALLBACK_CLEANUP_MS),
    ) {
        Ok(result) => result,
        Err(error) if error.kind() == EvidenceErrorKind::ExecutorUnavailable => {
            return HybridOutcome::Refused {
                reason: "tool_unavailable",
            };
        }
        Err(error) if error.kind() == EvidenceErrorKind::ExecutorTimedOut => {
            return HybridOutcome::Incomplete {
                reason: "tool_timeout",
            };
        }
        Err(error) if error.kind() == EvidenceErrorKind::ExecutorOutputLimit => {
            return HybridOutcome::Incomplete {
                reason: "tool_output_limit_exceeded",
            };
        }
        Err(_) => {
            return HybridOutcome::Refused {
                reason: "result_schema_invalid",
            };
        }
    };
    match result.external_state() {
        ExternalExecutionState::Unavailable => HybridOutcome::Refused {
            reason: "tool_unavailable",
        },
        ExternalExecutionState::TimedOut => HybridOutcome::Incomplete {
            reason: "tool_timeout",
        },
        ExternalExecutionState::OutputLimit => HybridOutcome::Incomplete {
            reason: "tool_output_limit_exceeded",
        },
        ExternalExecutionState::Failed => HybridOutcome::Incomplete {
            reason: "tool_exit_nonzero",
        },
        ExternalExecutionState::Other => HybridOutcome::Refused {
            reason: "result_schema_invalid",
        },
        ExternalExecutionState::Completed => {
            if result.external_output_truncated() {
                return HybridOutcome::Incomplete {
                    reason: "tool_output_limit_exceeded",
                };
            }
            let Some(stdout) = result.external_stdout() else {
                return HybridOutcome::Refused {
                    reason: "result_schema_invalid",
                };
            };
            match parse_phaver_output(stdout, request) {
                Ok(steps) => HybridOutcome::Enclosure { steps },
                Err(ParseError::InvertedInterval) => HybridOutcome::Refused {
                    reason: "interval_inverted",
                },
                Err(ParseError::Schema) => HybridOutcome::Refused {
                    reason: "result_schema_invalid",
                },
                Err(ParseError::OutOfRange) => HybridOutcome::Incomplete {
                    reason: "result_out_of_range",
                },
                Err(ParseError::OutputLimit) => HybridOutcome::Incomplete {
                    reason: "tool_output_limit_exceeded",
                },
            }
        }
    }
}

fn validate_request(request: &HybridRequest) -> Result<(), ()> {
    if !valid_mode(&request.initial_mode)
        || request.initial_set.lower > request.initial_set.upper
        || request.flow_delta.lower > request.flow_delta.upper
        || request.transitions.len() > MAX_TRANSITIONS
        || request.horizon > 1_000_000
    {
        return Err(());
    }
    for transition in &request.transitions {
        if !valid_mode(&transition.from_mode)
            || !valid_mode(&transition.to_mode)
            || transition.guard.lower > transition.guard.upper
        {
            return Err(());
        }
    }
    let targets: BTreeSet<_> = request
        .transitions
        .iter()
        .map(|transition| transition.to_mode.as_str())
        .collect();
    if request.empty_targets.iter().any(|mode| {
        !valid_mode(mode) || mode == &request.initial_mode || !targets.contains(mode.as_str())
    }) {
        return Err(());
    }
    Ok(())
}

fn valid_mode(mode: &str) -> bool {
    !mode.is_empty()
        && mode.len() <= MAX_MODE_BYTES
        && mode
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !mode.as_bytes()[0].is_ascii_digit()
}

fn render_model(request: &HybridRequest) -> Result<String, ()> {
    validate_request(request)?;
    let mut modes = BTreeSet::from([request.initial_mode.clone()]);
    let mut by_source = BTreeMap::<String, Vec<&HybridTransition>>::new();
    for transition in &request.transitions {
        modes.insert(transition.from_mode.clone());
        modes.insert(transition.to_mode.clone());
        by_source
            .entry(transition.from_mode.clone())
            .or_default()
            .push(transition);
    }
    modes.extend(request.empty_targets.iter().cloned());
    let mut output = String::from("automaton A\ncontr_var: x,clk;\nsynclabs: a;\n");
    for mode in modes {
        output.push_str(&format!(
            "loc {mode}: while clk <= {} wait {{clk'==1 & x'>={} & x'<={}}};\n",
            request.horizon, request.flow_delta.lower, request.flow_delta.upper
        ));
        for transition in by_source.get(&mode).into_iter().flatten() {
            output.push_str(&format!(
                "  when x>={} & x<={} sync a do {{x'==x+({}) & clk'==clk}} goto {};\n",
                transition.guard.lower,
                transition.guard.upper,
                transition.reset_offset,
                transition.to_mode
            ));
        }
    }
    output.push_str(&format!(
        "initially: {} & clk==0 & x>={} & x<={};\nend\nR = A.reachable;\nR.remove(clk);\nR.print;\n",
        request.initial_mode, request.initial_set.lower, request.initial_set.upper
    ));
    Ok(output)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParseError {
    Schema,
    InvertedInterval,
    OutOfRange,
    OutputLimit,
}

fn parse_phaver_output(
    stdout: &[u8],
    request: &HybridRequest,
) -> Result<Vec<Vec<HybridEnclosure>>, ParseError> {
    let text = std::str::from_utf8(stdout)
        .map_err(|_| ParseError::Schema)?
        .trim();
    let body = text
        .strip_prefix("R = A.{")
        .and_then(|text| text.strip_suffix("};"))
        .ok_or(ParseError::Schema)?;
    if body.contains("};") {
        return Err(ParseError::Schema);
    }
    let modes = requested_modes(request);
    let mut found = BTreeMap::<String, Vec<((i128, i128), (i128, i128))>>::new();
    for result_mode in body.split(',') {
        let result_mode = result_mode.trim();
        if result_mode.is_empty() {
            return Err(ParseError::Schema);
        }
        let (mode, constraints) = result_mode.split_once('&').ok_or(ParseError::Schema)?;
        let mode = mode.trim();
        if !modes.contains(mode) || found.contains_key(mode) {
            return Err(ParseError::Schema);
        }
        let constraints = constraints.trim();
        let constraints = match (constraints.starts_with('('), constraints.ends_with(')')) {
            (true, true) if constraints.len() >= 2 => &constraints[1..constraints.len() - 1],
            (false, false) => constraints,
            _ => return Err(ParseError::Schema),
        };
        let mut intervals = Vec::new();
        let disjuncts: Vec<_> = constraints.split('|').map(str::trim).collect();
        if disjuncts.is_empty() || disjuncts.iter().any(|constraint| constraint.is_empty()) {
            return Err(ParseError::Schema);
        }
        let is_explicit_empty = disjuncts.as_slice() == ["false"];
        if disjuncts.contains(&"false") && !is_explicit_empty {
            return Err(ParseError::Schema);
        }
        if is_explicit_empty && !request.empty_targets.contains(mode) {
            return Err(ParseError::Schema);
        }
        if request.empty_targets.contains(mode) && !is_explicit_empty {
            return Err(ParseError::Schema);
        }
        for constraint in disjuncts {
            if let Some((lower, rest)) = constraint.split_once("<= x <=") {
                let upper = rest.trim();
                let lower = rational(lower.trim())?;
                let upper = rational(upper)?;
                if compare_rational(&lower, &upper) == std::cmp::Ordering::Greater {
                    return Err(ParseError::InvertedInterval);
                }
                intervals.push((lower, upper));
            } else if let Some(value) = constraint.strip_prefix("x ==") {
                let value = rational(value.trim())?;
                intervals.push((value, value));
            } else if constraint != "false" {
                return Err(ParseError::Schema);
            }
        }
        found.entry(mode.to_owned()).or_default().extend(intervals);
    }
    if found.is_empty() {
        return Err(ParseError::Schema);
    }
    if !found
        .get(&request.initial_mode)
        .is_some_and(|intervals| !intervals.is_empty())
    {
        return Err(ParseError::Schema);
    }
    if modes
        .iter()
        .any(|mode| !found.contains_key(mode) && !request.empty_targets.contains(mode))
    {
        return Err(ParseError::Schema);
    }
    let mut global = BTreeMap::<String, ((i128, i128), (i128, i128))>::new();
    for (mode, intervals) in found {
        let Some(lower) = intervals
            .iter()
            .map(|interval| interval.0)
            .min_by(compare_rational)
        else {
            continue;
        };
        let Some(upper) = intervals
            .iter()
            .map(|interval| interval.1)
            .max_by(compare_rational)
        else {
            continue;
        };
        global.insert(mode, (lower, upper));
    }
    let step_count = usize::try_from(request.horizon)
        .unwrap_or(usize::MAX)
        .checked_add(1)
        .ok_or(ParseError::OutputLimit)?;
    let entry_count = step_count
        .checked_mul(modes.len())
        .ok_or(ParseError::OutputLimit)?;
    if entry_count > MAX_RESULT_ENTRIES {
        return Err(ParseError::OutputLimit);
    }
    let mut steps = Vec::with_capacity(step_count);
    for _ in 0..=request.horizon {
        let mut entries = Vec::with_capacity(modes.len());
        for mode in &modes {
            let interval = match global.get(mode) {
                Some((lower, upper)) => {
                    let widening = i128::from(request.error_bound);
                    let lower_shift = widening
                        .checked_mul(lower.1)
                        .ok_or(ParseError::OutOfRange)?;
                    let upper_shift = widening
                        .checked_mul(upper.1)
                        .ok_or(ParseError::OutOfRange)?;
                    let low = lower
                        .0
                        .checked_sub(lower_shift)
                        .ok_or(ParseError::OutOfRange)?;
                    let high = upper
                        .0
                        .checked_add(upper_shift)
                        .ok_or(ParseError::OutOfRange)?;
                    let integer_lower = div_ceil(low, lower.1);
                    let integer_upper = div_floor(high, upper.1);
                    if integer_lower > integer_upper {
                        None
                    } else {
                        Some(IntegerInterval {
                            lower: integer_lower,
                            upper: integer_upper,
                        })
                    }
                }
                None => None,
            };
            entries.push(HybridEnclosure {
                mode: mode.clone(),
                interval,
            });
        }
        steps.push(entries);
    }
    let wire = ToolResult {
        schema: RESULT_SCHEMA.to_owned(),
        requested_horizon: request.horizon,
        steps: steps
            .iter()
            .enumerate()
            .map(|(step, modes)| ToolStep {
                step: u64::try_from(step).unwrap_or(u64::MAX),
                modes: modes
                    .iter()
                    .map(|mode| ToolMode {
                        mode: mode.mode.clone(),
                        intervals: mode
                            .interval
                            .map(|interval| ToolInterval {
                                lower: interval.lower.to_string(),
                                upper: interval.upper.to_string(),
                            })
                            .into_iter()
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&wire).map_err(|_| ParseError::Schema)?;
    validate_result_document(request, &bytes)
}

fn requested_modes(request: &HybridRequest) -> BTreeSet<String> {
    let mut modes = BTreeSet::from([request.initial_mode.clone()]);
    for transition in &request.transitions {
        modes.insert(transition.from_mode.clone());
        modes.insert(transition.to_mode.clone());
    }
    modes.extend(request.empty_targets.iter().cloned());
    modes
}

fn rational(value: &str) -> Result<(i128, i128), ParseError> {
    let mut parts = value.split('/');
    let numerator = parts
        .next()
        .ok_or(ParseError::Schema)?
        .parse::<i128>()
        .map_err(|_| ParseError::Schema)?;
    let denominator = match parts.next() {
        Some(value) => value.parse::<i128>().map_err(|_| ParseError::Schema)?,
        None => 1,
    };
    if parts.next().is_some()
        || denominator <= 0
        || canonical_rational(value, numerator, denominator).is_err()
    {
        return Err(ParseError::Schema);
    }
    if numerator == i128::MIN {
        return Err(ParseError::OutOfRange);
    }
    Ok((numerator, denominator))
}

fn canonical_rational(value: &str, numerator: i128, denominator: i128) -> Result<(), ()> {
    if denominator == 1 {
        return (numerator.to_string() == value).then_some(()).ok_or(());
    }
    if numerator == 0 || gcd(numerator.unsigned_abs(), denominator as u128) != 1 {
        return Err(());
    }
    (format!("{numerator}/{denominator}") == value)
        .then_some(())
        .ok_or(())
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn compare_rational(left: &(i128, i128), right: &(i128, i128)) -> std::cmp::Ordering {
    let left_sign = left.0.signum();
    let right_sign = right.0.signum();
    match left_sign.cmp(&right_sign) {
        std::cmp::Ordering::Equal if left_sign == 0 => std::cmp::Ordering::Equal,
        std::cmp::Ordering::Equal if left_sign < 0 => {
            compare_positive((-left.0, left.1), (-right.0, right.1)).reverse()
        }
        std::cmp::Ordering::Equal => compare_positive(*left, *right),
        order => order,
    }
}

fn compare_positive(mut left: (i128, i128), mut right: (i128, i128)) -> std::cmp::Ordering {
    let mut reverse = false;
    loop {
        let left_whole = left.0 / left.1;
        let right_whole = right.0 / right.1;
        if left_whole != right_whole {
            let order = left_whole.cmp(&right_whole);
            return if reverse { order.reverse() } else { order };
        }
        let left_rem = left.0 % left.1;
        let right_rem = right.0 % right.1;
        match (left_rem == 0, right_rem == 0) {
            (true, true) => return std::cmp::Ordering::Equal,
            (true, false) => {
                return if reverse {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                };
            }
            (false, true) => {
                return if reverse {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
            (false, false) => {
                left = (left.1, left_rem);
                right = (right.1, right_rem);
                reverse = !reverse;
            }
        }
    }
}

fn div_floor(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    if numerator % denominator < 0 {
        quotient - 1
    } else {
        quotient
    }
}

fn div_ceil(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    if numerator % denominator > 0 {
        quotient + 1
    } else {
        quotient
    }
}

fn validate_result_document(
    request: &HybridRequest,
    bytes: &[u8],
) -> Result<Vec<Vec<HybridEnclosure>>, ParseError> {
    let result: ToolResult = serde_json::from_slice(bytes).map_err(|_| ParseError::Schema)?;
    if result.schema != RESULT_SCHEMA
        || result.requested_horizon != request.horizon
        || result.steps.len()
            != usize::try_from(request.horizon)
                .unwrap_or(usize::MAX)
                .saturating_add(1)
    {
        return Err(ParseError::Schema);
    }
    let expected = requested_modes(request);
    let mut decoded = Vec::with_capacity(result.steps.len());
    for (expected_step, step) in result.steps.into_iter().enumerate() {
        if step.step != u64::try_from(expected_step).map_err(|_| ParseError::Schema)? {
            return Err(ParseError::Schema);
        }
        let actual: BTreeSet<_> = step.modes.iter().map(|mode| mode.mode.as_str()).collect();
        let expected_refs: BTreeSet<_> = expected.iter().map(String::as_str).collect();
        if actual != expected_refs || actual.len() != step.modes.len() {
            return Err(ParseError::Schema);
        }
        let mut modes = Vec::with_capacity(step.modes.len());
        for mode in step.modes {
            let explicitly_empty = mode.intervals.is_empty();
            let mut hull: Option<IntegerInterval> = None;
            for interval in mode.intervals {
                let (lower, lower_denominator) = rational(&interval.lower)?;
                let (upper, upper_denominator) = rational(&interval.upper)?;
                if compare_rational(&(lower, lower_denominator), &(upper, upper_denominator))
                    == std::cmp::Ordering::Greater
                {
                    return Err(ParseError::InvertedInterval);
                }
                let lo = div_ceil(lower, lower_denominator);
                let hi = div_floor(upper, upper_denominator);
                if lo <= hi {
                    hull = Some(match hull {
                        Some(existing) => IntegerInterval {
                            lower: existing.lower.min(lo),
                            upper: existing.upper.max(hi),
                        },
                        None => IntegerInterval {
                            lower: lo,
                            upper: hi,
                        },
                    });
                }
            }
            if request.empty_targets.contains(&mode.mode) != explicitly_empty {
                return Err(ParseError::Schema);
            }
            modes.push(HybridEnclosure {
                mode: mode.mode,
                interval: hull,
            });
        }
        decoded.push(modes);
    }
    Ok(decoded)
}

struct CapabilityRoot(PathBuf);

impl CapabilityRoot {
    fn create() -> Result<Self, ()> {
        let id = ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("quire-analyze-phaver-{}-{id}", std::process::id()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(&root).map_err(|_| ())?;
        }
        #[cfg(not(unix))]
        fs::create_dir(&root).map_err(|_| ())?;
        Ok(Self(root))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for CapabilityRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::{parse_result_line, MAX_PRODUCER_STREAM_BYTES};
    use ix_trace_rs::trace;
    use serde_json::{json, Value};
    use std::{
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicU64, Ordering as AtomicOrdering},
    };

    static STUB_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct StubProducer(PathBuf);

    impl StubProducer {
        fn new(state: Value, stdout: &str, truncated: bool) -> Self {
            let result = json!({
                "protocol": "engineering-assurance.producer-execution-result/v1",
                "producer": {"name": "phaver", "version": "0.38", "executable": "/phaver"},
                "cancellationEvent": null,
                "timing": {"preflightNanos": 1, "executionNanos": null, "totalNanos": 1},
                "observedHost": null,
                "process": {
                    "terminalStatus": null,
                    "stdout": {"bytes": stdout.as_bytes(), "truncated": truncated},
                    "stderr": {"bytes": [], "truncated": false}
                },
                "artifacts": [],
                "state": state
            });
            let result = serde_json::to_string(&result).expect("serialize producer result");
            parse_result_line(format!("{result}\n").as_bytes())
                .expect("stub result uses producer-execute's result protocol");
            let script = format!("printf '%s\\n' {}\n", shell_quote(&result));
            Self::from_script(format!(
                "#!/bin/sh\nprintf 'called\\n' >> '{{calls}}'\nIFS= read -r request\nprintf '%s\\n' \"$request\" > '{{request}}'\n{script}"
            ))
        }

        fn sleeping() -> Self {
            Self::from_script(
                "#!/bin/sh\nexec 2>/dev/null\nprintf 'called\\n' >> '{calls}'\nIFS= read -r request\nprintf '%s\\n' \"$request\" > '{request}'\n/bin/sleep 20 &\nchild=$!\ntrap 'wait \"$child\"; exit' TERM\nprintf '%s %s\\n' \"$$\" \"$child\" > '{pid}'\nwait\n".into(),
            )
        }

        fn from_script(script: String) -> Self {
            let id = STUB_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "quire-analyze-hybrid-stub-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&root).expect("create stub directory");
            let calls = root.join("calls");
            let request = root.join("request.json");
            let pid = root.join("pid");
            let script = script
                .replace("{calls}", &calls.display().to_string())
                .replace("{request}", &request.display().to_string())
                .replace("{pid}", &pid.display().to_string());
            let executable = root.join("producer-execute");
            fs::write(&executable, script).expect("write stub producer");
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
                .expect("make stub executable");
            Self(root)
        }

        fn config(&self, tool: Option<PathBuf>) -> HybridToolConfig {
            HybridToolConfig {
                executable: tool,
                producer_execute: self.0.join("producer-execute"),
                environment: BTreeMap::from([("LD_LIBRARY_PATH".into(), "/phaver/runtime".into())]),
            }
        }

        fn call_count(&self) -> usize {
            fs::read_to_string(self.0.join("calls")).map_or(0, |calls| calls.lines().count())
        }

        fn request(&self) -> Value {
            serde_json::from_slice(
                &fs::read(self.0.join("request.json")).expect("captured request"),
            )
            .expect("request JSON")
        }

        fn pids(&self) -> Vec<u32> {
            fs::read_to_string(self.0.join("pid"))
                .expect("captured child pid")
                .split_whitespace()
                .map(|pid| {
                    pid.parse()
                        .expect("valid producer-execute and descendant pids")
                })
                .collect()
        }
    }

    impl Drop for StubProducer {
        fn drop(&mut self) {
            if std::env::var_os("KEEP_HYBRID_STUB").is_none() {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }

    fn shell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    fn descend_mut<'a>(value: &'a mut Value, component: &str) -> &'a mut Value {
        if value.is_array() {
            let index = component.parse::<usize>().expect("array index");
            value
                .as_array_mut()
                .expect("array")
                .get_mut(index)
                .expect("array member")
        } else {
            &mut value[component]
        }
    }

    fn request() -> HybridRequest {
        HybridRequest {
            initial_mode: "m".into(),
            initial_set: Interval { lower: 0, upper: 2 },
            flow_delta: Interval { lower: 0, upper: 0 },
            transitions: vec![HybridTransition {
                from_mode: "m".into(),
                to_mode: "t".into(),
                guard: Interval { lower: 0, upper: 2 },
                reset_offset: 5,
            }],
            empty_targets: BTreeSet::new(),
            horizon: 2,
            error_bound: 0,
        }
    }

    #[trace("TC-041", "FR-042-AC-5", "FR-042-AC-6")]
    #[test]
    fn malformed_and_tool_none_requests_do_not_launch_a_process() {
        let config = HybridToolConfig {
            executable: None,
            producer_execute: "/definitely/missing/producer-execute".into(),
            environment: BTreeMap::new(),
        };
        assert_eq!(
            reach_hybrid(&request(), &config),
            HybridOutcome::Refused {
                reason: "tool_not_configured"
            }
        );
        let mut malformed = request();
        malformed.initial_set = Interval { lower: 4, upper: 3 };
        assert_eq!(
            reach_hybrid(&malformed, &config),
            HybridOutcome::Refused {
                reason: "malformed_request"
            }
        );
    }

    #[trace("TC-041", "FR-042-AC-5", "FR-042-AC-6")]
    #[test]
    fn producer_request_uses_artifact_binding_and_declared_limits_once() {
        let phaver = "R = A.{m & \n0 <= x <= 2, t & \n5 <= x <= 7};\n";
        let stub = StubProducer::new(
            json!({"kind": "completed", "observation": {"exitCode": 0}}),
            phaver,
            false,
        );
        let outcome = reach_hybrid(&request(), &stub.config(Some("/absolute/phaver".into())));
        assert!(
            matches!(outcome, HybridOutcome::Enclosure { .. }),
            "stub outcome: {outcome:?}; directory: {:?}",
            stub.0
        );
        assert_eq!(stub.call_count(), 1);

        let request = stub.request();
        assert_eq!(request["producer"]["executable"], "/absolute/phaver");
        assert_eq!(
            request["arguments"],
            json!([
                {"kind": "literal", "value": "-v000000"},
                {"kind": "input_artifact", "role": "model"}
            ])
        );
        assert_eq!(
            request["inputs"],
            json!([{"role": "model", "path": "model.pha"}])
        );
        assert_eq!(request["stdin"], json!({"kind": "null"}));
        assert_eq!(request["environment"]["LD_LIBRARY_PATH"], "/phaver/runtime");
        assert_eq!(request["budget"]["timeoutMillis"], MAX_SOLVER_WALL_TIME_MS);
        assert_eq!(request["budget"]["maxInputBytes"], MAX_SOLVER_STDIN_BYTES);
        assert_eq!(
            request["budget"]["maxStdoutBytes"],
            MAX_PRODUCER_STREAM_BYTES
        );
        assert_eq!(request["budget"]["maxStderrBytes"], MAX_SOLVER_STDERR_BYTES);
        assert_eq!(request["budget"]["maxOutputArtifacts"], 0);
        assert_eq!(request["budget"]["maxOutputBytes"], 0);
    }

    #[trace("TC-041", "FR-042-AC-7")]
    #[test]
    fn producer_failures_and_invalid_outputs_never_return_enclosures() {
        let cases = [
            (
                json!({"kind": "timed_out"}),
                "",
                false,
                HybridOutcome::Incomplete {
                    reason: "tool_timeout",
                },
            ),
            (
                json!({"kind": "failed", "reason": {"kind": "stdout_too_large"}}),
                "",
                false,
                HybridOutcome::Incomplete {
                    reason: "tool_output_limit_exceeded",
                },
            ),
            (
                json!({"kind": "failed", "reason": {"kind": "exit_code", "value": 17}}),
                "",
                false,
                HybridOutcome::Incomplete {
                    reason: "tool_exit_nonzero",
                },
            ),
            (
                json!({"kind": "unavailable"}),
                "",
                false,
                HybridOutcome::Refused {
                    reason: "tool_unavailable",
                },
            ),
            (
                json!({"kind": "completed", "observation": {"exitCode": 0}}),
                "not PHAVer output",
                false,
                HybridOutcome::Refused {
                    reason: "result_schema_invalid",
                },
            ),
            (
                json!({"kind": "completed", "observation": {"exitCode": 0}}),
                "R = A.{m & 0 <= x <= 2",
                false,
                HybridOutcome::Refused {
                    reason: "result_schema_invalid",
                },
            ),
            (
                json!({"kind": "completed", "observation": {"exitCode": 0}}),
                "R = A.{m & 2 <= x <= 1};\n",
                false,
                HybridOutcome::Refused {
                    reason: "interval_inverted",
                },
            ),
            (
                json!({"kind": "completed", "observation": {"exitCode": 0}}),
                "R = A.{m & \n0 <= x <= 2};\n",
                true,
                HybridOutcome::Incomplete {
                    reason: "tool_output_limit_exceeded",
                },
            ),
        ];
        for (state, stdout, truncated, expected) in cases {
            let stub = StubProducer::new(state, stdout, truncated);
            assert_eq!(
                reach_hybrid(&request(), &stub.config(Some("/absolute/phaver".into()))),
                expected
            );
            assert_eq!(stub.call_count(), 1);
        }

        let stub = StubProducer::new(
            json!({"kind": "completed", "observation": {"exitCode": 0}}),
            "",
            false,
        );
        assert_eq!(
            reach_hybrid(&request(), &stub.config(None)),
            HybridOutcome::Refused {
                reason: "tool_not_configured"
            }
        );
        assert_eq!(stub.call_count(), 0);
        let mut malformed = request();
        malformed.flow_delta.lower = 2;
        assert_eq!(
            reach_hybrid(&malformed, &stub.config(Some("/absolute/phaver".into()))),
            HybridOutcome::Refused {
                reason: "malformed_request"
            }
        );
        assert_eq!(stub.call_count(), 0);
    }

    #[trace("TC-041", "FR-042-AC-7")]
    #[test]
    fn empty_target_results_require_request_authorization_and_well_formed_disjuncts() {
        let authorized = {
            let mut request = request();
            request.empty_targets.insert("t".into());
            request
        };
        let outputs = [
            ("R = A.{m & 0 <= x <= 2, t & false};\n", true),
            ("R = A.{m & 0 <= x <= 2};\n", true),
            ("R = A.{m & 0 <= x <= 2, t & 5 <= x <= 7};\n", false),
        ];
        for (stdout, accepted) in outputs {
            let stub = StubProducer::new(
                json!({"kind": "completed", "observation": {"exitCode": 0}}),
                stdout,
                false,
            );
            let outcome = reach_hybrid(&authorized, &stub.config(Some("/absolute/phaver".into())));
            assert_eq!(matches!(outcome, HybridOutcome::Enclosure { .. }), accepted);
            if let HybridOutcome::Enclosure { steps } = outcome {
                assert!(steps
                    .iter()
                    .all(|step| step[1].mode == "t" && step[1].interval.is_none()));
            }
            assert_eq!(stub.call_count(), 1);
        }

        for stdout in [
            "R = A.{m & 0 <= x <= 2, t & false};\n",
            "R = A.{m & (0 <= x <= 2 | ), t & 5 <= x <= 7};\n",
            "R = A.{m & ( | 0 <= x <= 2), t & 5 <= x <= 7};\n",
            "R = A.{m & (0 <= x <= 2 || 1 <= x <= 2), t & 5 <= x <= 7};\n",
        ] {
            let stub = StubProducer::new(
                json!({"kind": "completed", "observation": {"exitCode": 0}}),
                stdout,
                false,
            );
            assert_eq!(
                reach_hybrid(&request(), &stub.config(Some("/absolute/phaver".into()))),
                HybridOutcome::Refused {
                    reason: "result_schema_invalid"
                }
            );
            assert_eq!(stub.call_count(), 1);
        }
    }

    #[trace("TC-041", "FR-042-AC-5", "FR-042-AC-7")]
    #[test]
    #[ignore = "verifies bounded producer-execute supervisor timeout and process cleanup"]
    fn sleeping_producer_is_terminated_within_supervisor_and_cleanup_budgets() {
        let stub = StubProducer::sleeping();
        let started = std::time::Instant::now();
        let outcome = reach_hybrid(&request(), &stub.config(Some("/absolute/phaver".into())));
        assert_eq!(
            outcome,
            HybridOutcome::Incomplete {
                reason: "tool_timeout"
            }
        );
        assert!(started.elapsed() < Duration::from_secs(7));
        assert_eq!(stub.call_count(), 1);
        for pid in stub.pids() {
            assert!(
                !Path::new(&format!("/proc/{pid}")).exists(),
                "producer-execute process-group member {pid} remains after cleanup"
            );
        }
    }

    #[trace("TC-041", "FR-042-AC-4", "FR-042-AC-7")]
    #[test]
    fn phaver_hulls_are_emitted_for_each_requested_step_and_mode() {
        let output = b"R = A.{m & \n(0 <= x <= 2 | 0 <= x <= 2),\nt & \n5 <= x <= 7};\n";
        let steps = parse_phaver_output(output, &request()).expect("valid PHAVer output");
        assert_eq!(steps.len(), 3);
        for step in steps {
            assert_eq!(step.len(), 2);
            assert_eq!(step[0].mode, "m");
            assert_eq!(
                step[0].interval,
                Some(IntegerInterval { lower: 0, upper: 2 })
            );
            assert_eq!(step[1].mode, "t");
            assert_eq!(
                step[1].interval,
                Some(IntegerInterval { lower: 5, upper: 7 })
            );
        }
        let underflow = HybridRequest {
            initial_mode: "m".into(),
            initial_set: Interval {
                lower: i64::MIN,
                upper: i64::MIN,
            },
            flow_delta: Interval {
                lower: -1,
                upper: 0,
            },
            transitions: vec![],
            empty_targets: BTreeSet::new(),
            horizon: 1,
            error_bound: 0,
        };
        let underflow_out = b"R = A.{m & \n(-9223372036854775809 <= x <= -9223372036854775808 | x == -9223372036854775808)};\n";
        assert_eq!(
            parse_phaver_output(underflow_out, &underflow),
            Ok(vec![
                vec![HybridEnclosure {
                    mode: "m".into(),
                    interval: Some(IntegerInterval {
                        lower: i128::from(i64::MIN) - 1,
                        upper: i128::from(i64::MIN)
                    }),
                }],
                vec![HybridEnclosure {
                    mode: "m".into(),
                    interval: Some(IntegerInterval {
                        lower: i128::from(i64::MIN) - 1,
                        upper: i128::from(i64::MIN)
                    }),
                }],
            ])
        );
    }

    #[trace("TC-041", "FR-042-AC-4", "FR-042-AC-7")]
    #[test]
    fn oracle_accepts_explicit_empty_enclosure_for_unreachable_target() {
        let mut unreachable = fit_case(
            "unreachable-target",
            "m",
            (0, 0),
            (0, 0),
            1,
            0,
            &[("m", "t", 1, 1, 0)],
        );
        unreachable.request.empty_targets.insert("t".into());
        assert_eq!(
            oracle_next(&unreachable.request, &oracle_initial(&unreachable.request)),
            BTreeMap::from([("m".to_owned(), vec![(0, 0)])])
        );
        let outcome = HybridOutcome::Enclosure {
            steps: (0..=1)
                .map(|_| {
                    vec![
                        HybridEnclosure {
                            mode: "m".into(),
                            interval: Some(IntegerInterval { lower: 0, upper: 0 }),
                        },
                        HybridEnclosure {
                            mode: "t".into(),
                            interval: None,
                        },
                    ]
                })
                .collect(),
        };
        assert_contains_oracle(&unreachable.request, &outcome);
    }

    #[trace("TC-041", "FR-042-AC-7")]
    #[test]
    fn result_schema_rejects_unknown_fields_duplicate_modes_and_bad_rationals() {
        let valid = serde_json::json!({
            "schema": RESULT_SCHEMA,
            "requested_horizon": 2,
            "steps": (0..=2).map(|step| serde_json::json!({
                "step": step,
                "modes": [
                    {"mode": "m", "intervals": [{"lower": "0", "upper": "2"}]},
                    {"mode": "t", "intervals": [{"lower": "5", "upper": "7"}]}
                ]
            })).collect::<Vec<_>>()
        });
        assert!(validate_result_document(&request(), &serde_json::to_vec(&valid).unwrap()).is_ok());

        let mut multiple_intervals = valid.clone();
        multiple_intervals["steps"][0]["modes"][0]["intervals"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"lower": "-3", "upper": "4"}));
        let hull = validate_result_document(
            &request(),
            &serde_json::to_vec(&multiple_intervals).unwrap(),
        )
        .unwrap();
        assert_eq!(
            hull[0][0].interval,
            Some(IntegerInterval {
                lower: -3,
                upper: 4
            })
        );

        let mut empty_request = request();
        empty_request.empty_targets.insert("t".into());
        let mut empty = valid.clone();
        for step in 0..=2 {
            empty["steps"][step]["modes"][1]["intervals"] = serde_json::json!([]);
        }
        assert_eq!(
            validate_result_document(&empty_request, &serde_json::to_vec(&empty).unwrap()).unwrap()
                [0][1]
                .interval,
            None
        );
        assert!(
            validate_result_document(&request(), &serde_json::to_vec(&empty).unwrap()).is_err()
        );

        let mut unknown = valid.clone();
        unknown["extra"] = serde_json::json!(true);
        assert!(
            validate_result_document(&request(), &serde_json::to_vec(&unknown).unwrap()).is_err()
        );

        for (path, field) in [
            (vec!["steps", "0"], "extra"),
            (vec!["steps", "0", "modes", "0"], "extra"),
            (vec!["steps", "0", "modes", "0", "intervals", "0"], "extra"),
        ] {
            let mut nested_unknown = valid.clone();
            let mut value = &mut nested_unknown;
            for component in &path {
                value = descend_mut(value, component);
            }
            value[field] = serde_json::json!(true);
            assert!(validate_result_document(
                &request(),
                &serde_json::to_vec(&nested_unknown).unwrap()
            )
            .is_err());
        }

        for (path, field) in [
            (vec![], "schema"),
            (vec!["steps", "0"], "step"),
            (vec!["steps", "0"], "modes"),
            (vec!["steps", "0", "modes", "0"], "mode"),
            (vec!["steps", "0", "modes", "0"], "intervals"),
            (vec!["steps", "0", "modes", "0", "intervals", "0"], "lower"),
        ] {
            let mut missing = valid.clone();
            let mut value = &mut missing;
            for component in &path {
                value = descend_mut(value, component);
            }
            value.as_object_mut().expect("object").remove(field);
            assert!(
                validate_result_document(&request(), &serde_json::to_vec(&missing).unwrap())
                    .is_err()
            );
        }

        let mut wrong_horizon = valid.clone();
        wrong_horizon["requested_horizon"] = serde_json::json!(3);
        assert!(
            validate_result_document(&request(), &serde_json::to_vec(&wrong_horizon).unwrap())
                .is_err()
        );
        let mut omitted_step = valid.clone();
        omitted_step["steps"].as_array_mut().unwrap().pop();
        assert!(
            validate_result_document(&request(), &serde_json::to_vec(&omitted_step).unwrap())
                .is_err()
        );
        let mut duplicate_step = valid.clone();
        duplicate_step["steps"][1]["step"] = serde_json::json!(0);
        assert!(validate_result_document(
            &request(),
            &serde_json::to_vec(&duplicate_step).unwrap()
        )
        .is_err());
        let mut missing_mode = valid.clone();
        missing_mode["steps"][0]["modes"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(
            validate_result_document(&request(), &serde_json::to_vec(&missing_mode).unwrap())
                .is_err()
        );

        let mut duplicate = valid.clone();
        duplicate["steps"][0]["modes"][1] = duplicate["steps"][0]["modes"][0].clone();
        assert!(
            validate_result_document(&request(), &serde_json::to_vec(&duplicate).unwrap()).is_err()
        );

        for invalid in ["-0", "+1", "01", "2/4", "1/1", "1/0"] {
            assert!(
                rational(invalid).is_err(),
                "accepted non-canonical {invalid}"
            );
        }
    }

    fn phaver_config() -> HybridToolConfig {
        HybridToolConfig {
            executable: Some("/home/peter/.local/share/phaver/bin/phaver".into()),
            producer_execute: "/home/peter/.local/bin/engineering-assurance".into(),
            environment: BTreeMap::from([(
                "LD_LIBRARY_PATH".into(),
                "/home/peter/.local/share/phaver/runtime/usr/lib/i386-linux-gnu".into(),
            )]),
        }
    }

    type OracleStates = BTreeMap<String, Vec<(i128, i128)>>;

    fn normalize_intervals(intervals: &mut Vec<(i128, i128)>) {
        intervals.sort_unstable();
        let mut normalized = Vec::with_capacity(intervals.len());
        for (lower, upper) in intervals.drain(..) {
            if let Some(last) = normalized.last_mut() {
                let last: &mut (i128, i128) = last;
                if lower <= last.1.saturating_add(1) {
                    last.1 = last.1.max(upper);
                    continue;
                }
            }
            normalized.push((lower, upper));
        }
        *intervals = normalized;
    }

    fn oracle_initial(request: &HybridRequest) -> OracleStates {
        BTreeMap::from([(
            request.initial_mode.clone(),
            vec![(
                i128::from(request.initial_set.lower),
                i128::from(request.initial_set.upper),
            )],
        )])
    }

    /// Propagate exact sets of integer states as normalized interval unions.
    /// Integer interval Minkowski sums, guard intersections, and additive resets
    /// preserve this representation without enumerating wide domains.
    fn oracle_next(request: &HybridRequest, states: &OracleStates) -> OracleStates {
        let delta_lower = i128::from(request.flow_delta.lower);
        let delta_upper = i128::from(request.flow_delta.upper);
        let mut next = OracleStates::new();
        for (mode, intervals) in states {
            let flowed: Vec<_> = intervals
                .iter()
                .map(|(lower, upper)| (lower + delta_lower, upper + delta_upper))
                .collect();
            next.entry(mode.clone())
                .or_default()
                .extend(flowed.iter().copied());
            for transition in request
                .transitions
                .iter()
                .filter(|transition| transition.from_mode == *mode)
            {
                let guard_lower = i128::from(transition.guard.lower);
                let guard_upper = i128::from(transition.guard.upper);
                let reset = i128::from(transition.reset_offset);
                for (lower, upper) in &flowed {
                    let lower = (*lower).max(guard_lower);
                    let upper = (*upper).min(guard_upper);
                    if lower <= upper {
                        next.entry(transition.to_mode.clone())
                            .or_default()
                            .push((lower + reset, upper + reset));
                    }
                }
            }
        }
        for intervals in next.values_mut() {
            normalize_intervals(intervals);
        }
        next
    }

    fn assert_contains_oracle(request: &HybridRequest, outcome: &HybridOutcome) {
        let HybridOutcome::Enclosure { steps } = outcome else {
            panic!("expected a complete fit enclosure for {request:?}, got {outcome:?}");
        };
        if request.horizon > 100_000
            && request.transitions.is_empty()
            && request.flow_delta == (Interval { lower: 0, upper: 0 })
        {
            assert_eq!(steps.len(), usize::try_from(request.horizon).unwrap() + 1);
            let lower = i128::from(request.initial_set.lower);
            let upper = i128::from(request.initial_set.upper);
            for (step_index, enclosures) in steps.iter().enumerate() {
                let enclosure = enclosures
                    .iter()
                    .find(|enclosure| enclosure.mode == request.initial_mode)
                    .and_then(|enclosure| enclosure.interval)
                    .unwrap_or_else(|| panic!("step {step_index} omitted invariant initial mode"));
                assert!(enclosure.lower <= lower && enclosure.upper >= upper);
            }
            return;
        }
        let mut expected = oracle_initial(request);
        assert_eq!(steps.len(), usize::try_from(request.horizon).unwrap() + 1);
        for (step_index, enclosures) in steps.iter().enumerate() {
            for (mode, states) in &expected {
                if states.is_empty() {
                    continue;
                }
                let enclosure = enclosures
                    .iter()
                    .find(|enclosure| enclosure.mode == *mode)
                    .and_then(|enclosure| enclosure.interval)
                    .unwrap_or_else(|| panic!("step {step_index} omitted reachable mode {mode}"));
                assert!(
                    states.iter().all(
                        |(lower, upper)| enclosure.lower <= *lower && *upper <= enclosure.upper
                    ),
                    "step {step_index} mode {mode} returned {enclosure:?}, exact oracle {states:?}"
                );
            }
            expected = oracle_next(request, &expected);
        }
    }

    struct NamedFitCase {
        name: String,
        request: HybridRequest,
    }

    fn fit_case(
        name: &str,
        initial_mode: &str,
        initial_set: (i64, i64),
        delta: (i64, i64),
        horizon: u64,
        error_bound: u64,
        transitions: &[(&str, &str, i64, i64, i64)],
    ) -> NamedFitCase {
        NamedFitCase {
            name: name.to_owned(),
            request: HybridRequest {
                initial_mode: initial_mode.into(),
                initial_set: Interval {
                    lower: initial_set.0,
                    upper: initial_set.1,
                },
                flow_delta: Interval {
                    lower: delta.0,
                    upper: delta.1,
                },
                transitions: transitions
                    .iter()
                    .map(|(from, to, lower, upper, reset)| HybridTransition {
                        from_mode: (*from).into(),
                        to_mode: (*to).into(),
                        guard: Interval {
                            lower: *lower,
                            upper: *upper,
                        },
                        reset_offset: *reset,
                    })
                    .collect(),
                empty_targets: BTreeSet::new(),
                horizon,
                error_bound,
            },
        }
    }

    #[trace("TC-041", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
    #[test]
    #[ignore = "runs the declared PHAVer fit cases through producer-execute"]
    fn tc_041_fit_cases_contain_independent_finite_oracle_states() {
        let mut cases = vec![
            fit_case(
                "F1_underflow",
                "m",
                (i64::MIN, i64::MIN),
                (-1, 0),
                1,
                0,
                &[],
            ),
            fit_case("F1_overflow", "m", (i64::MAX, i64::MAX), (0, 1), 1, 0, &[]),
            fit_case(
                "F2_reset_overflow",
                "m",
                (i64::MAX - 1, i64::MAX),
                (0, 0),
                1,
                0,
                &[("m", "t", i64::MAX - 1, i64::MAX, 1)],
            ),
            fit_case(
                "F3_widen_lower",
                "m",
                (i64::MIN, i64::MIN),
                (0, 0),
                0,
                1,
                &[],
            ),
            fit_case(
                "F3_widen_upper",
                "m",
                (i64::MAX, i64::MAX),
                (0, 0),
                0,
                1,
                &[],
            ),
            fit_case(
                "F4_partial_guard",
                "m",
                (0, 10),
                (0, 0),
                1,
                0,
                &[("m", "t", 8, 20, 100)],
            ),
            fit_case(
                "F4_covering_guard",
                "m",
                (0, 10),
                (0, 0),
                1,
                0,
                &[("m", "t", -5, 20, 100)],
            ),
            fit_case("F7_intermediate_steps", "m", (0, 0), (1, 1), 2, 0, &[]),
            fit_case("F8_error_bound_max", "m", (0, 0), (0, 0), 0, u64::MAX, &[]),
            fit_case("F9_million_horizon", "m", (0, 0), (0, 0), 1_000_000, 0, &[]),
            fit_case(
                "AC3_upper_guard_touch",
                "m",
                (0, 2),
                (1, 1),
                1,
                0,
                &[("m", "t", 3, 5, 0)],
            ),
            fit_case(
                "AC5_multiple_targets",
                "m",
                (0, 5),
                (0, 0),
                1,
                0,
                &[("m", "t", 0, 20, 0), ("m", "u", 0, 20, -50)],
            ),
            fit_case(
                "AC5_same_target_join",
                "m",
                (0, 5),
                (0, 0),
                1,
                0,
                &[("m", "t", 0, 20, 0), ("m", "t", 0, 20, 100)],
            ),
            fit_case(
                "AC5_duplicate_reordered_a",
                "m",
                (0, 5),
                (0, 0),
                1,
                0,
                &[
                    ("m", "t", 0, 20, 100),
                    ("m", "u", 0, 20, -50),
                    ("m", "t", 0, 20, 100),
                ],
            ),
            fit_case(
                "AC5_duplicate_reordered_b",
                "m",
                (0, 5),
                (0, 0),
                1,
                0,
                &[
                    ("m", "t", 0, 20, 100),
                    ("m", "t", 0, 20, 100),
                    ("m", "u", 0, 20, -50),
                ],
            ),
            fit_case(
                "AC6_exact_bounded_states",
                "m",
                (0, 2),
                (0, 1),
                2,
                0,
                &[("m", "t", 1, 2, 3)],
            ),
            fit_case("AC10_valid_error_widening", "m", (2, 4), (0, 0), 0, 5, &[]),
            fit_case(
                "AC11_unreachable_target",
                "m",
                (0, 0),
                (0, 0),
                1,
                0,
                &[("m", "t", 1, 1, 0)],
            ),
        ];
        cases
            .iter_mut()
            .find(|case| case.name == "AC11_unreachable_target")
            .expect("named unreachable-target fixture")
            .request
            .empty_targets
            .insert("t".into());
        let required = BTreeSet::from([
            "F1_underflow",
            "F1_overflow",
            "F2_reset_overflow",
            "F3_widen_lower",
            "F3_widen_upper",
            "F4_partial_guard",
            "F4_covering_guard",
            "F7_intermediate_steps",
            "F8_error_bound_max",
            "F9_million_horizon",
            "AC3_upper_guard_touch",
            "AC5_multiple_targets",
            "AC5_same_target_join",
            "AC5_duplicate_reordered_a",
            "AC5_duplicate_reordered_b",
            "AC6_exact_bounded_states",
            "AC10_valid_error_widening",
            "AC11_unreachable_target",
        ]);
        let executed: BTreeSet<_> = cases.iter().map(|case| case.name.as_str()).collect();
        assert_eq!(executed, required, "fit case census changed");
        let mut order_permutation_results = BTreeMap::new();
        for case in &cases {
            let outcome = reach_hybrid(&case.request, &phaver_config());
            if case.name.starts_with("AC5_duplicate_reordered_") {
                order_permutation_results.insert(case.name.as_str(), outcome.clone());
            }
            assert_contains_oracle(&case.request, &outcome);
        }
        assert_eq!(
            order_permutation_results.get("AC5_duplicate_reordered_a"),
            order_permutation_results.get("AC5_duplicate_reordered_b"),
            "duplicate/reordered transition fixture changed the PHAVer enclosure"
        );
        let extremes = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
        let errors = [0, 1, u64::MAX];
        let mut required_ac12 = BTreeSet::new();
        for initial in extremes {
            for reset in extremes {
                for error in errors {
                    required_ac12.insert(format!("AC12_{initial}_{reset}_{error}"));
                }
            }
        }
        let mut executed_ac12 = BTreeSet::new();
        for initial in extremes {
            for reset in extremes {
                for error in errors {
                    let name = format!("AC12_{initial}_{reset}_{error}");
                    assert!(
                        executed_ac12.insert(name.clone()),
                        "duplicate fit case {name}"
                    );
                    let case = fit_case(
                        &name,
                        "m",
                        (initial, initial),
                        (reset.min(0), reset.max(0)),
                        2,
                        error,
                        &[("m", "t", i64::MIN, i64::MAX, reset)],
                    );
                    assert_contains_oracle(
                        &case.request,
                        &reach_hybrid(&case.request, &phaver_config()),
                    );
                }
            }
        }
        assert_eq!(executed_ac12, required_ac12, "AC12 fit case census changed");
    }

    #[trace("TC-041", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
    #[test]
    #[ignore = "executes installed PHAVer through producer-execute"]
    fn selected_phaver_path_returns_a_containing_enclosure() {
        let outcome = reach_hybrid(&request(), &phaver_config());
        let HybridOutcome::Enclosure { steps } = outcome else {
            panic!("expected complete PHAVer enclosure, got {outcome:?}");
        };
        assert_eq!(steps.len(), 3);
        for step in steps {
            assert!(step.iter().any(|entry| {
                entry.mode == "m"
                    && entry
                        .interval
                        .is_some_and(|interval| interval.lower <= 0 && interval.upper >= 2)
            }));
            assert!(step.iter().any(|entry| {
                entry.mode == "t"
                    && entry
                        .interval
                        .is_some_and(|interval| interval.lower <= 5 && interval.upper >= 7)
            }));
        }
    }

    #[trace("TC-041", "FR-042-AC-5", "FR-042-AC-7")]
    #[test]
    #[ignore = "100,000 transition boundary exercises NFR-001 timeout"]
    fn f10_same_target_timeout_is_typed_incomplete_without_enclosure() {
        let oversized = HybridRequest {
            initial_mode: "m".into(),
            initial_set: Interval { lower: 0, upper: 0 },
            flow_delta: Interval { lower: 0, upper: 0 },
            horizon: 0,
            error_bound: 0,
            empty_targets: BTreeSet::new(),
            transitions: (0..100_000)
                .map(|_| HybridTransition {
                    from_mode: "m".into(),
                    to_mode: "t".into(),
                    guard: Interval { lower: 0, upper: 0 },
                    reset_offset: 0,
                })
                .collect(),
        };
        let started = std::time::Instant::now();
        let outcome = reach_hybrid(&oversized, &phaver_config());
        let elapsed = started.elapsed();
        eprintln!(
            "F10 same-target complete timeout result in {} ms",
            elapsed.as_millis()
        );
        assert!(
            elapsed
                < Duration::from_millis(
                    MAX_SOLVER_WALL_TIME_MS
                        + PRODUCER_EXECUTE_CLEANUP_MS
                        + PRODUCER_EXECUTE_STARTUP_MARGIN_MS
                ),
            "producer-execute timed_out result must arrive before the outer fallback deadline"
        );
        assert_eq!(
            outcome,
            HybridOutcome::Incomplete {
                reason: "tool_timeout"
            }
        );
    }

    #[trace("TC-041", "FR-042-AC-5", "FR-042-AC-7")]
    #[test]
    #[ignore = "100,000 distinct target modes exercise NFR-001 timeout"]
    fn f10_distinct_modes_timeout_is_typed_incomplete_without_enclosure() {
        let oversized = HybridRequest {
            initial_mode: "m".into(),
            initial_set: Interval { lower: 0, upper: 0 },
            flow_delta: Interval { lower: 0, upper: 0 },
            horizon: 0,
            error_bound: 0,
            empty_targets: BTreeSet::new(),
            transitions: (0..100_000)
                .map(|index| HybridTransition {
                    from_mode: "m".into(),
                    to_mode: format!("t{index}"),
                    guard: Interval { lower: 0, upper: 0 },
                    reset_offset: 0,
                })
                .collect(),
        };
        let started = std::time::Instant::now();
        let outcome = reach_hybrid(&oversized, &phaver_config());
        let elapsed = started.elapsed();
        eprintln!(
            "F10 distinct-target complete timeout result in {} ms",
            elapsed.as_millis()
        );
        assert!(
            elapsed
                < Duration::from_millis(
                    MAX_SOLVER_WALL_TIME_MS
                        + PRODUCER_EXECUTE_CLEANUP_MS
                        + PRODUCER_EXECUTE_STARTUP_MARGIN_MS
                ),
            "producer-execute timed_out result must arrive before the outer fallback deadline"
        );
        assert_eq!(
            outcome,
            HybridOutcome::Incomplete {
                reason: "tool_timeout"
            }
        );
    }
}
