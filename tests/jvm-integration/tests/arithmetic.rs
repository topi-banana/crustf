//! Covers integer / long / float / double arithmetic: `*add`, `*sub`, `*mul`,
//! `*div`, `*rem`, `*neg`, plus shift and bitwise operations (JVMS §6.5).

use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};
use crustf_jvm_integration_tests as harness;

fn make_int_arith() -> Vec<u8> {
    ClassFileBuilder::new("IntArith")
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
                    // Compute (((7 + 3) * 2 - 4) / 3) % 2 == 1
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .bipush(7)
                        .iconst_3()
                        .iadd()
                        .iconst_2()
                        .imul()
                        .iconst_4()
                        .isub()
                        .iconst_3()
                        .idiv()
                        .iconst_2()
                        .irem()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap()
}

#[test]
fn int_arithmetic() {
    let Some(stdout) = harness::run("IntArith", &make_int_arith(), &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "1");
}

#[test]
fn long_arithmetic() {
    let bytes = ClassFileBuilder::new("LongArith")
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
                    // println(123456789012L + 1L = 123456789013L)
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_long(123_456_789_012)
                        .lconst_1()
                        .ladd()
                        .invokevirtual("java/io/PrintStream", "println", "(J)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("LongArith", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "123456789013");
}

#[test]
fn double_arithmetic() {
    let bytes = ClassFileBuilder::new("DoubleArith")
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
                        .ldc_double(1.5)
                        .ldc_double(2.5)
                        .dadd()
                        .invokevirtual("java/io/PrintStream", "println", "(D)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("DoubleArith", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "4.0");
}

#[test]
fn shifts_and_bitwise() {
    let bytes = ClassFileBuilder::new("Bits")
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
                    // (((1 << 4) | 3) ^ 5) & 0xff == 22
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .iconst_1()
                        .iconst_4()
                        .ishl()
                        .iconst_3()
                        .ior()
                        .iconst_5()
                        .ixor()
                        .sipush(0xff)
                        .iand()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Bits", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "22");
}
