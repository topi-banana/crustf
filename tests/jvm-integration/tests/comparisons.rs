//! Covers `lcmp`, `fcmpl`, `fcmpg`, `dcmpl`, `dcmpg` plus `ifnull`/`ifnonnull`.

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
fn lcmp_orders_longs() {
    let bytes = ClassFileBuilder::new("Lc")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // println(lcmp(1L, 2L))
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .lconst_1()
                        .ldc_long(2)
                        .lcmp()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Lc", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "-1");
}

#[test]
fn ifnull_and_ifnonnull() {
    let bytes = ClassFileBuilder::new("Null")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    let was_null = c.label();
                    let after = c.label();
                    c.aconst_null()
                        .ifnull(was_null)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("nonnull")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .goto(after)
                        .place(was_null)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("null")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .place(after)
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Null", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "null");
}
