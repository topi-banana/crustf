//! Covers numeric conversions: `i2l`, `i2f`, `i2d`, `l2i`, `l2f`, `l2d`,
//! `f2i`, `f2l`, `f2d`, `d2i`, `d2l`, `d2f`, `i2b`, `i2c`, `i2s`.

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
fn int_to_long_to_double() {
    let bytes = ClassFileBuilder::new("Conv")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // ((int)42).toLong.toDouble + 0.5 == 42.5
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .bipush(42)
                        .i2l()
                        .l2d()
                        .ldc_double(0.5)
                        .dadd()
                        .invokevirtual("java/io/PrintStream", "println", "(D)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Conv", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "42.5");
}

#[test]
fn double_to_int_truncates() {
    let bytes = ClassFileBuilder::new("Trunc")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_double(3.7)
                        .d2i()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Trunc", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "3");
}

#[test]
fn int_narrowing_i2b_i2s_i2c() {
    let bytes = ClassFileBuilder::new("Narrow")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // (byte) 0x01AB -> -85  (i2b keeps low 8 bits, sign-extends)
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .sipush(0x01AB)
                        .i2b()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        // (short) 0x1234 -> 4660
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_int(0xA1234)
                        .i2s()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        // (char) 0xFFFF -> 65535
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_int(-1)
                        .i2c()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Narrow", &bytes, &[]) else {
        return;
    };
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["-85", "4660", "65535"]);
}
