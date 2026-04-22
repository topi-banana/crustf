//! Build-time tests for the annotation builder.

use crustf_asm::{
    AccessFlags, Annotation, ClassFileBuilder, ElementValue, FieldBuilder, MethodBuilder,
};

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
fn class_annotation_is_emitted_invisibly() {
    let bytes = ClassFileBuilder::new("Annotated")
        .annotation(
            Annotation::invisible("Lorg/spongepowered/asm/mixin/Mixin;").element(
                "value",
                ElementValue::Array(vec![ElementValue::Class(
                    "Lnet/minecraft/server/MinecraftServer;".into(),
                )]),
            ),
        )
        .method(init())
        .build()
        .unwrap();

    assert!(
        contains(&bytes, "RuntimeInvisibleAnnotations"),
        "expected RuntimeInvisibleAnnotations attribute"
    );
    assert!(contains(&bytes, "Lorg/spongepowered/asm/mixin/Mixin;"));
    assert!(contains(&bytes, "Lnet/minecraft/server/MinecraftServer;"));
}

#[test]
fn method_annotation_with_nested_annotation() {
    let bytes = ClassFileBuilder::new("Annotated")
        .method(init())
        .method(
            MethodBuilder::new("onRun", "()V")
                .access_flags(AccessFlags::PRIVATE)
                .annotation(
                    Annotation::visible("Lorg/spongepowered/asm/mixin/injection/Inject;")
                        .element("method", ElementValue::String("runServer".into()))
                        .element(
                            "at",
                            ElementValue::Array(vec![ElementValue::from(
                                Annotation::visible("Lorg/spongepowered/asm/mixin/injection/At;")
                                    .element("value", ElementValue::String("HEAD".into())),
                            )]),
                        ),
                )
                .code(|c| {
                    c.return_void();
                }),
        )
        .build()
        .unwrap();

    assert!(contains(&bytes, "RuntimeVisibleAnnotations"));
    assert!(contains(
        &bytes,
        "Lorg/spongepowered/asm/mixin/injection/Inject;"
    ));
    assert!(contains(
        &bytes,
        "Lorg/spongepowered/asm/mixin/injection/At;"
    ));
    assert!(contains(&bytes, "runServer"));
    assert!(contains(&bytes, "HEAD"));
}

#[test]
fn field_annotation_routes_to_field_info() {
    let bytes = ClassFileBuilder::new("Annotated")
        .field(
            FieldBuilder::new("value", "I")
                .access_flags(AccessFlags::PUBLIC)
                .annotation(Annotation::invisible("Ljavax/annotation/Nullable;")),
        )
        .method(init())
        .build()
        .unwrap();

    assert!(contains(&bytes, "Ljavax/annotation/Nullable;"));
    assert!(contains(&bytes, "RuntimeInvisibleAnnotations"));
}

#[test]
fn visible_and_invisible_land_in_separate_attributes() {
    let bytes = ClassFileBuilder::new("Annotated")
        .annotation(Annotation::visible("Lv/V;"))
        .annotation(Annotation::invisible("Li/I;"))
        .method(init())
        .build()
        .unwrap();

    assert!(contains(&bytes, "RuntimeVisibleAnnotations"));
    assert!(contains(&bytes, "RuntimeInvisibleAnnotations"));
    assert!(contains(&bytes, "Lv/V;"));
    assert!(contains(&bytes, "Li/I;"));
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}
