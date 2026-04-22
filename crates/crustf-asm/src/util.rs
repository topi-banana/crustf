//! Small helpers shared by the fluent builders.

use alloc::vec::Vec;

use crustf_spec::attribute::Attribute;
use crustf_spec::constant::ConstantPool;
use crustf_spec::error::Result;

/// Intern the attribute's JVMS name into `pool` (if any) and push the
/// attribute onto `attrs`. This is the canonical way to add an attribute
/// from inside a builder: the predefined variants already know their own
/// name via [`Attribute::name`], while `Attribute::Raw` carries its own
/// interned index and is left untouched.
pub(crate) fn push_attr(
    pool: &mut ConstantPool,
    attrs: &mut Vec<Attribute>,
    attr: Attribute,
) -> Result<()> {
    if let Some(name) = attr.name() {
        pool.intern_utf8(name)?;
    }
    attrs.push(attr);
    Ok(())
}
