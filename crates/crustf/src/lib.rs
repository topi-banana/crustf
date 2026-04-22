//! # crustf
//!
//! `crustf` is a pure Rust JVM class file emitter with an assembler-style
//! fluent API. The crate is a thin umbrella that re-exports every public
//! type the user is likely to reach for:
//!
//! * [`asm`] — fluent class / method / field / code builders
//! * [`spec`] — on-disk data model (`ClassFile`, `ConstantPool`, `Attribute`,
//!   `Instruction`, …) and the binary encoder
//!
//! ```no_run
//! use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};
//!
//! let bytes = ClassFileBuilder::new("TestHello")
//!     .method(
//!         MethodBuilder::new("<init>", "()V")
//!             .access_flags(AccessFlags::PUBLIC)
//!             .code(|c| { c.aload(0).invokespecial("java/lang/Object", "<init>", "()V").return_void(); }),
//!     )
//!     .method(
//!         MethodBuilder::new("main", "([Ljava/lang/String;)V")
//!             .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
//!             .code(|c| {
//!                 c.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
//!                  .ldc_string("Hello from crustf!")
//!                  .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
//!                  .return_void();
//!             }),
//!     )
//!     .build()
//!     .unwrap();
//! # let _ = bytes;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub use crustf_asm as asm;
pub use crustf_spec as spec;

pub use crustf_asm::{
    AccessFlags, ClassFileBuilder, CodeBuilder, FieldBuilder, FieldConstant, Label, MethodBuilder,
};
pub use crustf_spec::{
    encode, Annotation, ArrayType, Attribute, BootstrapMethod, ClassAccess, ClassFile,
    CodeAttribute, Constant, ConstantPool, ConstantTag, ElementValue, Error, ExceptionHandler,
    Field, FieldAccess, FieldDescriptor, InnerClass, InnerClassAccess, Instruction,
    LineNumberEntry, LocalVariableEntry, LocalVariableTypeEntry, Method, MethodAccess,
    MethodDescriptor, MethodParameter, ModuleAttribute, ParameterAccess, RecordComponent,
    ReferenceKind, Result, StackMapFrame, TypeAnnotation, VerificationType, Version,
    WideInstruction, JAVA_11, JAVA_17, JAVA_21, JAVA_25, JAVA_8,
};
