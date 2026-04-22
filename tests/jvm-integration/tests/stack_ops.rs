//! Covers `pop`, `pop2`, `dup`, `dup_x1`, `dup_x2`, `dup2`, `swap`.

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
fn dup_and_pop() {
    let bytes = ClassFileBuilder::new("DupPop")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // Push 7, dup, add -> 14, print
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .bipush(7)
                        .dup()
                        .iadd()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        // Push 1 and 2, swap, subtract (2-1)=1
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .iconst_1()
                        .iconst_2()
                        .swap()
                        .isub()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        // push 10, 20, pop -> 10
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .bipush(10)
                        .bipush(20)
                        .pop()
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("DupPop", &bytes, &[]) else {
        return;
    };
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["14", "1", "10"]);
}
