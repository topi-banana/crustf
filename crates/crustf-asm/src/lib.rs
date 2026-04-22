//! # crustf-asm
//!
//! Fluent, assembler-style builder that compiles down to the on-disk class
//! file format via `crustf-spec`. The builder hides the constant pool,
//! handles label and branch resolution, picks the narrowest available
//! encoding for loads/stores, and exposes a method body DSL for the full
//! JVM instruction set (JVMS §6.5).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod access;
pub mod annotation;
pub mod builder;
pub mod code;
pub mod field;
pub mod label;
pub mod method;
mod util;

pub use access::AccessFlags;
pub use annotation::{Annotation, ElementValue};
pub use builder::ClassFileBuilder;
pub use code::CodeBuilder;
pub use field::{FieldBuilder, FieldConstant};
pub use label::Label;
pub use method::MethodBuilder;

pub use crustf_spec::{
    ArrayType, Attribute, ClassAccess, ClassFile, Constant, ConstantPool, FieldAccess, Instruction,
    MethodAccess, ReferenceKind, Version, JAVA_17, JAVA_21, JAVA_25, JAVA_8,
};
