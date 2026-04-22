//! Binary encoder that serializes a [`ClassFile`] to its on-disk form.
//!
//! All attribute bodies are written in strict JVMS §4 / §4.7 order. Unknown
//! attribute names are consulted against the class's constant pool so the
//! caller is responsible for having interned them before calling into this
//! module (use [`ConstantPool::intern_utf8`] for that).

use alloc::vec::Vec;

use crate::attribute::{
    Annotation, Attribute, BootstrapMethod, CodeAttribute, ElementValue, ExceptionHandler,
    InnerClass, LineNumberEntry, LocalVariableEntry, LocalVariableTypeEntry, MethodParameter,
    ModuleAttribute, ModuleExports, ModuleOpens, ModuleProvides, ModuleRequires, RecordComponent,
    StackMapFrame, TypeAnnotation, VerificationType,
};
use crate::classfile::{ClassFile, MAGIC};
use crate::constant::{Constant, ConstantPool};
use crate::error::{Error, Result};
use crate::field::Field;
use crate::method::Method;
use crate::mutf8;

/// Encode a class file to bytes.
pub fn encode(cf: &ClassFile) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(256);
    encode_into(cf, &mut buf)?;
    Ok(buf)
}

/// Encode a class file into the provided buffer.
pub fn encode_into(cf: &ClassFile, out: &mut Vec<u8>) -> Result<()> {
    write_u32(out, MAGIC);
    write_u16(out, cf.version.minor);
    write_u16(out, cf.version.major);

    write_u16(out, cf.constant_pool.count());
    for (_, entry) in cf.constant_pool.iter() {
        encode_constant(entry, out);
    }

    write_u16(out, cf.access_flags.bits());
    write_u16(out, cf.this_class);
    write_u16(out, cf.super_class);

    write_u16_len(out, cf.interfaces.len(), "interfaces")?;
    for i in &cf.interfaces {
        write_u16(out, *i);
    }

    write_u16_len(out, cf.fields.len(), "fields")?;
    for f in &cf.fields {
        encode_field(f, out, &cf.constant_pool)?;
    }

    write_u16_len(out, cf.methods.len(), "methods")?;
    for m in &cf.methods {
        encode_method(m, out, &cf.constant_pool)?;
    }

    encode_attributes(&cf.attributes, out, &cf.constant_pool)?;
    Ok(())
}

fn encode_field(f: &Field, out: &mut Vec<u8>, pool: &ConstantPool) -> Result<()> {
    write_u16(out, f.access_flags.bits());
    write_u16(out, f.name_index);
    write_u16(out, f.descriptor_index);
    encode_attributes(&f.attributes, out, pool)?;
    Ok(())
}

fn encode_method(m: &Method, out: &mut Vec<u8>, pool: &ConstantPool) -> Result<()> {
    write_u16(out, m.access_flags.bits());
    write_u16(out, m.name_index);
    write_u16(out, m.descriptor_index);
    encode_attributes(&m.attributes, out, pool)?;
    Ok(())
}

fn encode_constant(c: &Constant, out: &mut Vec<u8>) {
    out.push(c.tag() as u8);
    match c {
        Constant::Utf8(s) => {
            let bytes = mutf8::encode(s);
            write_u16(out, bytes.len() as u16);
            out.extend_from_slice(&bytes);
        }
        Constant::Integer(v) => out.extend_from_slice(&v.to_be_bytes()),
        Constant::Float(bits) => out.extend_from_slice(&bits.to_be_bytes()),
        Constant::Long(v) => out.extend_from_slice(&v.to_be_bytes()),
        Constant::Double(bits) => out.extend_from_slice(&bits.to_be_bytes()),
        Constant::Class { name_index }
        | Constant::Module { name_index }
        | Constant::Package { name_index } => {
            write_u16(out, *name_index);
        }
        Constant::String { string_index } => write_u16(out, *string_index),
        Constant::Fieldref {
            class_index,
            name_and_type_index,
        }
        | Constant::Methodref {
            class_index,
            name_and_type_index,
        }
        | Constant::InterfaceMethodref {
            class_index,
            name_and_type_index,
        } => {
            write_u16(out, *class_index);
            write_u16(out, *name_and_type_index);
        }
        Constant::NameAndType {
            name_index,
            descriptor_index,
        } => {
            write_u16(out, *name_index);
            write_u16(out, *descriptor_index);
        }
        Constant::MethodHandle {
            reference_kind,
            reference_index,
        } => {
            out.push(*reference_kind as u8);
            write_u16(out, *reference_index);
        }
        Constant::MethodType { descriptor_index } => write_u16(out, *descriptor_index),
        Constant::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        }
        | Constant::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => {
            write_u16(out, *bootstrap_method_attr_index);
            write_u16(out, *name_and_type_index);
        }
    }
}

fn encode_attributes(attrs: &[Attribute], out: &mut Vec<u8>, pool: &ConstantPool) -> Result<()> {
    write_u16_len(out, attrs.len(), "attributes")?;
    for a in attrs {
        encode_attribute(a, out, pool)?;
    }
    Ok(())
}

fn encode_attribute(attr: &Attribute, out: &mut Vec<u8>, pool: &ConstantPool) -> Result<()> {
    write_u16(out, attr.resolve_name_index(pool)?);

    let mut body = Vec::new();
    encode_attribute_body(attr, &mut body, pool)?;
    let len = u32::try_from(body.len())
        .map_err(|_| Error::Encoding(alloc::format!("attribute body {} bytes > u4", body.len())))?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&body);
    Ok(())
}

fn encode_attribute_body(attr: &Attribute, out: &mut Vec<u8>, pool: &ConstantPool) -> Result<()> {
    match attr {
        Attribute::Raw { info, .. } => out.extend_from_slice(info),
        Attribute::ConstantValue {
            constantvalue_index,
        } => write_u16(out, *constantvalue_index),
        Attribute::Code(code) => encode_code(code, out, pool)?,
        Attribute::StackMapTable(frames) => {
            write_u16_len(out, frames.len(), "StackMapTable frames")?;
            for frame in frames {
                encode_stack_map_frame(frame, out)?;
            }
        }
        Attribute::Exceptions {
            exception_index_table,
        } => {
            write_u16_len(out, exception_index_table.len(), "Exceptions")?;
            for i in exception_index_table {
                write_u16(out, *i);
            }
        }
        Attribute::InnerClasses(items) => {
            write_u16_len(out, items.len(), "InnerClasses")?;
            for ic in items {
                encode_inner_class(ic, out);
            }
        }
        Attribute::EnclosingMethod {
            class_index,
            method_index,
        } => {
            write_u16(out, *class_index);
            write_u16(out, *method_index);
        }
        Attribute::Synthetic | Attribute::Deprecated => {}
        Attribute::Signature { signature_index } => write_u16(out, *signature_index),
        Attribute::SourceFile { sourcefile_index } => write_u16(out, *sourcefile_index),
        Attribute::SourceDebugExtension(bytes) => out.extend_from_slice(bytes),
        Attribute::LineNumberTable(items) => {
            write_u16_len(out, items.len(), "LineNumberTable")?;
            for e in items {
                encode_line_number(e, out);
            }
        }
        Attribute::LocalVariableTable(items) => {
            write_u16_len(out, items.len(), "LocalVariableTable")?;
            for e in items {
                encode_local_variable(e, out);
            }
        }
        Attribute::LocalVariableTypeTable(items) => {
            write_u16_len(out, items.len(), "LocalVariableTypeTable")?;
            for e in items {
                encode_local_variable_type(e, out);
            }
        }
        Attribute::RuntimeVisibleAnnotations(anns)
        | Attribute::RuntimeInvisibleAnnotations(anns) => {
            write_u16_len(out, anns.len(), "annotations")?;
            for a in anns {
                encode_annotation(a, out)?;
            }
        }
        Attribute::RuntimeVisibleParameterAnnotations(params)
        | Attribute::RuntimeInvisibleParameterAnnotations(params) => {
            let n = u8::try_from(params.len()).map_err(|_| Error::TooManyEntries {
                what: "parameter annotations",
                count: params.len(),
            })?;
            out.push(n);
            for anns in params {
                write_u16_len(out, anns.len(), "annotations")?;
                for a in anns {
                    encode_annotation(a, out)?;
                }
            }
        }
        Attribute::RuntimeVisibleTypeAnnotations(anns)
        | Attribute::RuntimeInvisibleTypeAnnotations(anns) => {
            write_u16_len(out, anns.len(), "type annotations")?;
            for a in anns {
                encode_type_annotation(a, out)?;
            }
        }
        Attribute::AnnotationDefault(ev) => encode_element_value(ev, out)?,
        Attribute::BootstrapMethods(methods) => {
            write_u16_len(out, methods.len(), "BootstrapMethods")?;
            for m in methods {
                encode_bootstrap_method(m, out)?;
            }
        }
        Attribute::MethodParameters(params) => {
            let n = u8::try_from(params.len()).map_err(|_| Error::TooManyEntries {
                what: "MethodParameters",
                count: params.len(),
            })?;
            out.push(n);
            for p in params {
                encode_method_parameter(p, out);
            }
        }
        Attribute::Module(m) => encode_module(m, out)?,
        Attribute::ModulePackages { package_indices } => {
            write_u16_len(out, package_indices.len(), "ModulePackages")?;
            for p in package_indices {
                write_u16(out, *p);
            }
        }
        Attribute::ModuleMainClass { main_class_index } => write_u16(out, *main_class_index),
        Attribute::NestHost { host_class_index } => write_u16(out, *host_class_index),
        Attribute::NestMembers { classes } => {
            write_u16_len(out, classes.len(), "NestMembers")?;
            for c in classes {
                write_u16(out, *c);
            }
        }
        Attribute::Record(components) => {
            write_u16_len(out, components.len(), "Record")?;
            for c in components {
                encode_record_component(c, out, pool)?;
            }
        }
        Attribute::PermittedSubclasses { classes } => {
            write_u16_len(out, classes.len(), "PermittedSubclasses")?;
            for c in classes {
                write_u16(out, *c);
            }
        }
    }
    Ok(())
}

fn encode_code(code: &CodeAttribute, out: &mut Vec<u8>, pool: &ConstantPool) -> Result<()> {
    write_u16(out, code.max_stack);
    write_u16(out, code.max_locals);
    let len = u32::try_from(code.code.len()).map_err(|_| Error::CodeTooLarge(code.code.len()))?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&code.code);
    write_u16_len(out, code.exception_table.len(), "exception_table")?;
    for e in &code.exception_table {
        encode_exception_handler(e, out);
    }
    encode_attributes(&code.attributes, out, pool)?;
    Ok(())
}

fn encode_exception_handler(e: &ExceptionHandler, out: &mut Vec<u8>) {
    write_u16(out, e.start_pc);
    write_u16(out, e.end_pc);
    write_u16(out, e.handler_pc);
    write_u16(out, e.catch_type);
}

fn encode_stack_map_frame(frame: &StackMapFrame, out: &mut Vec<u8>) -> Result<()> {
    match frame {
        StackMapFrame::Same { offset_delta } => {
            if *offset_delta > 63 {
                return Err(Error::Encoding(alloc::format!(
                    "same_frame offset_delta {offset_delta} > 63",
                )));
            }
            out.push(*offset_delta);
        }
        StackMapFrame::SameLocals1StackItem {
            offset_delta,
            stack,
        } => {
            if *offset_delta > 63 {
                return Err(Error::Encoding(alloc::format!(
                    "same_locals_1 offset_delta {offset_delta} > 63",
                )));
            }
            out.push(64 + *offset_delta);
            encode_verification_type(*stack, out);
        }
        StackMapFrame::SameLocals1StackItemExtended {
            offset_delta,
            stack,
        } => {
            out.push(247);
            write_u16(out, *offset_delta);
            encode_verification_type(*stack, out);
        }
        StackMapFrame::Chop {
            chopped,
            offset_delta,
        } => {
            if !(1..=3).contains(chopped) {
                return Err(Error::Encoding(alloc::format!(
                    "chop_frame k={chopped} must be 1..=3",
                )));
            }
            out.push(251 - *chopped);
            write_u16(out, *offset_delta);
        }
        StackMapFrame::SameExtended { offset_delta } => {
            out.push(251);
            write_u16(out, *offset_delta);
        }
        StackMapFrame::Append {
            offset_delta,
            locals,
        } => {
            if !(1..=3).contains(&locals.len()) {
                return Err(Error::Encoding(alloc::format!(
                    "append_frame locals {} must be 1..=3",
                    locals.len(),
                )));
            }
            out.push(251 + locals.len() as u8);
            write_u16(out, *offset_delta);
            for v in locals {
                encode_verification_type(*v, out);
            }
        }
        StackMapFrame::Full {
            offset_delta,
            locals,
            stack,
        } => {
            out.push(255);
            write_u16(out, *offset_delta);
            write_u16_len(out, locals.len(), "full_frame locals")?;
            for v in locals {
                encode_verification_type(*v, out);
            }
            write_u16_len(out, stack.len(), "full_frame stack")?;
            for v in stack {
                encode_verification_type(*v, out);
            }
        }
    }
    Ok(())
}

fn encode_verification_type(v: VerificationType, out: &mut Vec<u8>) {
    out.push(v.tag());
    match v {
        VerificationType::Object { cpool_index } => write_u16(out, cpool_index),
        VerificationType::Uninitialized { offset } => write_u16(out, offset),
        _ => {}
    }
}

fn encode_inner_class(ic: &InnerClass, out: &mut Vec<u8>) {
    write_u16(out, ic.inner_class_info_index);
    write_u16(out, ic.outer_class_info_index);
    write_u16(out, ic.inner_name_index);
    write_u16(out, ic.inner_class_access_flags.bits());
}

fn encode_line_number(e: &LineNumberEntry, out: &mut Vec<u8>) {
    write_u16(out, e.start_pc);
    write_u16(out, e.line_number);
}

fn encode_local_variable(e: &LocalVariableEntry, out: &mut Vec<u8>) {
    write_u16(out, e.start_pc);
    write_u16(out, e.length);
    write_u16(out, e.name_index);
    write_u16(out, e.descriptor_index);
    write_u16(out, e.index);
}

fn encode_local_variable_type(e: &LocalVariableTypeEntry, out: &mut Vec<u8>) {
    write_u16(out, e.start_pc);
    write_u16(out, e.length);
    write_u16(out, e.name_index);
    write_u16(out, e.signature_index);
    write_u16(out, e.index);
}

fn encode_annotation(a: &Annotation, out: &mut Vec<u8>) -> Result<()> {
    write_u16(out, a.type_index);
    write_u16_len(out, a.element_value_pairs.len(), "element_value_pairs")?;
    for (name, ev) in &a.element_value_pairs {
        write_u16(out, *name);
        encode_element_value(ev, out)?;
    }
    Ok(())
}

fn encode_element_value(ev: &ElementValue, out: &mut Vec<u8>) -> Result<()> {
    out.push(ev.tag());
    match ev {
        ElementValue::Byte(i)
        | ElementValue::Char(i)
        | ElementValue::Double(i)
        | ElementValue::Float(i)
        | ElementValue::Int(i)
        | ElementValue::Long(i)
        | ElementValue::Short(i)
        | ElementValue::Boolean(i)
        | ElementValue::String(i) => write_u16(out, *i),
        ElementValue::Enum {
            type_name_index,
            const_name_index,
        } => {
            write_u16(out, *type_name_index);
            write_u16(out, *const_name_index);
        }
        ElementValue::Class(i) => write_u16(out, *i),
        ElementValue::Annotation(inner) => encode_annotation(inner, out)?,
        ElementValue::Array(items) => {
            write_u16_len(out, items.len(), "element_value array")?;
            for e in items {
                encode_element_value(e, out)?;
            }
        }
    }
    Ok(())
}

fn encode_type_annotation(a: &TypeAnnotation, out: &mut Vec<u8>) -> Result<()> {
    out.push(a.target_type);
    out.extend_from_slice(&a.target_info);
    let n = u8::try_from(a.target_path.len()).map_err(|_| Error::TooManyEntries {
        what: "type_path",
        count: a.target_path.len(),
    })?;
    out.push(n);
    for (kind, idx) in &a.target_path {
        out.push(*kind);
        out.push(*idx);
    }
    write_u16(out, a.type_index);
    write_u16_len(out, a.element_value_pairs.len(), "element_value_pairs")?;
    for (name, ev) in &a.element_value_pairs {
        write_u16(out, *name);
        encode_element_value(ev, out)?;
    }
    Ok(())
}

fn encode_bootstrap_method(m: &BootstrapMethod, out: &mut Vec<u8>) -> Result<()> {
    write_u16(out, m.bootstrap_method_ref);
    write_u16_len(out, m.bootstrap_arguments.len(), "bootstrap_arguments")?;
    for a in &m.bootstrap_arguments {
        write_u16(out, *a);
    }
    Ok(())
}

fn encode_method_parameter(p: &MethodParameter, out: &mut Vec<u8>) {
    write_u16(out, p.name_index);
    write_u16(out, p.access_flags.bits());
}

fn encode_module(m: &ModuleAttribute, out: &mut Vec<u8>) -> Result<()> {
    write_u16(out, m.module_name_index);
    write_u16(out, m.module_flags.bits());
    write_u16(out, m.module_version_index);
    write_u16_len(out, m.requires.len(), "module.requires")?;
    for r in &m.requires {
        encode_module_requires(r, out);
    }
    write_u16_len(out, m.exports.len(), "module.exports")?;
    for e in &m.exports {
        encode_module_exports(e, out)?;
    }
    write_u16_len(out, m.opens.len(), "module.opens")?;
    for o in &m.opens {
        encode_module_opens(o, out)?;
    }
    write_u16_len(out, m.uses.len(), "module.uses")?;
    for u in &m.uses {
        write_u16(out, *u);
    }
    write_u16_len(out, m.provides.len(), "module.provides")?;
    for p in &m.provides {
        encode_module_provides(p, out)?;
    }
    Ok(())
}

fn encode_module_requires(r: &ModuleRequires, out: &mut Vec<u8>) {
    write_u16(out, r.requires_index);
    write_u16(out, r.requires_flags.bits());
    write_u16(out, r.requires_version_index);
}

fn encode_module_exports(e: &ModuleExports, out: &mut Vec<u8>) -> Result<()> {
    write_u16(out, e.exports_index);
    write_u16(out, e.exports_flags.bits());
    write_u16_len(out, e.exports_to_index.len(), "exports_to")?;
    for t in &e.exports_to_index {
        write_u16(out, *t);
    }
    Ok(())
}

fn encode_module_opens(o: &ModuleOpens, out: &mut Vec<u8>) -> Result<()> {
    write_u16(out, o.opens_index);
    write_u16(out, o.opens_flags.bits());
    write_u16_len(out, o.opens_to_index.len(), "opens_to")?;
    for t in &o.opens_to_index {
        write_u16(out, *t);
    }
    Ok(())
}

fn encode_module_provides(p: &ModuleProvides, out: &mut Vec<u8>) -> Result<()> {
    write_u16(out, p.provides_index);
    write_u16_len(out, p.provides_with_index.len(), "provides_with")?;
    for w in &p.provides_with_index {
        write_u16(out, *w);
    }
    Ok(())
}

fn encode_record_component(
    c: &RecordComponent,
    out: &mut Vec<u8>,
    pool: &ConstantPool,
) -> Result<()> {
    write_u16(out, c.name_index);
    write_u16(out, c.descriptor_index);
    encode_attributes(&c.attributes, out, pool)
}

#[inline]
fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}

#[inline]
fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn write_u16_len(out: &mut Vec<u8>, n: usize, what: &'static str) -> Result<()> {
    let n = u16::try_from(n).map_err(|_| Error::TooManyEntries { what, count: n })?;
    write_u16(out, n);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::{ClassAccess, MethodAccess};
    use crate::version::JAVA_17;

    fn empty_class(name: &str) -> ClassFile {
        let mut pool = ConstantPool::new();
        let this_class = pool.intern_class(name).unwrap();
        let super_class = pool.intern_class("java/lang/Object").unwrap();
        ClassFile {
            version: JAVA_17,
            constant_pool: pool,
            access_flags: ClassAccess::PUBLIC | ClassAccess::SUPER,
            this_class,
            super_class,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new(),
        }
    }

    #[test]
    fn header_matches_spec() {
        let cf = empty_class("Foo");
        let bytes = encode(&cf).unwrap();
        assert_eq!(&bytes[..4], &MAGIC.to_be_bytes());
        assert_eq!(&bytes[4..6], &0u16.to_be_bytes()); // minor
        assert_eq!(&bytes[6..8], &JAVA_17.major.to_be_bytes());
    }

    #[test]
    fn counts_are_encoded_when_empty() {
        let cf = empty_class("Foo");
        let bytes = encode(&cf).unwrap();
        // After the fixed 24 byte prelude (magic+ver+cp_count+cp+access+this+super),
        // we expect interfaces_count=0, fields_count=0, methods_count=0, attributes_count=0.
        assert_eq!(&bytes[bytes.len() - 8..], &[0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn every_constant_pool_kind_encodes() {
        use crate::constant::{Constant, ReferenceKind};

        let mut pool = ConstantPool::new();
        let u = pool.intern(Constant::Utf8("x".into())).unwrap();
        pool.intern(Constant::Integer(1)).unwrap();
        pool.intern(Constant::Float(2.0f32.to_bits())).unwrap();
        pool.intern(Constant::Long(3)).unwrap();
        pool.intern(Constant::Double(4.0f64.to_bits())).unwrap();
        let c = pool.intern(Constant::Class { name_index: u }).unwrap();
        pool.intern(Constant::String { string_index: u }).unwrap();
        let nt = pool
            .intern(Constant::NameAndType {
                name_index: u,
                descriptor_index: u,
            })
            .unwrap();
        pool.intern(Constant::Fieldref {
            class_index: c,
            name_and_type_index: nt,
        })
        .unwrap();
        pool.intern(Constant::Methodref {
            class_index: c,
            name_and_type_index: nt,
        })
        .unwrap();
        pool.intern(Constant::InterfaceMethodref {
            class_index: c,
            name_and_type_index: nt,
        })
        .unwrap();
        let mref = pool
            .intern(Constant::Methodref {
                class_index: c,
                name_and_type_index: nt,
            })
            .unwrap();
        pool.intern(Constant::MethodHandle {
            reference_kind: ReferenceKind::InvokeStatic,
            reference_index: mref,
        })
        .unwrap();
        pool.intern(Constant::MethodType {
            descriptor_index: u,
        })
        .unwrap();
        pool.intern(Constant::Dynamic {
            bootstrap_method_attr_index: 0,
            name_and_type_index: nt,
        })
        .unwrap();
        pool.intern(Constant::InvokeDynamic {
            bootstrap_method_attr_index: 0,
            name_and_type_index: nt,
        })
        .unwrap();
        pool.intern(Constant::Module { name_index: u }).unwrap();
        pool.intern(Constant::Package { name_index: u }).unwrap();

        let cf = ClassFile {
            version: JAVA_17,
            constant_pool: pool,
            access_flags: ClassAccess::PUBLIC,
            this_class: c,
            super_class: c,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new(),
        };
        let bytes = encode(&cf).unwrap();
        assert_eq!(&bytes[..4], &MAGIC.to_be_bytes());
    }

    #[test]
    fn method_with_code_round_trips_through_encoder() {
        let mut pool = ConstantPool::new();
        let this_class = pool.intern_class("Foo").unwrap();
        let super_class = pool.intern_class("java/lang/Object").unwrap();
        let code_name = pool.intern_utf8("Code").unwrap();
        let _ = code_name;
        let name = pool.intern_utf8("m").unwrap();
        let desc = pool.intern_utf8("()V").unwrap();

        let code_bytes = alloc::vec![0xb1]; // return
        let cf = ClassFile {
            version: JAVA_17,
            constant_pool: pool,
            access_flags: ClassAccess::PUBLIC,
            this_class,
            super_class,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: alloc::vec![Method {
                access_flags: MethodAccess::PUBLIC,
                name_index: name,
                descriptor_index: desc,
                attributes: alloc::vec![Attribute::Code(CodeAttribute {
                    max_stack: 0,
                    max_locals: 1,
                    code: code_bytes,
                    exception_table: Vec::new(),
                    attributes: Vec::new(),
                })],
            }],
            attributes: Vec::new(),
        };
        let bytes = encode(&cf).unwrap();
        // Magic + version sanity
        assert_eq!(&bytes[..4], &MAGIC.to_be_bytes());
        // Class file contains the "Code" attribute name string as mUTF-8.
        assert!(bytes.windows(4).any(|w| w == b"Code"));
    }
}
