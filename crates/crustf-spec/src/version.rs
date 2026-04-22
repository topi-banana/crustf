//! Class file version numbers (JVMS §4.1).

use core::fmt;

/// JVMS §4.1 Table 4.1-A: class file major version = Java SE release + 44.
const JAVA_SE_MAJOR_OFFSET: u16 = 44;

/// Sentinel minor version that signals preview features (JVMS §4.1).
pub const PREVIEW_MINOR: u16 = 0xFFFF;

/// Class file version (`major`, `minor`) as emitted after the magic word.
///
/// Note that JVMS §4.1 writes `minor_version` before `major_version` on disk;
/// this struct prefers the `major` first field order because API callers
/// think in terms of "Java N" (the major number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

impl Version {
    #[inline]
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Convert a Java SE release (for example `8`, `17`, `21`, `25`) to the
    /// corresponding class file major version. Minor is zero.
    #[inline]
    #[must_use]
    pub const fn java_se(release: u16) -> Self {
        Self {
            major: release + JAVA_SE_MAJOR_OFFSET,
            minor: 0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn is_preview(self) -> bool {
        self.minor == PREVIEW_MINOR
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// JDK 1.1 predates the `major = 44 + release` formula; its version is `45.3`.
pub const JAVA_1_1: Version = Version::new(45, 3);
pub const JAVA_1_2: Version = Version::java_se(2);
pub const JAVA_1_3: Version = Version::java_se(3);
pub const JAVA_1_4: Version = Version::java_se(4);
pub const JAVA_5: Version = Version::java_se(5);
pub const JAVA_6: Version = Version::java_se(6);
pub const JAVA_7: Version = Version::java_se(7);
pub const JAVA_8: Version = Version::java_se(8);
pub const JAVA_9: Version = Version::java_se(9);
pub const JAVA_10: Version = Version::java_se(10);
pub const JAVA_11: Version = Version::java_se(11);
pub const JAVA_12: Version = Version::java_se(12);
pub const JAVA_13: Version = Version::java_se(13);
pub const JAVA_14: Version = Version::java_se(14);
pub const JAVA_15: Version = Version::java_se(15);
pub const JAVA_16: Version = Version::java_se(16);
pub const JAVA_17: Version = Version::java_se(17);
pub const JAVA_18: Version = Version::java_se(18);
pub const JAVA_19: Version = Version::java_se(19);
pub const JAVA_20: Version = Version::java_se(20);
pub const JAVA_21: Version = Version::java_se(21);
pub const JAVA_22: Version = Version::java_se(22);
pub const JAVA_23: Version = Version::java_se(23);
pub const JAVA_24: Version = Version::java_se(24);
pub const JAVA_25: Version = Version::java_se(25);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_se_maps_to_expected_major() {
        assert_eq!(JAVA_8.major, 52);
        assert_eq!(JAVA_17.major, 61);
        assert_eq!(JAVA_21.major, 65);
        assert_eq!(JAVA_25.major, 69);
    }

    #[test]
    fn preview_minor_matches_sentinel() {
        let v = Version::new(65, PREVIEW_MINOR);
        assert!(v.is_preview());
    }
}
