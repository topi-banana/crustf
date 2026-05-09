//! IEEE 802.3 / PKZIP CRC-32 (polynomial `0xEDB88320`, reflected).
//!
//! A dedicated implementation is cheaper to maintain than a dependency
//! because the algorithm is trivial, the lookup table is fully `const`,
//! and the crate has no other reason to link into the `std` feature tree.

const POLY: u32 = 0xEDB88320;

const CRC32_TABLE: [u32; 256] = build_table();

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            crc = if crc & 1 == 1 {
                POLY ^ (crc >> 1)
            } else {
                crc >> 1
            };
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
}

/// Compute the CRC-32 of a byte slice in one call.
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut h = Hasher::new();
    h.update(bytes);
    h.finalize()
}

/// Incremental CRC-32 computation for streaming callers.
#[derive(Debug, Clone)]
pub struct Hasher {
    state: u32,
}

impl Hasher {
    #[must_use]
    pub const fn new() -> Self {
        Self { state: !0 }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        let mut crc = self.state;
        for &b in bytes {
            let idx = ((crc ^ u32::from(b)) & 0xFF) as usize;
            crc = CRC32_TABLE[idx] ^ (crc >> 8);
        }
        self.state = crc;
    }

    #[must_use]
    pub fn finalize(self) -> u32 {
        !self.state
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(crc32(&[]), 0);
    }

    #[test]
    fn abc() {
        // Known CRC-32 of ASCII "123456789": 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn chunked_matches_one_shot() {
        let msg: &[u8] = b"the quick brown fox jumps over the lazy dog";
        let mut h = Hasher::new();
        h.update(&msg[..10]);
        h.update(&msg[10..]);
        assert_eq!(crc32(msg), h.finalize());
    }
}
