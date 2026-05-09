//! Helpers for driving a real JVM on bytes built by `crustf`.
//!
//! Each test produces a class file or JAR with `crustf` / `crustf-jar-builder`,
//! writes it to an isolated work directory and executes it with the `java`
//! binary found on PATH. The binary's stdout is captured and returned for
//! assertions.
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

/// Write `data` to a fresh per-call temp directory under `<tmp>/crustf-test-<pid>-<n>`
/// and return the full file path.
pub fn stage_file(file_name: &str, data: &[u8]) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("crustf-test-{}-{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join(file_name);
    std::fs::write(&path, data).expect("write staged file");
    path
}

/// Stage a class file and return the directory holding it (suitable for `-cp`).
pub fn stage(class_name: &str, bytes: &[u8]) -> PathBuf {
    stage_file(&format!("{class_name}.class"), bytes)
        .parent()
        .expect("stage_file path has a parent")
        .to_path_buf()
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

/// Run `java`, configured by `configure`, capture stdout, panic on non-zero
/// exit. Returns `None` when `java` is not available.
fn execute<F>(label: &str, configure: F) -> Option<String>
where
    F: FnOnce(&mut Command),
{
    if !java_available() {
        eprintln!("skipping {label}: java binary not on PATH");
        return None;
    }
    let mut cmd = Command::new("java");
    configure(&mut cmd);
    let output = cmd.output().expect("spawn java");
    assert!(
        output.status.success(),
        "java {label} exited {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Invoke `java -cp <dir> <class_name>` and capture stdout.
pub fn run(class_name: &str, bytes: &[u8], args: &[&str]) -> Option<String> {
    let dir = stage(class_name, bytes);
    execute(class_name, |cmd| {
        cmd.arg("-cp").arg(&dir).arg(class_name).args(args);
    })
}

/// Invoke `java -jar <path>` on `jar_bytes` and capture stdout.
pub fn run_jar(label: &str, jar_bytes: &[u8], args: &[&str]) -> Option<String> {
    let jar_path = stage_file(&format!("{label}.jar"), jar_bytes);
    execute(label, |cmd| {
        cmd.arg("-jar").arg(&jar_path).args(args);
    })
}

/// Variant of [`run`] for programs that read stdin. Distinct from `execute`
/// because it must keep the child alive to pipe stdin before waiting.
pub fn run_with_stdin(class_name: &str, bytes: &[u8], stdin_data: &str) -> Option<String> {
    if !java_available() {
        eprintln!("skipping {class_name}: java binary not on PATH");
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
    assert!(
        output.status.success(),
        "java {class_name} exited {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
