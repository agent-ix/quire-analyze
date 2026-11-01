use std::process::Command;

/// Trace: TC-008, FR-005-AC-1
#[test]
fn invalid_argv_preserves_exit_and_diagnostic_streams() {
    let output = Command::new(env!("CARGO_BIN_EXE_quire-analyze"))
        .arg("publish-report")
        .arg("--input")
        .output()
        .expect("run quire-analyze");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        b"quire-analyze: usage: quire-analyze publish-report --input REPORT.json --output NEW_REPORT.json\n"
    );
}
