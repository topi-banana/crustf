//! Recursive Fibonacci example.
//!
//! Emits a `Fibonacci.class` with two static methods — `fib(int):int`
//! (classic two-call recursion) and `main(String[]):void` (a `for`-loop
//! that prints `fib(0)` through `fib(9)`). The example exercises:
//!
//! * forward and backward short branches (`if_icmpge`, `goto`) resolved
//!   through the `CodeBuilder` label machinery,
//! * `invokestatic` self-recursion, so the constant pool has to resolve a
//!   method reference to the class being built,
//! * `iinc` + local slot access with `max_locals` inferred from use.
//!
//! ```text
//! cargo run -p crustf-examples --bin fibonacci
//! cargo run -p crustf-examples --bin fibonacci -- /tmp/myout
//! ```

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};

const CLASS_NAME: &str = "Fibonacci";

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
            println!();
            println!("expected output: 0 1 1 2 3 5 8 13 21 34 (one per line)");
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
        // static int fib(int n) { return n < 2 ? n : fib(n-1) + fib(n-2); }
        .method(
            MethodBuilder::new("fib", "(I)I")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    let recurse = c.label();
                    c.iload(0)
                        .iconst_2()
                        .if_icmpge(recurse)
                        .iload(0)
                        .ireturn()
                        .place(recurse)
                        .iload(0)
                        .iconst_1()
                        .isub()
                        .invokestatic(CLASS_NAME, "fib", "(I)I")
                        .iload(0)
                        .iconst_2()
                        .isub()
                        .invokestatic(CLASS_NAME, "fib", "(I)I")
                        .iadd()
                        .ireturn();
                }),
        )
        // public static void main(String[] args) {
        //     for (int i = 0; i < 10; i++) System.out.println(fib(i));
        // }
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    let head = c.label();
                    let end = c.label();
                    c.iconst_0()
                        .istore(1)
                        .place(head)
                        .iload(1)
                        .bipush(10)
                        .if_icmpge(end)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .iload(1)
                        .invokestatic(CLASS_NAME, "fib", "(I)I")
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .iinc(1, 1)
                        .goto(head)
                        .place(end)
                        .return_void();
                }),
        )
        .build()
}
