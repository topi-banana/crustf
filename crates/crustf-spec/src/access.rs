//! Access flag bit sets (JVMS §4.1, §4.5, §4.6, §4.7.6, §4.7.24).

use bitflags::bitflags;

bitflags! {
    /// `access_flags` field of a [`ClassFile`](crate::ClassFile).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ClassAccess: u16 {
        const PUBLIC     = 0x0001;
        const FINAL      = 0x0010;
        const SUPER      = 0x0020;
        const INTERFACE  = 0x0200;
        const ABSTRACT   = 0x0400;
        const SYNTHETIC  = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM       = 0x4000;
        const MODULE     = 0x8000;
    }
}

bitflags! {
    /// Access flags for a [`Field`](crate::Field).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct FieldAccess: u16 {
        const PUBLIC    = 0x0001;
        const PRIVATE   = 0x0002;
        const PROTECTED = 0x0004;
        const STATIC    = 0x0008;
        const FINAL     = 0x0010;
        const VOLATILE  = 0x0040;
        const TRANSIENT = 0x0080;
        const SYNTHETIC = 0x1000;
        const ENUM      = 0x4000;
    }
}

bitflags! {
    /// Access flags for a [`Method`](crate::Method).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MethodAccess: u16 {
        const PUBLIC        = 0x0001;
        const PRIVATE       = 0x0002;
        const PROTECTED     = 0x0004;
        const STATIC        = 0x0008;
        const FINAL         = 0x0010;
        const SYNCHRONIZED  = 0x0020;
        const BRIDGE        = 0x0040;
        const VARARGS       = 0x0080;
        const NATIVE        = 0x0100;
        const ABSTRACT      = 0x0400;
        const STRICT        = 0x0800;
        const SYNTHETIC     = 0x1000;
    }
}

bitflags! {
    /// Access flags used in `InnerClasses` entries.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct InnerClassAccess: u16 {
        const PUBLIC     = 0x0001;
        const PRIVATE    = 0x0002;
        const PROTECTED  = 0x0004;
        const STATIC     = 0x0008;
        const FINAL      = 0x0010;
        const INTERFACE  = 0x0200;
        const ABSTRACT   = 0x0400;
        const SYNTHETIC  = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM       = 0x4000;
    }
}

bitflags! {
    /// Access flags for entries inside `MethodParameters`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ParameterAccess: u16 {
        const FINAL     = 0x0010;
        const SYNTHETIC = 0x1000;
        const MANDATED  = 0x8000;
    }
}

bitflags! {
    /// Module flags used in `Module` attributes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ModuleFlags: u16 {
        const OPEN      = 0x0020;
        const SYNTHETIC = 0x1000;
        const MANDATED  = 0x8000;
    }
}

bitflags! {
    /// Flags used inside module `requires`/`exports`/`opens` sub records.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ModuleRequiresFlags: u16 {
        const TRANSITIVE = 0x0020;
        const STATIC_PHASE = 0x0040;
        const SYNTHETIC = 0x1000;
        const MANDATED  = 0x8000;
    }
}

bitflags! {
    /// Flags for module `exports` records.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ModuleExportsFlags: u16 {
        const SYNTHETIC = 0x1000;
        const MANDATED  = 0x8000;
    }
}

bitflags! {
    /// Flags for module `opens` records.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ModuleOpensFlags: u16 {
        const SYNTHETIC = 0x1000;
        const MANDATED  = 0x8000;
    }
}
