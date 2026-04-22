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
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};

const CLASS_NAME: &str = "HelloWorld";

fn main() -> ExitCode {
    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir);
    match run(&out_dir) {
        Ok(path) => {
            println!("wrote {}", path.display());
            println!("run with:");
            println!("    java -cp {} {CLASS_NAME}", out_dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(out_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    fs::create_dir_all(out_dir)?;
    let bytes = build_class()?;
    let path = out_dir.join(format!("{CLASS_NAME}.class"));
    fs::write(&path, &bytes)?;
    Ok(path)
}

fn build_class() -> crustf::Result<Vec<u8>> {
    ClassFileBuilder::new(CLASS_NAME)
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
