//! The one way tests write an executable, shared across the workspace's test binaries with
//! `#[path = ".../clm-git/tests/exec/mod.rs"] mod exec;`.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Writes `text` to `path` as an executable, from a child process. Were this process to open
/// `path` for writing, any thread's fork, a test's or a library's, could inherit that handle
/// and hold it until its exec, and running `path` meanwhile fails with `Text file busy`;
/// renaming into place doesn't help, since the handle is on the same file.
pub fn write_exec(path: &Path, text: &str) {
    let mut child = Command::new("sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success(), "{}", path.display());
}
