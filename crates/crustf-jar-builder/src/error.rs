//! Error type.

use alloc::string::String;
use core::fmt;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// ZIP file names are a `u16` length field; anything longer cannot be
    /// stored in a local-file-header.
    NameTooLong(usize),
    /// The stored uncompressed size exceeds the 32-bit limit. ZIP64 is not
    /// implemented.
    EntryTooLarge(usize),
    /// The archive would contain more entries than the EOCD `u16` field can
    /// address.
    TooManyEntries(usize),
    /// A file name contained bytes that are forbidden in ZIP entry paths
    /// (NUL, backslash, leading slash, or `..` path segments).
    InvalidName(String),
    /// Combined central directory + local headers would exceed `u32`.
    ArchiveTooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameTooLong(n) => write!(f, "ZIP entry name length {n} exceeds u16"),
            Self::EntryTooLarge(n) => {
                write!(f, "ZIP entry size {n} exceeds u32 (ZIP64 unsupported)")
            }
            Self::TooManyEntries(n) => write!(f, "archive contains {n} entries, > u16"),
            Self::InvalidName(s) => write!(f, "invalid ZIP entry name {s:?}"),
            Self::ArchiveTooLarge => f.write_str("archive offsets exceed u32 (ZIP64 unsupported)"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
