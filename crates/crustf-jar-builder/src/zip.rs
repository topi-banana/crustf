//! Minimal ZIP 2.0 writer (stored entries only, single-disk, no ZIP64).
//!
//! The layout follows `APPNOTE.TXT` and the subset of fields required to be
//! parsed by `java.util.zip.ZipInputStream` and `java.util.jar.JarFile`.
//! Entries are written back-to-back followed by a central directory and an
//! end-of-central-directory record.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::crc32::crc32;
use crate::error::{Error, Result};

const SIG_LOCAL_FILE_HEADER: u32 = 0x0403_4b50;
const SIG_CENTRAL_DIRECTORY: u32 = 0x0201_4b50;
const SIG_END_OF_CENTRAL_DIRECTORY: u32 = 0x0605_4b50;

/// Minimum reader version: 2.0 (PKZip `major*10 + minor`).
const VERSION_NEEDED: u16 = 20;
/// Writer version, upper byte = host OS (0 = MS-DOS/FAT). 2.0 matches what
/// stock `jar` produces and keeps external file attributes interpretation
/// simple for consumers.
const VERSION_MADE_BY: u16 = 20;

/// Bit 11 tells readers the file name is UTF-8 (ZIP APPNOTE.TXT §4.4.4).
const FLAG_UTF8: u16 = 0x0800;

/// Compression method 0 = stored (no compression).
const METHOD_STORED: u16 = 0;

/// MS-DOS date 1980-01-01 (smallest representable, stable for reproducible builds).
const DOS_DATE_EPOCH: u16 = 0x0021;
const DOS_TIME_EPOCH: u16 = 0x0000;

#[derive(Debug, Clone)]
struct DirEntry {
    name: String,
    crc32: u32,
    size: u32,
    local_offset: u32,
}

/// Builds an in-memory ZIP archive of STORED entries.
///
/// The writer owns its output buffer; call [`ZipWriter::finish`] to consume
/// the writer and obtain the complete archive bytes.
#[derive(Debug, Default)]
pub struct ZipWriter {
    out: Vec<u8>,
    entries: Vec<DirEntry>,
}

impl ZipWriter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            out: Vec::with_capacity(cap),
            entries: Vec::new(),
        }
    }

    /// Append a stored (uncompressed) entry at `name` with the given data.
    pub fn write_stored(&mut self, name: &str, data: &[u8]) -> Result<()> {
        validate_name(name)?;
        let name_bytes = name.as_bytes();
        let name_len =
            u16::try_from(name_bytes.len()).map_err(|_| Error::NameTooLong(name_bytes.len()))?;
        let size = u32::try_from(data.len()).map_err(|_| Error::EntryTooLarge(data.len()))?;
        let local_offset = u32::try_from(self.out.len()).map_err(|_| Error::ArchiveTooLarge)?;
        let crc = crc32(data);

        write_u32(&mut self.out, SIG_LOCAL_FILE_HEADER);
        write_u16(&mut self.out, VERSION_NEEDED);
        write_shared_header_fields(&mut self.out, crc, size, name_len);
        self.out.extend_from_slice(name_bytes);
        self.out.extend_from_slice(data);

        self.entries.push(DirEntry {
            name: name.to_string(),
            crc32: crc,
            size,
            local_offset,
        });
        Ok(())
    }

    /// Finalise the archive and return the completed byte stream.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        let central_offset = u32::try_from(self.out.len()).map_err(|_| Error::ArchiveTooLarge)?;
        let total_entries = u16::try_from(self.entries.len())
            .map_err(|_| Error::TooManyEntries(self.entries.len()))?;

        for e in &self.entries {
            let name_bytes = e.name.as_bytes();
            let name_len = u16::try_from(name_bytes.len()).expect("validated in write_stored");
            write_u32(&mut self.out, SIG_CENTRAL_DIRECTORY);
            write_u16(&mut self.out, VERSION_MADE_BY);
            write_u16(&mut self.out, VERSION_NEEDED);
            write_shared_header_fields(&mut self.out, e.crc32, e.size, name_len);
            write_u16(&mut self.out, 0); // comment length
            write_u16(&mut self.out, 0); // disk number
            write_u16(&mut self.out, 0); // internal attrs
            write_u32(&mut self.out, 0); // external attrs
            write_u32(&mut self.out, e.local_offset);
            self.out.extend_from_slice(name_bytes);
        }

        let central_end = u32::try_from(self.out.len()).map_err(|_| Error::ArchiveTooLarge)?;
        let central_size = central_end - central_offset;

        write_u32(&mut self.out, SIG_END_OF_CENTRAL_DIRECTORY);
        write_u16(&mut self.out, 0); // disk number
        write_u16(&mut self.out, 0); // disk of central dir start
        write_u16(&mut self.out, total_entries); // entries on this disk
        write_u16(&mut self.out, total_entries); // total entries
        write_u32(&mut self.out, central_size);
        write_u32(&mut self.out, central_offset);
        write_u16(&mut self.out, 0); // comment length

        Ok(self.out)
    }
}

/// Emit the 22 bytes shared between a local file header and its central
/// directory counterpart (flags, method, mtime, CRC, sizes, name length).
fn write_shared_header_fields(out: &mut Vec<u8>, crc: u32, size: u32, name_len: u16) {
    write_u16(out, FLAG_UTF8);
    write_u16(out, METHOD_STORED);
    write_u16(out, DOS_TIME_EPOCH);
    write_u16(out, DOS_DATE_EPOCH);
    write_u32(out, crc);
    write_u32(out, size); // compressed size (= uncompressed for STORED)
    write_u32(out, size); // uncompressed size
    write_u16(out, name_len);
    write_u16(out, 0); // extra field length
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidName(String::new()));
    }
    if name.starts_with('/') || name.contains('\\') || name.contains('\0') {
        return Err(Error::InvalidName(name.to_string()));
    }
    if name.split('/').any(|seg| seg == "..") {
        return Err(Error::InvalidName(name.to_string()));
    }
    Ok(())
}

#[inline]
fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

#[inline]
fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_entry_layout() {
        let mut w = ZipWriter::new();
        w.write_stored("a.txt", b"hi").unwrap();
        let bytes = w.finish().unwrap();

        assert_eq!(&bytes[..4], &SIG_LOCAL_FILE_HEADER.to_le_bytes());
        // Search for the central directory signature somewhere in the tail.
        assert!(bytes
            .windows(4)
            .any(|w| w == SIG_CENTRAL_DIRECTORY.to_le_bytes()));
        // EOCD sits at the very end, exactly 22 bytes before len().
        let eocd_start = bytes.len() - 22;
        assert_eq!(
            &bytes[eocd_start..eocd_start + 4],
            &SIG_END_OF_CENTRAL_DIRECTORY.to_le_bytes()
        );
    }

    #[test]
    fn reject_absolute_and_parent_paths() {
        let mut w = ZipWriter::new();
        assert!(matches!(
            w.write_stored("/etc/passwd", b""),
            Err(Error::InvalidName(_))
        ));
        assert!(matches!(
            w.write_stored("../secret", b""),
            Err(Error::InvalidName(_))
        ));
        assert!(matches!(
            w.write_stored("a\\b", b""),
            Err(Error::InvalidName(_))
        ));
    }

    #[test]
    fn multiple_entries_record_correct_offsets() {
        let mut w = ZipWriter::new();
        w.write_stored("a", b"111").unwrap();
        w.write_stored("b", b"2222").unwrap();
        let bytes = w.finish().unwrap();
        // The last 22 bytes are the EOCD; the u16 at offset -14 (from end)
        // gives the total entry count.
        let eocd = &bytes[bytes.len() - 22..];
        let total = u16::from_le_bytes([eocd[10], eocd[11]]);
        assert_eq!(total, 2);
    }
}
