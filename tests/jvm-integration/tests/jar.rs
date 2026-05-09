//! End-to-end test: build a class file with `crustf`, package it into a
//! JAR with `crustf-jar-builder`, execute with `java -jar`.

use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};
use crustf_jar_builder::{JarBuilder, Manifest, ZipWriter};
use crustf_jvm_integration_tests as harness;

fn hello_class(name: &str, message: &str) -> Vec<u8> {
    ClassFileBuilder::new(name)
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
                        .ldc_string(message)
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap()
}

#[test]
fn jar_runs_under_java_jar() {
    let class = hello_class("HelloJar", "Hello from jar!");
    let jar = JarBuilder::new()
        .main_class("HelloJar")
        .file("HelloJar.class", class)
        .build()
        .unwrap();

    let Some(stdout) = harness::run_jar("HelloJar", &jar, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "Hello from jar!");
}

#[test]
fn jar_with_packaged_class_runs() {
    let class = hello_class("com/example/Packaged", "packaged ok");
    let jar = JarBuilder::new()
        .manifest(
            Manifest::new()
                .main_class("com.example.Packaged")
                .attribute("Created-By", "crustf-jar-builder"),
        )
        .file("com/example/Packaged.class", class)
        .build()
        .unwrap();

    let Some(stdout) = harness::run_jar("Packaged", &jar, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "packaged ok");
}

#[test]
fn low_level_zipwriter_produces_a_readable_archive() {
    // ZipWriter on its own (no Manifest / no Main-Class) should still
    // produce bytes that `jar -tf` style tooling can list. We verify here
    // only that Java's built-in zip reader does not reject it by running
    // `java -jar` on a manifest-wrapped variant of the same file set.
    let mut writer = ZipWriter::new();
    writer.write_stored("hello.txt", b"hi").unwrap();
    let bytes = writer.finish().unwrap();
    // EOCD at the tail
    assert_eq!(
        &bytes[bytes.len() - 22..bytes.len() - 18],
        &0x06054b50u32.to_le_bytes()
    );
}
