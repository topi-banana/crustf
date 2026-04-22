//! Predefined and user-defined class file attributes (JVMS §4.7).
//!
//! All predefined attributes from JVMS §4.7.1 Table 4.7-C are modelled as
//! dedicated variants of [`Attribute`]. Foreign or not-yet-supported
//! attributes fall back to [`Attribute::Raw`]. The encoder is responsible
//! for knowing how to serialize each.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::access::{
    InnerClassAccess, ModuleExportsFlags, ModuleFlags, ModuleOpensFlags, ModuleRequiresFlags,
    ParameterAccess,
};

/// Exception table entry inside `Code` (JVMS §4.7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionHandler {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    /// `0` denotes a "finally"-style catch-all (`any`).
    pub catch_type: u16,
}

/// Body of a `Code` attribute. The bytecode is stored as an opaque byte
/// stream; higher level crates produce this via the `Instruction` encoder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeAttribute {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<ExceptionHandler>,
    pub attributes: Vec<Attribute>,
}

/// JVMS §4.7.4 `stack_map_frame`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackMapFrame {
    /// Tags 0-63.
    Same { offset_delta: u8 },
    /// Tags 64-127.
    SameLocals1StackItem {
        offset_delta: u8,
        stack: VerificationType,
    },
    /// Tag 247.
    SameLocals1StackItemExtended {
        offset_delta: u16,
        stack: VerificationType,
    },
    /// Tags 248-250 (`k = 251 - frame_type`).
    Chop { chopped: u8, offset_delta: u16 },
    /// Tag 251.
    SameExtended { offset_delta: u16 },
    /// Tags 252-254 (`k = frame_type - 251`).
    Append {
        offset_delta: u16,
        locals: Vec<VerificationType>,
    },
    /// Tag 255.
    Full {
        offset_delta: u16,
        locals: Vec<VerificationType>,
        stack: Vec<VerificationType>,
    },
}

/// JVMS §4.7.4 `verification_type_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationType {
    Top,
    Integer,
    Float,
    Double,
    Long,
    Null,
    UninitializedThis,
    Object {
        cpool_index: u16,
    },
    /// The `u2` is the byte code offset of the `new` instruction creating
    /// the object being verified.
    Uninitialized {
        offset: u16,
    },
}

impl VerificationType {
    #[must_use]
    pub const fn tag(self) -> u8 {
        match self {
            Self::Top => 0,
            Self::Integer => 1,
            Self::Float => 2,
            Self::Double => 3,
            Self::Long => 4,
            Self::Null => 5,
            Self::UninitializedThis => 6,
            Self::Object { .. } => 7,
            Self::Uninitialized { .. } => 8,
        }
    }
}

/// Entry of the `InnerClasses` attribute (JVMS §4.7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InnerClass {
    pub inner_class_info_index: u16,
    /// `0` when the inner class is not a member of its enclosing class.
    pub outer_class_info_index: u16,
    /// `0` when the inner class is anonymous.
    pub inner_name_index: u16,
    pub inner_class_access_flags: InnerClassAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineNumberEntry {
    pub start_pc: u16,
    pub line_number: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVariableEntry {
    pub start_pc: u16,
    pub length: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub index: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVariableTypeEntry {
    pub start_pc: u16,
    pub length: u16,
    pub name_index: u16,
    pub signature_index: u16,
    pub index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapMethod {
    pub bootstrap_method_ref: u16,
    pub bootstrap_arguments: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodParameter {
    /// `0` denotes an unnamed parameter.
    pub name_index: u16,
    pub access_flags: ParameterAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordComponent {
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleRequires {
    pub requires_index: u16,
    pub requires_flags: ModuleRequiresFlags,
    /// `0` when the requires has no version.
    pub requires_version_index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleExports {
    pub exports_index: u16,
    pub exports_flags: ModuleExportsFlags,
    pub exports_to_index: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleOpens {
    pub opens_index: u16,
    pub opens_flags: ModuleOpensFlags,
    pub opens_to_index: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleProvides {
    pub provides_index: u16,
    pub provides_with_index: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleAttribute {
    pub module_name_index: u16,
    pub module_flags: ModuleFlags,
    /// `0` when the module has no version.
    pub module_version_index: u16,
    pub requires: Vec<ModuleRequires>,
    pub exports: Vec<ModuleExports>,
    pub opens: Vec<ModuleOpens>,
    pub uses: Vec<u16>,
    pub provides: Vec<ModuleProvides>,
}

/// Runtime annotation (JVMS §4.7.16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    pub type_index: u16,
    pub element_value_pairs: Vec<(u16, ElementValue)>,
}

/// `element_value` inside an annotation (JVMS §4.7.16.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementValue {
    Byte(u16),
    Char(u16),
    Double(u16),
    Float(u16),
    Int(u16),
    Long(u16),
    Short(u16),
    Boolean(u16),
    String(u16),
    Enum {
        type_name_index: u16,
        const_name_index: u16,
    },
    Class(u16),
    Annotation(Box<Annotation>),
    Array(Vec<ElementValue>),
}

impl ElementValue {
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::Byte(_) => b'B',
            Self::Char(_) => b'C',
            Self::Double(_) => b'D',
            Self::Float(_) => b'F',
            Self::Int(_) => b'I',
            Self::Long(_) => b'J',
            Self::Short(_) => b'S',
            Self::Boolean(_) => b'Z',
            Self::String(_) => b's',
            Self::Enum { .. } => b'e',
            Self::Class(_) => b'c',
            Self::Annotation(_) => b'@',
            Self::Array(_) => b'[',
        }
    }
}

/// JVMS §4.7.20 `type_annotation`. `target_info` is stored as raw bytes
/// because its layout depends on `target_type` (14 different shapes). Most
/// users will not write these by hand; higher-level crates can provide a
/// typed wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAnnotation {
    pub target_type: u8,
    pub target_info: Vec<u8>,
    pub target_path: Vec<(u8, u8)>,
    pub type_index: u16,
    pub element_value_pairs: Vec<(u16, ElementValue)>,
}

/// One of the predefined attributes (JVMS §4.7), or a raw escape hatch for
/// custom/unknown attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Attribute {
    ConstantValue {
        constantvalue_index: u16,
    },
    Code(CodeAttribute),
    StackMapTable(Vec<StackMapFrame>),
    Exceptions {
        exception_index_table: Vec<u16>,
    },
    InnerClasses(Vec<InnerClass>),
    EnclosingMethod {
        class_index: u16,
        /// `0` when the enclosing scope is not a method.
        method_index: u16,
    },
    Synthetic,
    Signature {
        signature_index: u16,
    },
    SourceFile {
        sourcefile_index: u16,
    },
    SourceDebugExtension(Vec<u8>),
    LineNumberTable(Vec<LineNumberEntry>),
    LocalVariableTable(Vec<LocalVariableEntry>),
    LocalVariableTypeTable(Vec<LocalVariableTypeEntry>),
    Deprecated,
    RuntimeVisibleAnnotations(Vec<Annotation>),
    RuntimeInvisibleAnnotations(Vec<Annotation>),
    RuntimeVisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeInvisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeVisibleTypeAnnotations(Vec<TypeAnnotation>),
    RuntimeInvisibleTypeAnnotations(Vec<TypeAnnotation>),
    AnnotationDefault(ElementValue),
    BootstrapMethods(Vec<BootstrapMethod>),
    MethodParameters(Vec<MethodParameter>),
    Module(Box<ModuleAttribute>),
    ModulePackages {
        package_indices: Vec<u16>,
    },
    ModuleMainClass {
        main_class_index: u16,
    },
    NestHost {
        host_class_index: u16,
    },
    NestMembers {
        classes: Vec<u16>,
    },
    Record(Vec<RecordComponent>),
    PermittedSubclasses {
        classes: Vec<u16>,
    },
    Raw {
        name_index: u16,
        info: Vec<u8>,
    },
}

impl Attribute {
    /// Resolve this attribute's `name_index` via the supplied pool.
    ///
    /// For [`Attribute::Raw`] the caller's own index is returned. For any
    /// predefined variant the pool must already contain the JVMS-mandated
    /// name (typically a builder pre-interns it when it pushes the
    /// attribute).
    pub fn resolve_name_index(
        &self,
        pool: &crate::constant::ConstantPool,
    ) -> crate::error::Result<u16> {
        if let Self::Raw { name_index, .. } = self {
            return Ok(*name_index);
        }
        let name = self.name().expect("non-Raw attribute always has a name");
        pool.find_utf8(name).ok_or_else(|| {
            crate::error::Error::MissingUtf8(alloc::string::ToString::to_string(name))
        })
    }

    /// Spec-mandated attribute name (JVMS §4.7.1). `None` for [`Attribute::Raw`]
    /// because a raw attribute carries its own interned name index.
    #[must_use]
    pub const fn name(&self) -> Option<&'static str> {
        Some(match self {
            Self::ConstantValue { .. } => "ConstantValue",
            Self::Code(_) => "Code",
            Self::StackMapTable(_) => "StackMapTable",
            Self::Exceptions { .. } => "Exceptions",
            Self::InnerClasses(_) => "InnerClasses",
            Self::EnclosingMethod { .. } => "EnclosingMethod",
            Self::Synthetic => "Synthetic",
            Self::Signature { .. } => "Signature",
            Self::SourceFile { .. } => "SourceFile",
            Self::SourceDebugExtension(_) => "SourceDebugExtension",
            Self::LineNumberTable(_) => "LineNumberTable",
            Self::LocalVariableTable(_) => "LocalVariableTable",
            Self::LocalVariableTypeTable(_) => "LocalVariableTypeTable",
            Self::Deprecated => "Deprecated",
            Self::RuntimeVisibleAnnotations(_) => "RuntimeVisibleAnnotations",
            Self::RuntimeInvisibleAnnotations(_) => "RuntimeInvisibleAnnotations",
            Self::RuntimeVisibleParameterAnnotations(_) => "RuntimeVisibleParameterAnnotations",
            Self::RuntimeInvisibleParameterAnnotations(_) => "RuntimeInvisibleParameterAnnotations",
            Self::RuntimeVisibleTypeAnnotations(_) => "RuntimeVisibleTypeAnnotations",
            Self::RuntimeInvisibleTypeAnnotations(_) => "RuntimeInvisibleTypeAnnotations",
            Self::AnnotationDefault(_) => "AnnotationDefault",
            Self::BootstrapMethods(_) => "BootstrapMethods",
            Self::MethodParameters(_) => "MethodParameters",
            Self::Module(_) => "Module",
            Self::ModulePackages { .. } => "ModulePackages",
            Self::ModuleMainClass { .. } => "ModuleMainClass",
            Self::NestHost { .. } => "NestHost",
            Self::NestMembers { .. } => "NestMembers",
            Self::Record(_) => "Record",
            Self::PermittedSubclasses { .. } => "PermittedSubclasses",
            Self::Raw { .. } => return None,
        })
    }
}
