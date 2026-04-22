//! Unified access flag type used by the builders.
//!
//! The JVM defines distinct flag sets per context (class / field / method /
//! inner class / parameter). To make the fluent builder ergonomic we expose
//! a single [`AccessFlags`] bag that combines every bit from every context
//! and then truncate to the context-specific set on the way to the spec
//! crate. Aliases cover bits that carry different names in different
//! contexts (for example `SUPER` is the class-level spelling of the
//! `SYNCHRONIZED` bit).

use bitflags::bitflags;
use crustf_spec::{ClassAccess, FieldAccess, InnerClassAccess, MethodAccess, ParameterAccess};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct AccessFlags: u16 {
        const PUBLIC       = 0x0001;
        const PRIVATE      = 0x0002;
        const PROTECTED    = 0x0004;
        const STATIC       = 0x0008;
        const FINAL        = 0x0010;
        const SYNCHRONIZED = 0x0020;
        const VOLATILE     = 0x0040;
        const TRANSIENT    = 0x0080;
        const NATIVE       = 0x0100;
        const INTERFACE    = 0x0200;
        const ABSTRACT     = 0x0400;
        const STRICT       = 0x0800;
        const SYNTHETIC    = 0x1000;
        const ANNOTATION   = 0x2000;
        const ENUM         = 0x4000;
        const MODULE       = 0x8000;
    }
}

impl AccessFlags {
    /// Class-level alias of the 0x0020 bit.
    pub const SUPER: Self = Self::SYNCHRONIZED;
    /// Method-level alias of the 0x0040 bit.
    pub const BRIDGE: Self = Self::VOLATILE;
    /// Method-level alias of the 0x0080 bit.
    pub const VARARGS: Self = Self::TRANSIENT;
    /// Parameter/module-level alias of the 0x8000 bit.
    pub const MANDATED: Self = Self::MODULE;
    /// Module `requires` alias of the 0x0020 bit.
    pub const TRANSITIVE: Self = Self::SYNCHRONIZED;
    /// Module `requires` alias of the 0x0040 bit.
    pub const STATIC_PHASE: Self = Self::VOLATILE;
    /// Module `Module` attribute alias of the 0x0020 bit.
    pub const OPEN: Self = Self::SYNCHRONIZED;
}

macro_rules! impl_from_access_flags {
    ($($target:ty),+ $(,)?) => {
        $(
            impl From<AccessFlags> for $target {
                fn from(a: AccessFlags) -> Self {
                    Self::from_bits_truncate(a.bits())
                }
            }
        )+
    };
}

impl_from_access_flags!(
    ClassAccess,
    FieldAccess,
    MethodAccess,
    InnerClassAccess,
    ParameterAccess,
);
