//! Modified UTF-8 encoding used by `CONSTANT_Utf8_info` (JVMS §4.4.7).
//!
//! Java's flavor of UTF-8 differs from standard UTF-8 in two ways:
//!
//! * `U+0000` is written as the two byte sequence `0xC0 0x80` so that the byte
//!   `0x00` never appears in a `CONSTANT_Utf8` payload,
//! * codepoints above `U+FFFF` are encoded as a UTF-16 surrogate pair, each
//!   half as its own three byte group (six bytes total).

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::{Error, Result};

#[must_use]
pub fn encode(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    encode_into(s, &mut out);
    out
}

pub fn encode_into(s: &str, out: &mut Vec<u8>) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b != 0 && b < 0x80 {
            // ASCII fast-path: bulk-copy a run of pure ASCII (no NULs).
            let start = i;
            while i < bytes.len() && bytes[i] != 0 && bytes[i] < 0x80 {
                i += 1;
            }
            out.extend_from_slice(&bytes[start..i]);
            continue;
        }
        if b == 0 {
            out.extend_from_slice(&[0xC0, 0x80]);
            i += 1;
            continue;
        }
        // Slow path: decode one UTF-8 scalar, re-emit as modified UTF-8.
        // We know `i` points at a valid leading byte because `s` is `&str`.
        let ch = s[i..].chars().next().expect("str byte run starts on char");
        emit_non_ascii(ch, out);
        i += ch.len_utf8();
    }
}

fn emit_non_ascii(ch: char, out: &mut Vec<u8>) {
    let c = u32::from(ch);
    if c < 0x800 {
        out.extend_from_slice(&[0xC0 | (c >> 6) as u8, 0x80 | (c & 0x3F) as u8]);
    } else if c < 0x1_0000 {
        emit_three(c, out);
    } else {
        let c = c - 0x1_0000;
        emit_three(0xD800 | (c >> 10), out);
        emit_three(0xDC00 | (c & 0x3FF), out);
    }
}

#[inline]
fn emit_three(c: u32, out: &mut Vec<u8>) {
    out.extend_from_slice(&[
        0xE0 | (c >> 12) as u8,
        0x80 | ((c >> 6) & 0x3F) as u8,
        0x80 | (c & 0x3F) as u8,
    ]);
}

pub fn decode(bytes: &[u8]) -> Result<String> {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        if b0 == 0 {
            return Err(Error::InvalidUtf8 {
                offset: i,
                reason: "bare NUL byte",
            });
        }
        if b0 < 0x80 {
            out.push(b0 as char);
            i += 1;
        } else if b0 & 0xE0 == 0xC0 {
            let (cp, len) = decode_two(bytes, i)?;
            out.push(char::from_u32(cp).ok_or(Error::InvalidUtf8 {
                offset: i,
                reason: "codepoint out of range",
            })?);
            i += len;
        } else if b0 & 0xF0 == 0xE0 {
            let (cp, len) = decode_three(bytes, i)?;
            if (0xD800..0xDC00).contains(&cp) {
                let (low, len2) = decode_three(bytes, i + len)?;
                if !(0xDC00..0xE000).contains(&low) {
                    return Err(Error::InvalidUtf8 {
                        offset: i + len,
                        reason: "expected low surrogate",
                    });
                }
                let full = 0x1_0000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                out.push(char::from_u32(full).ok_or(Error::InvalidUtf8 {
                    offset: i,
                    reason: "surrogate decode out of range",
                })?);
                i += len + len2;
            } else {
                out.push(char::from_u32(cp).ok_or(Error::InvalidUtf8 {
                    offset: i,
                    reason: "codepoint out of range",
                })?);
                i += len;
            }
        } else {
            return Err(Error::InvalidUtf8 {
                offset: i,
                reason: "invalid leading byte",
            });
        }
    }
    Ok(out)
}

fn decode_two(bytes: &[u8], i: usize) -> Result<(u32, usize)> {
    let group = bytes.get(i..i + 2).ok_or(Error::InvalidUtf8 {
        offset: i,
        reason: "truncated 2-byte group",
    })?;
    let [b0, b1] = [group[0], group[1]];
    if b1 & 0xC0 != 0x80 {
        return Err(Error::InvalidUtf8 {
            offset: i + 1,
            reason: "bad continuation byte",
        });
    }
    Ok(((u32::from(b0 & 0x1F) << 6) | u32::from(b1 & 0x3F), 2))
}

fn decode_three(bytes: &[u8], i: usize) -> Result<(u32, usize)> {
    let group = bytes.get(i..i + 3).ok_or(Error::InvalidUtf8 {
        offset: i,
        reason: "truncated 3-byte group",
    })?;
    let [b0, b1, b2] = [group[0], group[1], group[2]];
    if b0 & 0xF0 != 0xE0 || b1 & 0xC0 != 0x80 || b2 & 0xC0 != 0x80 {
        return Err(Error::InvalidUtf8 {
            offset: i,
            reason: "bad 3-byte group",
        });
    }
    Ok((
        (u32::from(b0 & 0x0F) << 12) | (u32::from(b1 & 0x3F) << 6) | u32::from(b2 & 0x3F),
        3,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_roundtrip() {
        assert_eq!(encode("hello"), b"hello");
        assert_eq!(decode(b"hello").unwrap(), "hello");
    }

    #[test]
    fn nul_is_two_bytes() {
        assert_eq!(encode("\0"), [0xC0, 0x80]);
        assert_eq!(decode(&[0xC0, 0x80]).unwrap(), "\0");
    }

    #[test]
    fn supplementary_char_uses_surrogates() {
        let bytes = encode("\u{1F980}");
        assert_eq!(bytes.len(), 6);
        assert_eq!(decode(&bytes).unwrap(), "\u{1F980}");
    }

    #[test]
    fn japanese_roundtrip() {
        let s = "こんにちは";
        assert_eq!(decode(&encode(s)).unwrap(), s);
    }

    #[test]
    fn mixed_ascii_and_unicode_roundtrip() {
        let s = "java/lang/String でも\0通る";
        assert_eq!(decode(&encode(s)).unwrap(), s);
    }

    #[test]
    fn bare_nul_byte_errors() {
        let err = decode(&[b'a', 0, b'b']).unwrap_err();
        assert!(matches!(err, Error::InvalidUtf8 { offset: 1, .. }));
    }
}
