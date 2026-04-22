//! Top level `ClassFile` structure (JVMS §4.1).

use alloc::vec::Vec;

use crate::access::ClassAccess;
use crate::attribute::Attribute;
use crate::constant::ConstantPool;
use crate::field::Field;
use crate::method::Method;
use crate::version::{Version, JAVA_17};

/// Magic header word written as `u4` at the start of every class file.
pub const MAGIC: u32 = 0xCAFE_BABE;

#[derive(Debug, Clone)]
pub struct ClassFile {
    pub version: Version,
    pub constant_pool: ConstantPool,
    pub access_flags: ClassAccess,
    pub this_class: u16,
    /// `0` denotes no super class, which is only legal for `java.lang.Object`
    /// and for `module-info` class files.
    pub super_class: u16,
    pub interfaces: Vec<u16>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
}

impl Default for ClassFile {
    fn default() -> Self {
        Self {
            version: JAVA_17,
            constant_pool: ConstantPool::new(),
            access_flags: ClassAccess::empty(),
            this_class: 0,
            super_class: 0,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new(),
        }
    }
}
