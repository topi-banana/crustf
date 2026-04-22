//! `field_info` structure (JVMS §4.5).

use alloc::vec::Vec;

use crate::access::FieldAccess;
use crate::attribute::Attribute;

#[derive(Debug, Clone, Default)]
pub struct Field {
    pub access_flags: FieldAccess,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<Attribute>,
}
