//! Covers branches (`ifXX`, `if_icmpXX`, `goto`), `iinc`, and `tableswitch` /
//! `lookupswitch`.

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
fn for_loop_sum() {
    let bytes = ClassFileBuilder::new("Loop")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    let head = c.label();
                    let end = c.label();
                    // int sum = 0; for (int i=1;i<=10;i++) sum += i;
                    c.iconst_0()
                        .istore(1) // sum
                        .iconst_1()
                        .istore(2) // i
                        .place(head)
                        .iload(2)
                        .bipush(10)
                        .if_icmpgt(end)
                        .iload(1)
                        .iload(2)
                        .iadd()
                        .istore(1)
                        .iinc(2, 1)
                        .goto(head)
                        .place(end)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .iload(1)
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Loop", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "55");
}

#[test]
fn tableswitch_selects_branch() {
    let bytes = ClassFileBuilder::new("Switcher")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    let case0 = c.label();
                    let case1 = c.label();
                    let case2 = c.label();
                    let default = c.label();
                    let end = c.label();

                    c.iconst_1()
                        .tableswitch(default, 0, vec![case0, case1, case2])
                        .place(case0)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("zero")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .goto(end)
                        .place(case1)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("one")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .goto(end)
                        .place(case2)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("two")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .goto(end)
                        .place(default)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("default")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .place(end)
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Switcher", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "one");
}

#[test]
fn lookupswitch_selects_branch() {
    let bytes = ClassFileBuilder::new("Lookup")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    let case_7 = c.label();
                    let case_42 = c.label();
                    let default = c.label();
                    let end = c.label();

                    c.bipush(42)
                        .lookupswitch(default, vec![(7, case_7), (42, case_42)])
                        .place(case_7)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("seven")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .goto(end)
                        .place(case_42)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("forty-two")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .goto(end)
                        .place(default)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("?")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .place(end)
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Lookup", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "forty-two");
}
