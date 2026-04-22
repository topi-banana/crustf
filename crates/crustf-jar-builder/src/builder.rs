//! Fluent [`JarBuilder`].

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::Result;
use crate::manifest::Manifest;
use crate::zip::ZipWriter;

/// Canonical path used by the JAR specification for the manifest entry.
pub const MANIFEST_PATH: &str = "META-INF/MANIFEST.MF";

#[derive(Debug, Clone)]
struct FileEntry {
    name: String,
    data: Vec<u8>,
}

/// High-level builder for JAR archives.
///
/// Internally a [`Manifest`] plus a collection of file entries; `.build()`
/// writes the manifest first so `JarInputStream` can locate it without
/// scanning.
#[derive(Debug, Clone, Default)]
pub struct JarBuilder {
    manifest: Manifest,
    entries: Vec<FileEntry>,
}

impl JarBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the manifest in its entirety.
    #[must_use]
    pub fn manifest(mut self, manifest: Manifest) -> Self {
        self.manifest = manifest;
        self
    }

    /// Shorthand for `.manifest(Manifest::new().main_class(name))`.
    #[must_use]
    pub fn main_class(mut self, name: impl Into<String>) -> Self {
        self.manifest.set_main_class(name);
        self
    }

    /// Add a main-attribute to the manifest (overwrites previous value
    /// for the same attribute name).
    #[must_use]
    pub fn manifest_attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.manifest.set_attribute(name, value);
        self
    }

    /// Add one file to the archive.
    ///
    /// `name` is a forward-slash separated archive path (e.g.
    /// `"com/example/Hello.class"`). Names that would escape the archive
    /// root (`..`, absolute paths, backslashes) are rejected at `.build()`.
    #[must_use]
    pub fn file(mut self, name: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        self.entries.push(FileEntry {
            name: name.into(),
            data: data.into(),
        });
        self
    }

    /// Build the JAR into bytes.
    pub fn build(self) -> Result<Vec<u8>> {
        let manifest_bytes = self.manifest.to_bytes();
        let mut zip = ZipWriter::with_capacity(
            // rough estimate: manifest + 60 B overhead per entry + entry bytes
            manifest_bytes.len()
                + 64
                + self
                    .entries
                    .iter()
                    .map(|e| e.data.len() + 128)
                    .sum::<usize>(),
        );
        zip.write_stored(MANIFEST_PATH, &manifest_bytes)?;
        for e in self.entries {
            zip.write_stored(&e.name, &e.data)?;
        }
        zip.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_first_entry() {
        let jar = JarBuilder::new()
            .main_class("Hello")
            .file("Hello.class", b"CAFEBABE".to_vec())
            .build()
            .unwrap();
        // Local file header at offset 0 is followed by the manifest path.
        // The first 30 bytes are the header struct, then the filename.
        let name_start = 30;
        let name_end = name_start + MANIFEST_PATH.len();
        assert_eq!(&jar[name_start..name_end], MANIFEST_PATH.as_bytes());
    }

    #[test]
    fn main_class_attribute_is_serialised() {
        let jar = JarBuilder::new().main_class("com/foo/Hi").build().unwrap();
        assert!(jar.windows(24).any(|w| w == b"Main-Class: com.foo.Hi\r\n"));
    }
}
