//! Covers array creation and access: `newarray`, `anewarray`, `arraylength`,
//! `iastore`/`iaload`, `aastore`/`aaload`.

use crustf::{AccessFlags, ArrayType, ClassFileBuilder, MethodBuilder};
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
fn primitive_int_array() {
    let bytes = ClassFileBuilder::new("IntArr")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // int[] a = new int[3]; a[0]=10; a[1]=20; a[2]=30; println(a[0]+a[1]+a[2]);
                    c.iconst_3()
                        .newarray(ArrayType::Int)
                        .astore(1)
                        .aload(1)
                        .iconst_0()
                        .bipush(10)
                        .iastore()
                        .aload(1)
                        .iconst_1()
                        .bipush(20)
                        .iastore()
                        .aload(1)
                        .iconst_2()
                        .bipush(30)
                        .iastore()
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .aload(1)
                        .iconst_0()
                        .iaload()
                        .aload(1)
                        .iconst_1()
                        .iaload()
                        .iadd()
                        .aload(1)
                        .iconst_2()
                        .iaload()
                        .iadd()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("IntArr", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "60");
}

#[test]
fn reference_array_and_length() {
    let bytes = ClassFileBuilder::new("RefArr")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // String[] a = new String[2]; a[0]="x"; a[1]="y"; println(a.length);
                    c.iconst_2()
                        .anewarray("java/lang/String")
                        .astore(1)
                        .aload(1)
                        .iconst_0()
                        .ldc_string("x")
                        .aastore()
                        .aload(1)
                        .iconst_1()
                        .ldc_string("y")
                        .aastore()
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .aload(1)
                        .arraylength()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("RefArr", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "2");
}
