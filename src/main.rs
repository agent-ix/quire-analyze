use std::{env, fs, path::PathBuf, process::ExitCode};

use quire_analyze::{
    publish_report_new, validate_report_document, DifferentialDisposition, MAX_REPORT_BYTES,
};
use serde_json::Value;

fn main() -> ExitCode {
    match run() {
        Ok(class) => ExitCode::from(class),
        Err((class, message)) => {
            eprintln!("quire-analyze: {message}");
            ExitCode::from(class)
        }
    }
}

fn run() -> Result<u8, (u8, String)> {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new("publish-report")) {
        return Err((2, usage()));
    }
    let mut input = None;
    let mut output = None;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--input") if input.is_none() => input = arguments.next().map(PathBuf::from),
            Some("--output") if output.is_none() => output = arguments.next().map(PathBuf::from),
            _ => return Err((2, usage())),
        }
    }
    let input = input.ok_or_else(|| (2, usage()))?;
    let output = output.ok_or_else(|| (2, usage()))?;
    let metadata = fs::metadata(&input).map_err(|error| (2, error.to_string()))?;
    if !metadata.is_file() || metadata.len() > MAX_REPORT_BYTES as u64 {
        return Err((2, "input report is not a bounded regular file".to_owned()));
    }
    let bytes = fs::read(&input).map_err(|error| (2, error.to_string()))?;
    let disposition = validate_report_document(&bytes).map_err(|error| (2, error))?;
    let exit_class = report_exit_class(&bytes, disposition).map_err(|error| (2, error))?;
    publish_report_new(&output, &bytes).map_err(|error| (4, error.to_string()))?;
    eprintln!("quire-analyze: published {}", output.display());
    Ok(exit_class)
}

fn report_exit_class(bytes: &[u8], disposition: DifferentialDisposition) -> Result<u8, String> {
    if disposition != DifferentialDisposition::Agreement {
        return Ok(3);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    match value["differential"]["agreedStatus"].as_str() {
        Some("satisfied") => Ok(0),
        Some("refuted") => Ok(1),
        _ => Err("agreement report has no conclusive agreed status".to_owned()),
    }
}

fn usage() -> String {
    "usage: quire-analyze publish-report --input REPORT.json --output NEW_REPORT.json".to_owned()
}
