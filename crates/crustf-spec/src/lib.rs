//! # crustf-spec
//!
//! Low level data model and binary encoder for the JVM class file format
//! (`JVMS §4`). The crate only models the on-disk representation, it performs
//! no verification, no stack map inference and no byte-code semantics checks.
//! Those concerns belong to higher level crates such as `crustf-asm`.
//!
//! The crate is `no_std` friendly when the `std` feature is disabled, it still
//! depends on `alloc` for dynamic collections and is safe to compile for
//! `wasm32-unknown-unknown` and `wasm32-wasip1`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod access;
pub mod attribute;
pub mod classfile;
pub mod constant;
pub mod descriptor;
pub mod encode;
pub mod error;
pub mod field;
pub mod instruction;
pub mod method;
pub mod mutf8;
pub mod version;

pub use access::{
    ClassAccess, FieldAccess, InnerClassAccess, MethodAccess, ModuleExportsFlags, ModuleFlags,
    ModuleOpensFlags, ModuleRequiresFlags, ParameterAccess,
};
pub use attribute::{
    Annotation, Attribute, BootstrapMethod, CodeAttribute, ElementValue, ExceptionHandler,
    InnerClass, LineNumberEntry, LocalVariableEntry, LocalVariableTypeEntry, MethodParameter,
    ModuleAttribute, RecordComponent, StackMapFrame, TypeAnnotation, VerificationType,
};
pub use classfile::ClassFile;
pub use constant::{Constant, ConstantPool, ConstantTag, ReferenceKind};
pub use descriptor::{BaseType, FieldDescriptor, MethodDescriptor};
pub use encode::{encode, encode_into};
pub use error::{Error, Result};
pub use field::Field;
pub use instruction::{ArrayType, Instruction, WideInstruction};
pub use method::Method;
pub use version::*;
