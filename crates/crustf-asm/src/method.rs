//! Builder for a single `method_info`.

use alloc::string::String;
use alloc::vec::Vec;

use crustf_spec::attribute::Attribute;
use crustf_spec::constant::ConstantPool;
use crustf_spec::descriptor::MethodDescriptor;
use crustf_spec::error::Result;
use crustf_spec::{Method, MethodAccess};

use crate::access::AccessFlags;
use crate::code::CodeBuilder;
use crate::util::push_attr;

#[derive(Debug, Default)]
pub struct MethodBuilder {
    pub(crate) name: String,
    pub(crate) descriptor: String,
    pub(crate) access_flags: AccessFlags,
    pub(crate) code: Option<CodeBuilder>,
    pub(crate) extra_attributes: Vec<Attribute>,
    pub(crate) exceptions: Vec<String>,
    pub(crate) signature: Option<String>,
    pub(crate) synthetic: bool,
    pub(crate) deprecated: bool,
}

impl MethodBuilder {
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

    /// Open a `CodeBuilder` scope. Abstract or native methods should skip
    /// this call.
    #[must_use]
    pub fn code<F: FnOnce(&mut CodeBuilder)>(mut self, f: F) -> Self {
        let mut cb = self.code.take().unwrap_or_default();
        f(&mut cb);
        self.code = Some(cb);
        self
    }

    #[must_use]
    pub fn exception(mut self, internal_name: impl Into<String>) -> Self {
        self.exceptions.push(internal_name.into());
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

    /// Attach a raw attribute to the emitted `method_info`.
    #[must_use]
    pub fn attribute(mut self, attr: Attribute) -> Self {
        self.extra_attributes.push(attr);
        self
    }

    pub(crate) fn into_method(self, pool: &mut ConstantPool) -> Result<Method> {
        let name_index = pool.intern_utf8(&self.name)?;
        let descriptor_index = pool.intern_utf8(&self.descriptor)?;
        let mut attributes = Vec::new();

        if let Some(code) = self.code {
            let access: MethodAccess = self.access_flags.into();
            let params = MethodDescriptor::parse(&self.descriptor)?.param_slots();
            let min_locals = params + u16::from(!access.contains(MethodAccess::STATIC));
            let code_attr = code.finish(pool, min_locals)?;
            push_attr(pool, &mut attributes, Attribute::Code(code_attr))?;
        }
        if !self.exceptions.is_empty() {
            let mut indices = Vec::with_capacity(self.exceptions.len());
            for e in &self.exceptions {
                indices.push(pool.intern_class(e)?);
            }
            push_attr(
                pool,
                &mut attributes,
                Attribute::Exceptions {
                    exception_index_table: indices,
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

        Ok(Method {
            access_flags: MethodAccess::from(self.access_flags),
            name_index,
            descriptor_index,
            attributes,
        })
    }
}
