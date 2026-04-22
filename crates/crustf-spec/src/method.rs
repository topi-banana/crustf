//! `method_info` structure (JVMS §4.6).

use alloc::vec::Vec;

use crate::access::MethodAccess;
use crate::attribute::Attribute;

#[derive(Debug, Clone, Default)]
pub struct Method {
    pub access_flags: MethodAccess,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<Attribute>,
}
