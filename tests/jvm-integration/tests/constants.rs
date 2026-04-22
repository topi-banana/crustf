//! Covers every `ldc*` form, the tiny constant opcodes (`iconst_*`,
//! `aconst_null`, `dconst_*`, `fconst_*`, `lconst_*`) and `bipush`/`sipush`.

use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};
use crustf_jvm_integration_tests as harness;

fn init() -> MethodBuilder {
    MethodBuilder::new("<init>", "()V")
        .access_flags(AccessFlags::PUBLIC)
        .code(|c| {
            c.aload(0)
                .invokespecial("java/lang/Object", "<init>", "()V")
                .return_void();
        })
}

#[test]
fn tiny_int_constants() {
    let bytes = ClassFileBuilder::new("Tiny")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // println(-1 + 0 + 1 + 2 + 3 + 4 + 5) == 14
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .iconst_m1()
                        .iconst_0()
                        .iadd()
                        .iconst_1()
                        .iadd()
                        .iconst_2()
                        .iadd()
                        .iconst_3()
                        .iadd()
                        .iconst_4()
                        .iadd()
                        .iconst_5()
                        .iadd()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Tiny", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "14");
}

#[test]
fn ldc_forms() {
    let bytes = ClassFileBuilder::new("Ldc")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_int(100_000)
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_long(1_000_000_000_000)
                        .invokevirtual("java/io/PrintStream", "println", "(J)V")
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_float(1.25_f32)
                        .invokevirtual("java/io/PrintStream", "println", "(F)V")
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_double(2.5_f64)
                        .invokevirtual("java/io/PrintStream", "println", "(D)V")
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("str!")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Ldc", &bytes, &[]) else {
        return;
    };
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["100000", "1000000000000", "1.25", "2.5", "str!"]
    );
}

#[test]
fn bipush_and_sipush_limits() {
    let bytes = ClassFileBuilder::new("Push")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .bipush(-128)
                        .sipush(-32000)
                        .iadd()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Push", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "-32128");
}
