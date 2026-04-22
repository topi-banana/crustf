//! `META-INF/MANIFEST.MF` builder.
//!
//! Manifests in the JAR specification look like RFC 822 headers: newline
//! terminated `Name: Value` pairs, with lines longer than 72 bytes folded
//! onto continuation lines prefixed with a single space. The main
//! attributes section always starts with `Manifest-Version: 1.0` and ends
//! with a blank line.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

const MAX_LINE_BYTES: usize = 72;

/// A minimal JAR manifest: main attributes only, no per-entry sections.
#[derive(Debug, Clone)]
pub struct Manifest {
    entries: Vec<(String, String)>,
}

impl Manifest {
    /// Create a manifest seeded with `Manifest-Version: 1.0`, which the JAR
    /// spec requires as the first main attribute.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: alloc::vec![("Manifest-Version".to_string(), "1.0".to_string())],
        }
    }

    /// In-place counterpart to [`Manifest::attribute`].
    pub fn set_attribute(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();
        for (n, v) in &mut self.entries {
            if n.eq_ignore_ascii_case(&name) {
                *v = value;
                return;
            }
        }
        self.entries.push((name, value));
    }

    /// In-place counterpart to [`Manifest::main_class`].
    pub fn set_main_class(&mut self, internal_or_source_name: impl Into<String>) {
        let name: String = internal_or_source_name.into();
        self.set_attribute("Main-Class", name.replace('/', "."));
    }

    /// Set or append a main attribute. If `name` is already present its
    /// value is overwritten so callers can opt in to semantic helpers like
    /// [`Manifest::main_class`] repeatedly without accumulating duplicates.
    #[must_use]
    pub fn attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.set_attribute(name, value);
        self
    }

    /// Convenience wrapper for `Main-Class`. Slashes in the name are
    /// rewritten to dots since manifest values use Java-source syntax
    /// (`com.example.Hello`) while class files store the internal form.
    #[must_use]
    pub fn main_class(mut self, internal_or_source_name: impl Into<String>) -> Self {
        self.set_main_class(internal_or_source_name);
        self
    }

    /// Space-separated `Class-Path` attribute.
    #[must_use]
    pub fn class_path<I, S>(self, entries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut value = String::new();
        for e in entries {
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(e.as_ref());
        }
        self.attribute("Class-Path", value)
    }

    /// Serialise to bytes suitable for the `META-INF/MANIFEST.MF` archive
    /// entry.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        // `+ 4` covers `": "` and the trailing `\r\n`; `+ 2` adds the blank
        // terminating line. The common (unfolded) case allocates exactly once.
        let cap: usize = self
            .entries
            .iter()
            .map(|(k, v)| k.len() + v.len() + 4)
            .sum::<usize>()
            + 2;
        let mut out = Vec::with_capacity(cap);
        for (name, value) in &self.entries {
            let mut line = String::with_capacity(name.len() + value.len() + 2);
            line.push_str(name);
            line.push_str(": ");
            line.push_str(value);
            write_folded(&mut out, line.as_bytes());
        }
        out.extend_from_slice(b"\r\n");
        out
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

fn write_folded(out: &mut Vec<u8>, line: &[u8]) {
    let mut i = 0;
    let mut first = true;
    while i < line.len() {
        let budget = if first {
            MAX_LINE_BYTES
        } else {
            MAX_LINE_BYTES - 1
        };
        let end = (i + budget).min(line.len());
        if !first {
            out.push(b' ');
        }
        out.extend_from_slice(&line[i..end]);
        out.extend_from_slice(b"\r\n");
        i = end;
        first = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_first() {
        let bytes = Manifest::new().to_bytes();
        assert!(bytes.starts_with(b"Manifest-Version: 1.0\r\n"));
    }

    #[test]
    fn ends_with_blank_line() {
        let bytes = Manifest::new().main_class("HelloWorld").to_bytes();
        assert!(bytes.ends_with(b"\r\n\r\n"));
    }

    #[test]
    fn main_class_converts_slashes_to_dots() {
        let bytes = Manifest::new().main_class("com/example/Hello").to_bytes();
        let s = core::str::from_utf8(&bytes).unwrap();
        assert!(s.contains("Main-Class: com.example.Hello"), "{s}");
    }

    #[test]
    fn repeated_attribute_overwrites() {
        let m = Manifest::new().main_class("A").main_class("B");
        let s = String::from_utf8(m.to_bytes()).unwrap();
        assert!(s.contains("Main-Class: B"));
        assert!(!s.contains("Main-Class: A"));
    }

    #[test]
    fn long_lines_are_folded_at_72_bytes() {
        let long_value: String = "x".repeat(200);
        let bytes = Manifest::new().attribute("X-Long", long_value).to_bytes();
        for line in bytes.split(|b| *b == b'\n') {
            // strip trailing CR
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            assert!(
                line.len() <= MAX_LINE_BYTES,
                "line over 72 bytes: {:?}",
                line
            );
        }
    }

    #[test]
    fn class_path_spaces_entries() {
        let bytes = Manifest::new()
            .class_path(["a.jar", "b.jar", "c.jar"])
            .to_bytes();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("Class-Path: a.jar b.jar c.jar"));
    }
}
