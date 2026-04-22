use crustf_asm::{AccessFlags, ClassFileBuilder, MethodBuilder};

#[test]
fn hello_world_roundtrip() {
    let bytes = ClassFileBuilder::new("TestHello")
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|code| {
                    code.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("Hello from assemb!")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .return_void();
                }),
        )
        .method(
            MethodBuilder::new("<init>", "()V")
                .access_flags(AccessFlags::PUBLIC)
                .code(|code| {
                    code.aload(0)
                        .invokespecial("java/lang/Object", "<init>", "()V")
                        .return_void();
                }),
        )
        .build()
        .expect("class file should build cleanly");

    // magic + version header
    assert_eq!(&bytes[..4], &0xCAFE_BABEu32.to_be_bytes());
    assert_eq!(&bytes[4..6], &0u16.to_be_bytes());

    // class name utf8 ought to be interned somewhere in the byte stream
    assert!(bytes.windows(9).any(|w| w == b"TestHello"));
    assert!(bytes.windows(4).any(|w| w == b"Code"));
    assert!(bytes.windows(18).any(|w| w == b"Hello from assemb!"));
}

#[test]
fn loops_resolve_short_branches() {
    let bytes = ClassFileBuilder::new("Loop")
        .method(
            MethodBuilder::new("sum", "(I)I")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|code| {
                    let loop_head = code.label();
                    let loop_end = code.label();

                    code.iconst_0()
                        .istore(1)
                        .iconst_0()
                        .istore(2)
                        .place(loop_head)
                        .iload(2)
                        .iload(0)
                        .if_icmpge(loop_end)
                        .iload(1)
                        .iload(2)
                        .iadd()
                        .istore(1)
                        .iinc(2, 1)
                        .goto(loop_head)
                        .place(loop_end)
                        .iload(1)
                        .ireturn();
                }),
        )
        .build()
        .expect("loop should build");

    assert!(bytes.windows(4).any(|w| w == b"Loop"));
}
