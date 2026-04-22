//! Builder for a single `field_info`.

use alloc::string::String;
use alloc::vec::Vec;

use crustf_spec::attribute::Attribute;
use crustf_spec::constant::{Constant, ConstantPool};
use crustf_spec::error::Result;
use crustf_spec::{Field, FieldAccess};

use crate::access::AccessFlags;
use crate::util::push_attr;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldConstant {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
}

#[derive(Debug, Default)]
pub struct FieldBuilder {
    pub(crate) name: String,
    pub(crate) descriptor: String,
    pub(crate) access_flags: AccessFlags,
    pub(crate) constant_value: Option<FieldConstant>,
    pub(crate) signature: Option<String>,
    pub(crate) synthetic: bool,
    pub(crate) deprecated: bool,
    pub(crate) extra_attributes: Vec<Attribute>,
}

impl FieldBuilder {
    #[must_use]
    pub fn new(name: impl Into<String>, descriptor: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            descriptor: descriptor.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn access_flags(mut self, flags: AccessFlags) -> Self {
        self.access_flags = flags;
        self
    }

    #[must_use]
    pub fn constant_value(mut self, value: FieldConstant) -> Self {
        self.constant_value = Some(value);
        self
    }

    #[must_use]
    pub fn signature(mut self, s: impl Into<String>) -> Self {
        self.signature = Some(s.into());
        self
    }

    #[must_use]
    pub fn synthetic(mut self) -> Self {
        self.synthetic = true;
        self
    }

    #[must_use]
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    #[must_use]
    pub fn attribute(mut self, attr: Attribute) -> Self {
        self.extra_attributes.push(attr);
        self
    }

    pub(crate) fn into_field(self, pool: &mut ConstantPool) -> Result<Field> {
        let name_index = pool.intern_utf8(&self.name)?;
        let descriptor_index = pool.intern_utf8(&self.descriptor)?;
        let mut attributes = Vec::new();

        if let Some(v) = self.constant_value {
            let idx = match v {
                FieldConstant::Int(i) => pool.intern(Constant::Integer(i))?,
                FieldConstant::Long(l) => pool.intern(Constant::Long(l))?,
                FieldConstant::Float(f) => pool.intern(Constant::Float(f.to_bits()))?,
                FieldConstant::Double(d) => pool.intern(Constant::Double(d.to_bits()))?,
                FieldConstant::String(s) => pool.intern_string(&s)?,
            };
            push_attr(
                pool,
                &mut attributes,
                Attribute::ConstantValue {
                    constantvalue_index: idx,
                },
            )?;
        }
        if let Some(sig) = self.signature.as_deref() {
            let signature_index = pool.intern_utf8(sig)?;
            push_attr(
                pool,
                &mut attributes,
                Attribute::Signature { signature_index },
            )?;
        }
        if self.synthetic {
            push_attr(pool, &mut attributes, Attribute::Synthetic)?;
        }
        if self.deprecated {
            push_attr(pool, &mut attributes, Attribute::Deprecated)?;
        }
        for a in self.extra_attributes {
            push_attr(pool, &mut attributes, a)?;
        }

        Ok(Field {
            access_flags: FieldAccess::from(self.access_flags),
            name_index,
            descriptor_index,
            attributes,
        })
    }
}
