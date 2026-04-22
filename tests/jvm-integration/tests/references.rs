//! Covers object creation (`new` + `invokespecial`), instance fields
//! (`getfield`/`putfield`), static fields (`getstatic`/`putstatic`),
//! `checkcast`, `instanceof`.

use crustf::spec::attribute::Attribute;
use crustf::{AccessFlags, ClassFileBuilder, FieldBuilder, FieldConstant, MethodBuilder};
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
fn static_constant_field_is_read_by_getstatic() {
    let bytes = ClassFileBuilder::new("Konst")
        .field(
            FieldBuilder::new("GREETING", "Ljava/lang/String;")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC | AccessFlags::FINAL)
                .constant_value(FieldConstant::String("hello there".to_string())),
        )
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .getstatic("Konst", "GREETING", "Ljava/lang/String;")
                        .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Konst", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "hello there");
}

#[test]
fn instance_field_read_and_write() {
    let bytes = ClassFileBuilder::new("Box")
        .field(FieldBuilder::new("value", "I").access_flags(AccessFlags::PUBLIC))
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    // Box b = new Box(); b.value = 41 + 1; println(b.value);
                    c.new_class("Box")
                        .dup()
                        .invokespecial("Box", "<init>", "()V")
                        .astore(1)
                        .aload(1)
                        .bipush(41)
                        .iconst_1()
                        .iadd()
                        .putfield("Box", "value", "I")
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .aload(1)
                        .getfield("Box", "value", "I")
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Box", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "42");
}

#[test]
fn checkcast_and_instanceof() {
    let bytes = ClassFileBuilder::new("Cast")
        .method(init())
        .method(
            MethodBuilder::new("main", "([Ljava/lang/String;)V")
                .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
                .code(|c| {
                    c.ldc_string("abc")
                        .astore(1)
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .aload(1)
                        .instanceof("java/lang/String")
                        .invokevirtual("java/io/PrintStream", "println", "(Z)V")
                        .getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                        .aload(1)
                        .checkcast("java/lang/CharSequence")
                        .invokeinterface("java/lang/CharSequence", "length", "()I")
                        .invokevirtual("java/io/PrintStream", "println", "(I)V")
                        .return_void();
                }),
        )
        .build()
        .unwrap();
    let Some(stdout) = harness::run("Cast", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "true\n3".replace("\\n", "\n").trim());
    // normalize Windows line endings too just in case
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["true", "3"]);
    drop(Attribute::Synthetic); // ensure attribute import is used
}
