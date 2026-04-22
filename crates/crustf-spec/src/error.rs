//! Error types returned by the encoder and helpers.

use alloc::string::String;
use core::fmt;

pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Errors that may occur while encoding a class file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A `u2` sized container (constant pool, interfaces, methods, fields, ...) overflowed.
    TooManyEntries { what: &'static str, count: usize },
    /// Tried to look up a constant pool name that has not been interned.
    MissingUtf8(String),
    /// The code array exceeded the `u4` bound of the `Code` attribute (JVMS §4.7.3).
    CodeTooLarge(usize),
    /// A branch offset could not be represented in the required width.
    BranchOutOfRange { offset: i64, width_bits: u8 },
    /// A label was used but never resolved to an instruction position.
    UnresolvedLabel(usize),
    /// Modified UTF-8 byte sequence is malformed at `offset`.
    InvalidUtf8 { offset: usize, reason: &'static str },
    /// A generic message for violations picked up at encode time.
    Encoding(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyEntries { what, count } => {
                write!(f, "too many entries for {what}: {count} > 65535")
            }
            Self::MissingUtf8(s) => write!(f, "CONSTANT_Utf8 {s:?} is not in the pool"),
            Self::CodeTooLarge(n) => write!(f, "code array of {n} bytes exceeds u4 bounds"),
            Self::BranchOutOfRange { offset, width_bits } => {
                write!(
                    f,
                    "branch offset {offset} does not fit in {width_bits} bits"
                )
            }
            Self::UnresolvedLabel(id) => write!(f, "label #{id} was never placed"),
            Self::InvalidUtf8 { offset, reason } => {
                write!(f, "invalid modified UTF-8 at byte {offset}: {reason}")
            }
            Self::Encoding(msg) => f.write_str(msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
