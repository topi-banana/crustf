//! Top level [`ClassFileBuilder`].

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crustf_spec::attribute::Attribute;
use crustf_spec::classfile::ClassFile;
use crustf_spec::constant::ConstantPool;
use crustf_spec::error::Result;
use crustf_spec::version::{Version, JAVA_5};
use crustf_spec::{encode, ClassAccess};

use crate::access::AccessFlags;
use crate::field::FieldBuilder;
use crate::method::MethodBuilder;
use crate::util::push_attr;

#[derive(Debug)]
pub struct ClassFileBuilder {
    this_name: String,
    super_name: Option<String>,
    access_flags: AccessFlags,
    version: Version,
    interfaces: Vec<String>,
    fields: Vec<FieldBuilder>,
    methods: Vec<MethodBuilder>,
    source_file: Option<String>,
    signature: Option<String>,
    nest_host: Option<String>,
    nest_members: Vec<String>,
    permitted_subclasses: Vec<String>,
    extra_attributes: Vec<Attribute>,
}

impl ClassFileBuilder {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            this_name: name.into(),
            super_name: Some("java/lang/Object".to_string()),
            // `ACC_SUPER` must always be set for class files produced by JDK 8+
            // (JVMS §4.1) otherwise the JVM rejects modern invokespecial semantics.
            access_flags: AccessFlags::PUBLIC | AccessFlags::SUPER,
            // Default to class file major 49 (Java 5) so builds with back
            // branches still pass verification without a StackMapTable.
            // Callers that need modern features (invokedynamic, dynamic
            // constants, sealed classes, ...) can upgrade with `.version(…)`
            // and supply `StackMapTable` attributes themselves.
            version: JAVA_5,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            source_file: None,
            signature: None,
            nest_host: None,
            nest_members: Vec::new(),
            permitted_subclasses: Vec::new(),
            extra_attributes: Vec::new(),
        }
    }

    #[must_use]
    pub fn super_class(mut self, internal_name: impl Into<String>) -> Self {
        self.super_name = Some(internal_name.into());
        self
    }

    /// Emit a class file with `super_class = 0`. Only legal for
    /// `java/lang/Object` and `module-info` (JVMS §4.1).
    #[must_use]
    pub fn no_super_class(mut self) -> Self {
        self.super_name = None;
        self
    }

    #[must_use]
    pub fn access_flags(mut self, flags: AccessFlags) -> Self {
        self.access_flags = flags;
        self
    }

    #[must_use]
    pub fn version(mut self, v: Version) -> Self {
        self.version = v;
        self
    }

    #[must_use]
    pub fn interface(mut self, internal_name: impl Into<String>) -> Self {
        self.interfaces.push(internal_name.into());
        self
    }

    #[must_use]
    pub fn source_file(mut self, name: impl Into<String>) -> Self {
        self.source_file = Some(name.into());
        self
    }

    #[must_use]
    pub fn signature(mut self, s: impl Into<String>) -> Self {
        self.signature = Some(s.into());
        self
    }

    #[must_use]
    pub fn nest_host(mut self, internal_name: impl Into<String>) -> Self {
        self.nest_host = Some(internal_name.into());
        self
    }

    #[must_use]
    pub fn nest_member(mut self, internal_name: impl Into<String>) -> Self {
        self.nest_members.push(internal_name.into());
        self
    }

    #[must_use]
    pub fn permitted_subclass(mut self, internal_name: impl Into<String>) -> Self {
        self.permitted_subclasses.push(internal_name.into());
        self
    }

    #[must_use]
    pub fn field(mut self, f: FieldBuilder) -> Self {
        self.fields.push(f);
        self
    }

    #[must_use]
    pub fn method(mut self, m: MethodBuilder) -> Self {
        self.methods.push(m);
        self
    }

    #[must_use]
    pub fn attribute(mut self, attr: Attribute) -> Self {
        self.extra_attributes.push(attr);
        self
    }

    /// Finalise into a structured [`ClassFile`].
    pub fn build_class_file(self) -> Result<ClassFile> {
        let mut pool = ConstantPool::new();
        let this_class = pool.intern_class(&self.this_name)?;
        let super_class = match self.super_name {
            Some(name) => pool.intern_class(&name)?,
            None => 0,
        };

        let mut interface_indices = Vec::with_capacity(self.interfaces.len());
        for i in &self.interfaces {
            interface_indices.push(pool.intern_class(i)?);
        }

        let mut fields = Vec::with_capacity(self.fields.len());
        for f in self.fields {
            fields.push(f.into_field(&mut pool)?);
        }

        let mut methods = Vec::with_capacity(self.methods.len());
        for m in self.methods {
            methods.push(m.into_method(&mut pool)?);
        }

        let mut attributes: Vec<Attribute> = Vec::new();
        if let Some(src) = self.source_file.as_deref() {
            let sourcefile_index = pool.intern_utf8(src)?;
            push_attr(
                &mut pool,
                &mut attributes,
                Attribute::SourceFile { sourcefile_index },
            )?;
        }
        if let Some(sig) = self.signature.as_deref() {
            let signature_index = pool.intern_utf8(sig)?;
            push_attr(
                &mut pool,
                &mut attributes,
                Attribute::Signature { signature_index },
            )?;
        }
        if let Some(host) = self.nest_host.as_deref() {
            let host_class_index = pool.intern_class(host)?;
            push_attr(
                &mut pool,
                &mut attributes,
                Attribute::NestHost { host_class_index },
            )?;
        }
        if !self.nest_members.is_empty() {
            let classes = intern_class_list(&mut pool, &self.nest_members)?;
            push_attr(
                &mut pool,
                &mut attributes,
                Attribute::NestMembers { classes },
            )?;
        }
        if !self.permitted_subclasses.is_empty() {
            let classes = intern_class_list(&mut pool, &self.permitted_subclasses)?;
            push_attr(
                &mut pool,
                &mut attributes,
                Attribute::PermittedSubclasses { classes },
            )?;
        }
        for a in self.extra_attributes {
            push_attr(&mut pool, &mut attributes, a)?;
        }

        Ok(ClassFile {
            version: self.version,
            constant_pool: pool,
            access_flags: ClassAccess::from(self.access_flags),
            this_class,
            super_class,
            interfaces: interface_indices,
            fields,
            methods,
            attributes,
        })
    }

    /// Build and encode to bytes in one step.
    pub fn build(self) -> Result<Vec<u8>> {
        encode(&self.build_class_file()?)
    }
}

fn intern_class_list(pool: &mut ConstantPool, names: &[String]) -> Result<Vec<u16>> {
    let mut out = Vec::with_capacity(names.len());
    for n in names {
        out.push(pool.intern_class(n)?);
    }
    Ok(out)
}
