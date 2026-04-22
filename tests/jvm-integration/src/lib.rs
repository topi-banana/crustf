//! Helpers for driving a real JVM on bytes built by `crustf`.
//!
//! Each test produces a class file with `crustf`, writes it to an isolated
//! work directory and executes it with the `java` binary found on PATH. The
//! binary's stdout is captured and returned for assertions.
//!
//! Tests skip with a clear note if `java` is not on PATH so the suite still
//! passes on hosts without a JDK; CI installs one explicitly.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static COUNTER: AtomicU64 = AtomicU64::new(0);
static JAVA_READY: OnceLock<bool> = OnceLock::new();

/// Write `bytes` as `<class_name>.class` under an isolated temp directory
/// and return the directory path.
pub fn stage(class_name: &str, bytes: &[u8]) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("crustf-test-{}-{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join(format!("{class_name}.class"));
    std::fs::write(&path, bytes).expect("write class file");
    dir
}

/// Returns `true` when `java` is on `PATH` and reports a version.
pub fn java_available() -> bool {
    *JAVA_READY.get_or_init(|| {
        Command::new("java")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Invoke `java` with the supplied class as main and capture stdout.
pub fn run(class_name: &str, bytes: &[u8], args: &[&str]) -> Option<String> {
    if !java_available() {
        eprintln!("skipping: java binary not on PATH");
        return None;
    }
    let dir = stage(class_name, bytes);
    let output = Command::new("java")
        .arg("-cp")
        .arg(&dir)
        .arg(class_name)
        .args(args)
        .output()
        .expect("spawn java");
    if !output.status.success() {
        panic!(
            "java {class_name} exited {:?}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        );
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Variant of [`run`] for programs that read stdin.
pub fn run_with_stdin(class_name: &str, bytes: &[u8], stdin_data: &str) -> Option<String> {
    if !java_available() {
        return None;
    }
    let dir = stage(class_name, bytes);
    let mut child = Command::new("java")
        .arg("-cp")
        .arg(&dir)
        .arg(class_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn java");
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_data.as_bytes()).ok();
    }
    let output = child.wait_with_output().expect("wait on java");
    if !output.status.success() {
        panic!(
            "java {class_name} exited {:?}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        );
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
