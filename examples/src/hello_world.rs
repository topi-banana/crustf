//! Hello World example for `crustf`.
//!
//! Builds a `HelloWorld.class` that prints `Hello, world! (from crustf)`
//! when invoked as a Java entry point, writes it to an output directory,
//! and prints the `java` command needed to run it.
//!
//! ```text
//! cargo run -p crustf-examples --bin hello_world
//! cargo run -p crustf-examples --bin hello_world -- /tmp/myout
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};

fn main() -> ExitCode {
    let bytes = match build_class() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to build class: {e}");
            return ExitCode::FAILURE;
        }
    };

    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir);
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("failed to create {}: {e}", out_dir.display());
        return ExitCode::FAILURE;
    }
    let path = out_dir.join("HelloWorld.class");
    if let Err(e) = fs::write(&path, &bytes) {
        eprintln!("failed to write {}: {e}", path.display());
        return ExitCode::FAILURE;
    }

    println!("wrote {} ({} bytes)", path.display(), bytes.len());
    println!("run with:");
    println!("    java -cp {} HelloWorld", out_dir.display());

    ExitCode::SUCCESS
}

fn build_class() -> crustf::Result<Vec<u8>> {
    ClassFileBuilder::new("HelloWorld")
        .method(
            MethodBuilder::new("<init>", "()V")
                .access_flags(AccessFlags::PUBLIC)
                .code(|c| {
                    c.aload(0)
                        .invokespecial("java/lang/Object", "<init>", "()V")
                        .return_void();
                }),
        )
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("Hello, world! (from crustf)")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .return_void();
                }),
        )
        .build()
}
