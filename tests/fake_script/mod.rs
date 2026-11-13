//! Installs executable fake-solver scripts without racing a concurrent spawn.

use std::{fs, path::Path, process::Command};

/// Writes `body` to `path` as an executable script.
///
/// The destination is written by a short-lived `install` child that is reaped before this
/// returns. A thread that forks while this process holds a descriptor open for writing hands the
/// descriptor to its child until the child's exec, and executing the script in that window fails
/// with `ETXTBSY`. This process never opens the executable for writing, so no fork can inherit it.
pub fn install(path: &Path, body: &str) {
    let staging = path.with_extension("staging");
    fs::write(&staging, body).expect("write staging script");
    let status = Command::new("install")
        .args(["-m", "700"])
        .arg(&staging)
        .arg(path)
        .status()
        .expect("run install");
    assert!(status.success(), "install failed: {status}");
    fs::remove_file(&staging).expect("remove staging script");
}
