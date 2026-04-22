//! Type descriptors (JVMS §4.3).
//!
//! The grammar is intentionally re-implemented in Rust rather than passed
//! through to the pool as opaque strings so that higher level crates can
//! validate method shapes before emitting bytecode.

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Write};

use crate::error::{Error, Result};

/// Primitive base types (JVMS §4.3.2 `BaseType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
}

impl BaseType {
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            Self::Byte => 'B',
            Self::Char => 'C',
            Self::Double => 'D',
            Self::Float => 'F',
            Self::Int => 'I',
            Self::Long => 'J',
            Self::Short => 'S',
            Self::Boolean => 'Z',
        }
    }

    #[must_use]
    pub const fn slots(self) -> u16 {
        matches!(self, Self::Long | Self::Double) as u16 + 1
    }
}

impl fmt::Display for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char(self.letter())
    }
}

/// Field descriptor (JVMS §4.3.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldDescriptor {
    Base(BaseType),
    /// `Lclass_name;` — `class_name` is in internal form (slashes).
    Object(String),
    Array(Box<FieldDescriptor>),
}

impl FieldDescriptor {
    pub fn parse(s: &str) -> Result<Self> {
        let (desc, rest) = parse_field(s)?;
        if !rest.is_empty() {
            return Err(Error::Encoding(alloc::format!(
                "trailing bytes in field descriptor: {rest:?}"
            )));
        }
        Ok(desc)
    }

    #[must_use]
    pub fn slots(&self) -> u16 {
        match self {
            Self::Base(b) => b.slots(),
            _ => 1,
        }
    }
}

impl fmt::Display for FieldDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base(b) => b.fmt(f),
            Self::Object(name) => write!(f, "L{name};"),
            Self::Array(inner) => write!(f, "[{inner}"),
        }
    }
}

/// Method descriptor (JVMS §4.3.3). `None` return means `void`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodDescriptor {
    pub params: Vec<FieldDescriptor>,
    pub ret: Option<FieldDescriptor>,
}

impl MethodDescriptor {
    pub fn parse(s: &str) -> Result<Self> {
        let rest = s
            .strip_prefix('(')
            .ok_or_else(|| Error::Encoding("method descriptor must start with '('".to_string()))?;
        let mut params = Vec::new();
        let mut cursor = rest;
        loop {
            if let Some(after) = cursor.strip_prefix(')') {
                cursor = after;
                break;
            }
            let (p, c) = parse_field(cursor)?;
            params.push(p);
            cursor = c;
        }
        let ret = if cursor == "V" {
            None
        } else {
            let (r, rest) = parse_field(cursor)?;
            if !rest.is_empty() {
                return Err(Error::Encoding(alloc::format!("trailing: {rest:?}")));
            }
            Some(r)
        };
        Ok(Self { params, ret })
    }

    /// Total local slots required for the parameter list (`long`/`double` count twice).
    #[must_use]
    pub fn param_slots(&self) -> u16 {
        self.params.iter().map(FieldDescriptor::slots).sum()
    }
}

impl fmt::Display for MethodDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(")?;
        for p in &self.params {
            p.fmt(f)?;
        }
        f.write_str(")")?;
        match &self.ret {
            Some(r) => r.fmt(f),
            None => f.write_str("V"),
        }
    }
}

fn parse_field(s: &str) -> Result<(FieldDescriptor, &str)> {
    let mut chars = s.char_indices();
    let (_, first) = chars
        .next()
        .ok_or_else(|| Error::Encoding("empty field descriptor".to_string()))?;
    match first {
        'B' => Ok((FieldDescriptor::Base(BaseType::Byte), &s[1..])),
        'C' => Ok((FieldDescriptor::Base(BaseType::Char), &s[1..])),
        'D' => Ok((FieldDescriptor::Base(BaseType::Double), &s[1..])),
        'F' => Ok((FieldDescriptor::Base(BaseType::Float), &s[1..])),
        'I' => Ok((FieldDescriptor::Base(BaseType::Int), &s[1..])),
        'J' => Ok((FieldDescriptor::Base(BaseType::Long), &s[1..])),
        'S' => Ok((FieldDescriptor::Base(BaseType::Short), &s[1..])),
        'Z' => Ok((FieldDescriptor::Base(BaseType::Boolean), &s[1..])),
        'L' => {
            let end = s[1..]
                .find(';')
                .ok_or_else(|| Error::Encoding("unterminated 'L' in descriptor".to_string()))?;
            Ok((
                FieldDescriptor::Object(s[1..1 + end].to_string()),
                &s[2 + end..],
            ))
        }
        '[' => {
            let (inner, rest) = parse_field(&s[1..])?;
            Ok((FieldDescriptor::Array(Box::new(inner)), rest))
        }
        other => Err(Error::Encoding(alloc::format!(
            "unknown descriptor token {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_roundtrip() {
        let d = FieldDescriptor::parse("I").unwrap();
        assert_eq!(d.to_string(), "I");
        assert_eq!(d, FieldDescriptor::Base(BaseType::Int));
    }

    #[test]
    fn object_roundtrip() {
        let d = FieldDescriptor::parse("Ljava/lang/String;").unwrap();
        assert_eq!(d.to_string(), "Ljava/lang/String;");
    }

    #[test]
    fn array_roundtrip() {
        let d = FieldDescriptor::parse("[[Ljava/lang/Object;").unwrap();
        assert_eq!(d.to_string(), "[[Ljava/lang/Object;");
    }

    #[test]
    fn method_void() {
        let m = MethodDescriptor::parse("([Ljava/lang/String;)V").unwrap();
        assert_eq!(m.params.len(), 1);
        assert!(m.ret.is_none());
        assert_eq!(m.to_string(), "([Ljava/lang/String;)V");
        assert_eq!(m.param_slots(), 1);
    }

    #[test]
    fn method_long_double_slots() {
        let m = MethodDescriptor::parse("(JDI)V").unwrap();
        assert_eq!(m.param_slots(), 2 + 2 + 1);
    }
}
