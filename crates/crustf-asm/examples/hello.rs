use std::fs;

use crustf_asm::{AccessFlags, ClassFileBuilder, MethodBuilder};

fn main() {
    let bytes = ClassFileBuilder::new("HelloCrustf")
        .method(
            MethodBuilder::new("<init>", "()V")
                .access_flags(AccessFlags::PUBLIC)
                .code(|code| {
                    code.aload(0)
                        .invokespecial("java/lang/Object", "<init>", "()V")
                        .return_void();
                }),
        )
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|code| {
                    code.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .ldc_string("Hello from crustf!")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .return_void();
                }),
        )
        .build()
        .expect("class file should build");

    let dir = std::env::args().nth(1).unwrap_or_else(|| "/tmp".into());
    let path = format!("{dir}/HelloCrustf.class");
    fs::write(&path, &bytes).expect("write class file");
    println!("wrote {path} ({} bytes)", bytes.len());
}
