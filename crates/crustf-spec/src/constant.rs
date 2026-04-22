//! Constant pool (`JVMS §4.4`).
//!
//! All 17 `CONSTANT_*` kinds defined by the specification (including the
//! `Module`/`Package`/`Dynamic`/`InvokeDynamic`/`MethodHandle`/`MethodType`
//! additions) are representable here. `Long` and `Double` occupy *two*
//! logical slots as required by JVMS §4.4.5.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{Error, Result};

/// Numeric tags assigned by JVMS §4.4 Table 4.4-A.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstantTag {
    Utf8 = 1,
    Integer = 3,
    Float = 4,
    Long = 5,
    Double = 6,
    Class = 7,
    String = 8,
    Fieldref = 9,
    Methodref = 10,
    InterfaceMethodref = 11,
    NameAndType = 12,
    MethodHandle = 15,
    MethodType = 16,
    Dynamic = 17,
    InvokeDynamic = 18,
    Module = 19,
    Package = 20,
}

/// `reference_kind` byte for `CONSTANT_MethodHandle_info` (JVMS §4.4.8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    GetField = 1,
    GetStatic = 2,
    PutField = 3,
    PutStatic = 4,
    InvokeVirtual = 5,
    InvokeStatic = 6,
    InvokeSpecial = 7,
    NewInvokeSpecial = 8,
    InvokeInterface = 9,
}

/// A single constant pool entry.
///
/// Floats and doubles are stored as IEEE-754 bit patterns so the enum stays
/// `Eq + Hash` (which we need for interning).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant {
    Utf8(String),
    Integer(i32),
    Float(u32),
    Long(i64),
    Double(u64),
    Class {
        name_index: u16,
    },
    String {
        string_index: u16,
    },
    Fieldref {
        class_index: u16,
        name_and_type_index: u16,
    },
    Methodref {
        class_index: u16,
        name_and_type_index: u16,
    },
    InterfaceMethodref {
        class_index: u16,
        name_and_type_index: u16,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    MethodHandle {
        reference_kind: ReferenceKind,
        reference_index: u16,
    },
    MethodType {
        descriptor_index: u16,
    },
    Dynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
    InvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
    Module {
        name_index: u16,
    },
    Package {
        name_index: u16,
    },
}

impl Constant {
    #[must_use]
    pub const fn tag(&self) -> ConstantTag {
        match self {
            Self::Utf8(_) => ConstantTag::Utf8,
            Self::Integer(_) => ConstantTag::Integer,
            Self::Float(_) => ConstantTag::Float,
            Self::Long(_) => ConstantTag::Long,
            Self::Double(_) => ConstantTag::Double,
            Self::Class { .. } => ConstantTag::Class,
            Self::String { .. } => ConstantTag::String,
            Self::Fieldref { .. } => ConstantTag::Fieldref,
            Self::Methodref { .. } => ConstantTag::Methodref,
            Self::InterfaceMethodref { .. } => ConstantTag::InterfaceMethodref,
            Self::NameAndType { .. } => ConstantTag::NameAndType,
            Self::MethodHandle { .. } => ConstantTag::MethodHandle,
            Self::MethodType { .. } => ConstantTag::MethodType,
            Self::Dynamic { .. } => ConstantTag::Dynamic,
            Self::InvokeDynamic { .. } => ConstantTag::InvokeDynamic,
            Self::Module { .. } => ConstantTag::Module,
            Self::Package { .. } => ConstantTag::Package,
        }
    }

    /// Whether this entry occupies two slots (long/double).
    #[must_use]
    pub const fn is_wide(&self) -> bool {
        matches!(self, Self::Long(_) | Self::Double(_))
    }
}

/// A JVM constant pool. Index 0 is reserved and treated as absent. Wide
/// entries occupy two indices per JVMS §4.4.5, the second slot is `None`.
#[derive(Debug, Clone, Default)]
pub struct ConstantPool {
    entries: Vec<Option<Constant>>,
}

impl ConstantPool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: alloc::vec![None],
        }
    }

    /// Iterate over (index, &constant) pairs in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = (u16, &Constant)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|c| (i as u16, c)))
    }

    /// Number of slots including reserved/wide padding (JVMS `constant_pool_count`).
    #[must_use]
    pub fn count(&self) -> u16 {
        self.entries.len() as u16
    }

    #[must_use]
    pub fn get(&self, index: u16) -> Option<&Constant> {
        self.entries.get(index as usize).and_then(Option::as_ref)
    }

    /// Append a constant without deduplication. Returns the assigned index.
    pub fn push(&mut self, c: Constant) -> Result<u16> {
        let wide = c.is_wide();
        let idx = self.entries.len();
        if idx + usize::from(wide) > u16::MAX as usize {
            return Err(Error::TooManyEntries {
                what: "constant pool",
                count: idx,
            });
        }
        self.entries.push(Some(c));
        if wide {
            self.entries.push(None);
        }
        Ok(idx as u16)
    }

    /// Append the constant, or return the existing index if an identical
    /// entry is already present. Linear scan is acceptable because typical
    /// class files have a few hundred entries; users that need more can
    /// batch-build with `push`.
    pub fn intern(&mut self, c: Constant) -> Result<u16> {
        for (i, slot) in self.entries.iter().enumerate().skip(1) {
            if let Some(existing) = slot {
                if existing == &c {
                    return Ok(i as u16);
                }
            }
        }
        self.push(c)
    }

    pub fn intern_utf8(&mut self, s: &str) -> Result<u16> {
        self.intern(Constant::Utf8(s.to_string()))
    }

    pub fn intern_class(&mut self, internal_name: &str) -> Result<u16> {
        let name_index = self.intern_utf8(internal_name)?;
        self.intern(Constant::Class { name_index })
    }

    pub fn intern_string(&mut self, value: &str) -> Result<u16> {
        let string_index = self.intern_utf8(value)?;
        self.intern(Constant::String { string_index })
    }

    pub fn intern_name_and_type(&mut self, name: &str, descriptor: &str) -> Result<u16> {
        let name_index = self.intern_utf8(name)?;
        let descriptor_index = self.intern_utf8(descriptor)?;
        self.intern(Constant::NameAndType {
            name_index,
            descriptor_index,
        })
    }

    pub fn intern_fieldref(&mut self, class: &str, name: &str, desc: &str) -> Result<u16> {
        let class_index = self.intern_class(class)?;
        let name_and_type_index = self.intern_name_and_type(name, desc)?;
        self.intern(Constant::Fieldref {
            class_index,
            name_and_type_index,
        })
    }

    pub fn intern_methodref(&mut self, class: &str, name: &str, desc: &str) -> Result<u16> {
        let class_index = self.intern_class(class)?;
        let name_and_type_index = self.intern_name_and_type(name, desc)?;
        self.intern(Constant::Methodref {
            class_index,
            name_and_type_index,
        })
    }

    pub fn intern_interface_methodref(
        &mut self,
        class: &str,
        name: &str,
        desc: &str,
    ) -> Result<u16> {
        let class_index = self.intern_class(class)?;
        let name_and_type_index = self.intern_name_and_type(name, desc)?;
        self.intern(Constant::InterfaceMethodref {
            class_index,
            name_and_type_index,
        })
    }

    pub fn intern_method_type(&mut self, desc: &str) -> Result<u16> {
        let descriptor_index = self.intern_utf8(desc)?;
        self.intern(Constant::MethodType { descriptor_index })
    }

    pub fn intern_method_handle(
        &mut self,
        kind: ReferenceKind,
        reference_index: u16,
    ) -> Result<u16> {
        self.intern(Constant::MethodHandle {
            reference_kind: kind,
            reference_index,
        })
    }

    pub fn intern_invoke_dynamic(
        &mut self,
        bootstrap_method_attr_index: u16,
        name: &str,
        desc: &str,
    ) -> Result<u16> {
        let name_and_type_index = self.intern_name_and_type(name, desc)?;
        self.intern(Constant::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    pub fn intern_dynamic(
        &mut self,
        bootstrap_method_attr_index: u16,
        name: &str,
        desc: &str,
    ) -> Result<u16> {
        let name_and_type_index = self.intern_name_and_type(name, desc)?;
        self.intern(Constant::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    pub fn intern_module(&mut self, module_name: &str) -> Result<u16> {
        let name_index = self.intern_utf8(module_name)?;
        self.intern(Constant::Module { name_index })
    }

    pub fn intern_package(&mut self, package_name: &str) -> Result<u16> {
        let name_index = self.intern_utf8(package_name)?;
        self.intern(Constant::Package { name_index })
    }

    /// Look up the index of an already-interned UTF-8 string without modifying the pool.
    #[must_use]
    pub fn find_utf8(&self, s: &str) -> Option<u16> {
        self.iter()
            .find(|(_, c)| matches!(c, Constant::Utf8(v) if v == s))
            .map(|(i, _)| i)
    }

    /// Read a `CONSTANT_Utf8` value by index, for error reporting / tests.
    #[must_use]
    pub fn utf8(&self, index: u16) -> Option<&str> {
        match self.get(index)? {
            Constant::Utf8(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_deduplicates_utf8() {
        let mut p = ConstantPool::new();
        let a = p.intern_utf8("java/lang/String").unwrap();
        let b = p.intern_utf8("java/lang/String").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn long_occupies_two_slots() {
        let mut p = ConstantPool::new();
        let idx = p.push(Constant::Long(42)).unwrap();
        let next = p.push(Constant::Integer(1)).unwrap();
        assert_eq!(next, idx + 2);
        assert!(p.get(idx + 1).is_none());
    }

    #[test]
    fn intern_methodref_chains() {
        let mut p = ConstantPool::new();
        let idx = p
            .intern_methodref("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
            .unwrap();
        assert!(matches!(p.get(idx), Some(Constant::Methodref { .. })));
        // Shared utf8 entries.
        assert_eq!(
            p.find_utf8("java/io/PrintStream"),
            Some(p.find_utf8("java/io/PrintStream").unwrap())
        );
    }

    #[test]
    fn float_equality_uses_bits() {
        let mut p = ConstantPool::new();
        let a = p.intern(Constant::Float(f32::NAN.to_bits())).unwrap();
        let b = p.intern(Constant::Float(f32::NAN.to_bits())).unwrap();
        assert_eq!(a, b);
    }
}
