//! Build-time tests for the annotation builder.
//!
//! Each test goes through `build_class_file()` so we can match on the
//! resulting `Attribute` variants directly; substring checks on the byte
//! stream would still pass for a malformed attribute whose name happened
//! to be interned.

use crustf_asm::{
    AccessFlags, Annotation, ClassFileBuilder, ElementValue, FieldBuilder, MethodBuilder,
};
use crustf_spec::attribute::{Annotation as SpecAnnotation, Attribute};

fn init() -> MethodBuilder {
    MethodBuilder::new("<init>", "()V")
        .access_flags(AccessFlags::PUBLIC)
        .code(|c| {
            c.aload(0)
                .invokespecial("java/lang/Object", "<init>", "()V")
                .return_void();
        })
}

fn visible_annotations(attrs: &[Attribute]) -> Option<&[SpecAnnotation]> {
    attrs.iter().find_map(|a| match a {
        Attribute::RuntimeVisibleAnnotations(anns) => Some(anns.as_slice()),
        _ => None,
    })
}

fn invisible_annotations(attrs: &[Attribute]) -> Option<&[SpecAnnotation]> {
    attrs.iter().find_map(|a| match a {
        Attribute::RuntimeInvisibleAnnotations(anns) => Some(anns.as_slice()),
        _ => None,
    })
}

#[test]
fn class_annotation_is_emitted_invisibly() {
    let cf = ClassFileBuilder::new("Annotated")
        .annotation(
            Annotation::invisible("Lorg/spongepowered/asm/mixin/Mixin;").element(
                "value",
                ElementValue::Array(vec![ElementValue::Class(
                    "Lnet/minecraft/server/MinecraftServer;".into(),
                )]),
            ),
        )
        .method(init())
        .build_class_file()
        .unwrap();

    let anns = invisible_annotations(&cf.attributes).expect("RuntimeInvisibleAnnotations");
    assert_eq!(anns.len(), 1);
    let type_desc = cf.constant_pool.utf8(anns[0].type_index).unwrap();
    assert_eq!(type_desc, "Lorg/spongepowered/asm/mixin/Mixin;");
    // No visible annotations slipped into the other attribute.
    assert!(visible_annotations(&cf.attributes).is_none());
}

#[test]
fn method_annotation_with_nested_annotation() {
    let cf = ClassFileBuilder::new("Annotated")
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
        .build_class_file()
        .unwrap();

    let on_run = cf
        .methods
        .iter()
        .find(|m| cf.constant_pool.utf8(m.name_index) == Some("onRun"));
    let attrs = &on_run.expect("onRun method").attributes;
    let anns = visible_annotations(attrs).expect("RuntimeVisibleAnnotations on onRun");
    assert_eq!(anns.len(), 1);
    assert_eq!(
        cf.constant_pool.utf8(anns[0].type_index).unwrap(),
        "Lorg/spongepowered/asm/mixin/injection/Inject;"
    );
    // Two elements: `method` and `at`.
    assert_eq!(anns[0].element_value_pairs.len(), 2);
}

#[test]
fn field_annotation_routes_to_field_info() {
    let cf = ClassFileBuilder::new("Annotated")
        .field(
            FieldBuilder::new("value", "I")
                .access_flags(AccessFlags::PUBLIC)
                .annotation(Annotation::invisible("Ljavax/annotation/Nullable;")),
        )
        .method(init())
        .build_class_file()
        .unwrap();

    let field = &cf.fields[0];
    let anns =
        invisible_annotations(&field.attributes).expect("RuntimeInvisibleAnnotations on field");
    assert_eq!(anns.len(), 1);
    assert_eq!(
        cf.constant_pool.utf8(anns[0].type_index).unwrap(),
        "Ljavax/annotation/Nullable;"
    );
}

#[test]
fn visible_and_invisible_land_in_separate_attributes() {
    let cf = ClassFileBuilder::new("Annotated")
        .annotation(Annotation::visible("Lv/V;"))
        .annotation(Annotation::invisible("Li/I;"))
        .method(init())
        .build_class_file()
        .unwrap();

    let visible = visible_annotations(&cf.attributes).expect("visible");
    let invisible = invisible_annotations(&cf.attributes).expect("invisible");
    assert_eq!(visible.len(), 1);
    assert_eq!(invisible.len(), 1);
    assert_eq!(
        cf.constant_pool.utf8(visible[0].type_index).unwrap(),
        "Lv/V;"
    );
    assert_eq!(
        cf.constant_pool.utf8(invisible[0].type_index).unwrap(),
        "Li/I;"
    );
}
