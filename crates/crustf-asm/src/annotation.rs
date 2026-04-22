//! Runtime-visible and -invisible annotation builders.
//!
//! [`Annotation`] mirrors JVMS §4.7.16 but carries string-friendly
//! descriptors; the spec-level indexed form is produced during
//! [`crate::ClassFileBuilder::build_class_file`] when the pool is
//! available. Annotations partition into `RuntimeVisibleAnnotations` and
//! `RuntimeInvisibleAnnotations` at that point based on each instance's
//! [`Annotation::visible`] flag.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crustf_spec::attribute::{
    Annotation as SpecAnnotation, Attribute, ElementValue as SpecElementValue,
};
use crustf_spec::constant::{Constant, ConstantPool};
use crustf_spec::error::Result;

/// One annotation instance, attachable to a class, field, or method.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub(crate) type_descriptor: String,
    pub(crate) visible: bool,
    pub(crate) elements: Vec<(String, ElementValue)>,
}

impl Annotation {
    /// Build a `@Retention(RUNTIME)` style annotation — preserved in the
    /// `RuntimeVisibleAnnotations` attribute and reflected by the JVM.
    #[must_use]
    pub fn visible(type_descriptor: impl Into<String>) -> Self {
        Self {
            type_descriptor: type_descriptor.into(),
            visible: true,
            elements: Vec::new(),
        }
    }

    /// Build a `@Retention(CLASS)` style annotation — stored in
    /// `RuntimeInvisibleAnnotations`; readable by tools that parse class
    /// files but invisible to reflection.
    #[must_use]
    pub fn invisible(type_descriptor: impl Into<String>) -> Self {
        Self {
            type_descriptor: type_descriptor.into(),
            visible: false,
            elements: Vec::new(),
        }
    }

    /// Append a `name = value` pair to the annotation body.
    #[must_use]
    pub fn element(mut self, name: impl Into<String>, value: ElementValue) -> Self {
        self.elements.push((name.into(), value));
        self
    }

    pub(crate) fn resolve(&self, pool: &mut ConstantPool) -> Result<SpecAnnotation> {
        let type_index = pool.intern_utf8(&self.type_descriptor)?;
        let mut pairs = Vec::with_capacity(self.elements.len());
        for (name, value) in &self.elements {
            let name_index = pool.intern_utf8(name)?;
            pairs.push((name_index, value.resolve(pool)?));
        }
        Ok(SpecAnnotation {
            type_index,
            element_value_pairs: pairs,
        })
    }
}

/// Annotation element value, JVMS §4.7.16.1.
///
/// Numeric and string payloads are supplied as Rust values; conversion to
/// the interned constant pool entry happens at build time. Nested
/// annotations and arrays are modelled directly.
#[derive(Debug, Clone, PartialEq)]
pub enum ElementValue {
    Byte(i8),
    Char(u16),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Boolean(bool),
    String(String),
    /// Field descriptor of the class literal (e.g. `Ljava/lang/String;`,
    /// `[I`, `V`). Stored as a `CONSTANT_Utf8` rather than `CONSTANT_Class`
    /// per JVMS §4.7.16.1.
    Class(String),
    /// `@Enum(type = ..., name = ...)` — `type_descriptor` is the field
    /// descriptor of the enum class (e.g. `Ljava/lang/annotation/RetentionPolicy;`).
    Enum {
        type_descriptor: String,
        constant_name: String,
    },
    Annotation(Box<Annotation>),
    Array(Vec<ElementValue>),
}

impl ElementValue {
    /// Sugar for `ElementValue::Annotation(Box::new(inner))`.
    #[must_use]
    pub fn from_annotation(inner: Annotation) -> Self {
        Self::Annotation(Box::new(inner))
    }

    pub(crate) fn resolve(&self, pool: &mut ConstantPool) -> Result<SpecElementValue> {
        Ok(match self {
            Self::Byte(v) => SpecElementValue::Byte(pool.intern(Constant::Integer(i32::from(*v)))?),
            Self::Char(v) => SpecElementValue::Char(pool.intern(Constant::Integer(i32::from(*v)))?),
            Self::Short(v) => {
                SpecElementValue::Short(pool.intern(Constant::Integer(i32::from(*v)))?)
            }
            Self::Int(v) => SpecElementValue::Int(pool.intern(Constant::Integer(*v))?),
            Self::Long(v) => SpecElementValue::Long(pool.intern(Constant::Long(*v))?),
            Self::Float(v) => SpecElementValue::Float(pool.intern(Constant::Float(v.to_bits()))?),
            Self::Double(v) => {
                SpecElementValue::Double(pool.intern(Constant::Double(v.to_bits()))?)
            }
            Self::Boolean(v) => {
                SpecElementValue::Boolean(pool.intern(Constant::Integer(i32::from(*v)))?)
            }
            Self::String(s) => SpecElementValue::String(pool.intern_utf8(s)?),
            Self::Class(desc) => SpecElementValue::Class(pool.intern_utf8(desc)?),
            Self::Enum {
                type_descriptor,
                constant_name,
            } => SpecElementValue::Enum {
                type_name_index: pool.intern_utf8(type_descriptor)?,
                const_name_index: pool.intern_utf8(constant_name)?,
            },
            Self::Annotation(inner) => SpecElementValue::Annotation(Box::new(inner.resolve(pool)?)),
            Self::Array(items) => {
                let mut resolved = Vec::with_capacity(items.len());
                for i in items {
                    resolved.push(i.resolve(pool)?);
                }
                SpecElementValue::Array(resolved)
            }
        })
    }
}

impl From<Annotation> for ElementValue {
    fn from(a: Annotation) -> Self {
        Self::from_annotation(a)
    }
}

/// Resolve `annotations` against `pool`, split by visibility, and push as
/// `RuntimeVisibleAnnotations` / `RuntimeInvisibleAnnotations` attributes.
pub(crate) fn push_annotation_attrs(
    pool: &mut ConstantPool,
    attrs: &mut Vec<Attribute>,
    annotations: Vec<Annotation>,
) -> Result<()> {
    if annotations.is_empty() {
        return Ok(());
    }
    let (visible, invisible): (Vec<_>, Vec<_>) = annotations.into_iter().partition(|a| a.visible);
    if !visible.is_empty() {
        let mut resolved = Vec::with_capacity(visible.len());
        for a in &visible {
            resolved.push(a.resolve(pool)?);
        }
        crate::util::push_attr(pool, attrs, Attribute::RuntimeVisibleAnnotations(resolved))?;
    }
    if !invisible.is_empty() {
        let mut resolved = Vec::with_capacity(invisible.len());
        for a in &invisible {
            resolved.push(a.resolve(pool)?);
        }
        crate::util::push_attr(
            pool,
            attrs,
            Attribute::RuntimeInvisibleAnnotations(resolved),
        )?;
    }
    Ok(())
}
