//! Numeric versions, used for glibc releases (`2.17`) and symbol versions
//! (`GLIBC_2.2.5` without its prefix).
use std::{fmt, str::FromStr};

/// Numeric ABI version. Historical symbol versions may precede our target floor.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
///
/// Parsed from two or three dot-separated numbers (`2.17`, `2.2.5`); a missing
/// third number is zero. It displays without a zero patch.
pub struct Version {
    /// The first number.
    pub major: u32,
    /// The second number.
    pub minor: u32,
    /// The third number; zero when absent.
    pub patch: u32,
}

impl Version {
    /// Validate a glibc release directory (as opposed to a symbol version).
    ///
    /// # Errors
    ///
    /// Fails unless the version is 2.x with x at least 17 and no patch number.
    pub fn validate_release(self) -> Result<Self, String> {
        if self.major != 2 || self.minor < 17 || self.patch != 0 {
            return Err(
                "expected a glibc 2.x release starting at 2.17, without a patch component".into(),
            );
        }
        Ok(self)
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let parts: Vec<_> = text.split('.').collect();
        if !(2..=3).contains(&parts.len()) {
            return Err(format!("invalid numeric version {text:?}"));
        }
        let number = |s: &str| -> Result<u32, String> {
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("invalid numeric version {text:?}"));
            }
            s.parse()
                .map_err(|_| format!("version component too large: {text:?}"))
        };
        Ok(Self {
            major: number(parts[0])?,
            minor: number(parts[1])?,
            patch: if parts.len() == 3 {
                number(parts[2])?
            } else {
                0
            },
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)?;
        if self.patch != 0 {
            write!(f, ".{}", self.patch)?;
        }
        Ok(())
    }
}
