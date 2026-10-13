//! Linux target selection shared by the rustcommons toolchain.
//!
//! A target is chosen in one of two ways:
//!
//! - **A distribution preset** (`--distro sles-12-sp5`): supplies the glibc
//!   floor, the kernel floor, and a container image with the real userland.
//! - **Explicit floors** (`--glibc 2.22`, `--kernel 4.12`, or a versioned
//!   target such as `x86_64-unknown-linux-gnu.2.22`): each is optional. A
//!   missing kernel floor means the kernel is **not simulated**: programs run
//!   against the host kernel.
//!
//! A preset cannot be combined with explicit floors: "SLES 12 SP5 with glibc
//! 2.28" is not a real system. [`Selection::resolve`] enforces these rules.
//!
//! ```
//! use linux_targets::{Release, Request, Selection};
//!
//! // A preset sets both floors and a test image.
//! let sles = Selection::resolve(&Request { distro: Some("sles-12-sp5"), ..Default::default() })?;
//! assert_eq!(sles.glibc, Some(Release::parse("2.22")?));
//! assert_eq!(sles.kernel, Some(Release::parse("4.12")?));
//! assert!(sles.image().is_some());
//!
//! // A versioned target sets only the glibc floor; the kernel is not simulated.
//! let target = Selection::resolve(&Request {
//!     target: Some("x86_64-unknown-linux-gnu.2.17"),
//!     ..Default::default()
//! })?;
//! assert_eq!(target.triple.as_deref(), Some("x86_64-unknown-linux-gnu"));
//! assert_eq!(target.glibc, Some(Release::parse("2.17")?));
//! assert_eq!(target.kernel, None);
//! assert_eq!(target.describe(), "glibc 2.17, kernel not simulated (host)");
//!
//! // A preset cannot be combined with explicit floors.
//! assert!(Selection::resolve(&Request {
//!     distro: Some("centos-7"),
//!     glibc: Some("2.28"),
//!     ..Default::default()
//! })
//! .is_err());
//! # Ok::<(), String>(())
//! ```

#![warn(missing_docs)]

use std::cmp::Ordering;
use std::fmt;

const DISTRO_DATA: &str = include_str!("../data/distros.tsv");

/// A `major.minor` release number, such as a kernel (4.12) or glibc (2.22)
/// release. Comparison uses only these two components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Release {
    /// The major number: 4 in kernel 4.12, 2 in glibc 2.22.
    pub major: u32,
    /// The minor number: 12 in kernel 4.12, 22 in glibc 2.22.
    pub minor: u32,
}

impl Release {
    /// Parses `4.12`, `4.12.14`, `v5.14`, or a distribution kernel release
    /// such as `4.12.14-122.37-default`. Only the leading `major.minor` is
    /// used.
    ///
    /// # Errors
    ///
    /// Returns a message when `s` does not start with two numbers separated by
    /// a non-digit, such as `4` or `abc`.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim().trim_start_matches(['v', 'V']);
        let mut parts = s.split(|c: char| !c.is_ascii_digit());
        let major = parts.next().and_then(|p| p.parse().ok());
        let minor = parts.next().and_then(|p| p.parse().ok());
        match (major, minor) {
            (Some(major), Some(minor)) => Ok(Release { major, minor }),
            _ => Err(format!("not a release number: {s:?} (expected e.g. 4.12)")),
        }
    }
}

impl Ord for Release {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor).cmp(&(other.major, other.minor))
    }
}

impl PartialOrd for Release {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A mainline Linux kernel release.
pub type KernelVersion = Release;
/// A GNU C Library release.
pub type GlibcVersion = Release;

/// A distribution preset: a kernel floor, a glibc floor, and a test image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Distro {
    /// The preset's id for `--distro`, such as `sles-12-sp5`.
    pub id: &'static str,
    /// The release's full name, such as `SUSE Linux Enterprise Server 12 SP5`.
    pub name: &'static str,
    /// Mainline base of the release's GA kernel.
    pub kernel: KernelVersion,
    /// The glibc release the distribution ships.
    pub glibc: GlibcVersion,
    /// Container image providing the release's userland.
    pub image: &'static str,
    /// Syscalls the vendor kernel backports beyond its mainline base.
    pub kernel_backports: Vec<&'static str>,
    /// How the values were established, e.g. `image-checked`.
    pub status: &'static str,
}

/// All distribution presets from `data/distros.tsv`.
pub fn distros() -> Vec<Distro> {
    DISTRO_DATA
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|line| {
            let cols: Vec<&'static str> = line.split('\t').map(str::trim).collect();
            let [id, name, kernel, glibc, image, backports, status] = cols[..] else {
                panic!("distro data row needs 7 columns: {cols:?}");
            };
            Distro {
                id,
                name,
                kernel: Release::parse(kernel).expect("distro kernel"),
                glibc: Release::parse(glibc).expect("distro glibc"),
                image,
                kernel_backports: if backports == "-" {
                    Vec::new()
                } else {
                    backports.split(',').collect()
                },
                status,
            }
        })
        .collect()
}

/// The preset with this id.
pub fn distro(id: &str) -> Option<Distro> {
    distros().into_iter().find(|d| d.id == id)
}

/// What the user selected, before validation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Request<'a> {
    /// `--distro ID`
    pub distro: Option<&'a str>,
    /// `--glibc X.Y`
    pub glibc: Option<&'a str>,
    /// `--kernel X.Y`
    pub kernel: Option<&'a str>,
    /// `--target TRIPLE` or `--target TRIPLE.X.Y`
    pub target: Option<&'a str>,
}

/// A validated selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// The preset, when one was chosen.
    pub distro: Option<Distro>,
    /// Rust target triple without any glibc suffix, when one was given.
    pub triple: Option<String>,
    /// glibc floor; `None` means the host's glibc.
    pub glibc: Option<GlibcVersion>,
    /// Kernel floor; `None` means the kernel is not simulated (host kernel).
    pub kernel: Option<KernelVersion>,
}

impl Selection {
    /// Applies the selection rules:
    ///
    /// - `distro` excludes `glibc`, `kernel`, and a versioned `target`.
    /// - A versioned `target` (`TRIPLE.X.Y`) sets the glibc floor; it must
    ///   agree with `glibc` if both are given. It never implies a kernel.
    /// - Anything not given stays `None`: host glibc, host kernel.
    ///
    /// # Errors
    ///
    /// Returns a message when a preset is combined with explicit floors, the
    /// preset id is unknown, a release number does not parse, `glibc` disagrees
    /// with the target's suffix, or a suffixed target is not a `-gnu` triple.
    pub fn resolve(req: &Request<'_>) -> Result<Self, String> {
        let (triple, suffix) = match req.target {
            Some(t) => {
                let (triple, suffix) = split_versioned_target(t)?;
                (Some(triple), suffix)
            }
            None => (None, None),
        };
        if let Some(id) = req.distro {
            if req.glibc.is_some() || req.kernel.is_some() || suffix.is_some() {
                return Err(format!(
                    "--distro {id} sets the glibc and kernel floors; \
                     do not combine it with --glibc, --kernel, or a versioned --target"
                ));
            }
            let d = distro(id).ok_or_else(|| format!("unknown distro {id:?}"))?;
            return Ok(Selection {
                glibc: Some(d.glibc),
                kernel: Some(d.kernel),
                distro: Some(d),
                triple,
            });
        }
        let glibc = match (req.glibc.map(Release::parse).transpose()?, suffix) {
            (Some(a), Some(b)) if a != b => {
                return Err(format!("--glibc {a} conflicts with --target suffix .{b}"))
            }
            (a, b) => a.or(b),
        };
        Ok(Selection {
            distro: None,
            triple,
            glibc,
            kernel: req.kernel.map(Release::parse).transpose()?,
        })
    }

    /// The container image for runtime tests, if the selection has one.
    pub fn image(&self) -> Option<&'static str> {
        self.distro.as_ref().map(|d| d.image)
    }

    /// A one-line description for tool output, e.g.
    /// `glibc 2.22, kernel 4.12 (SUSE Linux Enterprise Server 12 SP5)` or
    /// `glibc host, kernel not simulated (host)`.
    pub fn describe(&self) -> String {
        let glibc = self.glibc.map_or("host".to_string(), |g| g.to_string());
        let kernel = self
            .kernel
            .map_or("not simulated (host)".to_string(), |k| k.to_string());
        match &self.distro {
            Some(d) => format!("glibc {glibc}, kernel {kernel} ({})", d.name),
            None => format!("glibc {glibc}, kernel {kernel}"),
        }
    }
}

/// Splits `x86_64-unknown-linux-gnu.2.17` into the Rust triple and the glibc
/// floor. A triple without a numeric `.X.Y` suffix is returned unchanged.
///
/// # Errors
///
/// Returns a message when a numeric suffix follows a triple that does not end
/// in `-gnu`, since only glibc targets take a glibc floor.
pub fn split_versioned_target(target: &str) -> Result<(String, Option<GlibcVersion>), String> {
    let mut parts = target.rsplitn(3, '.');
    let (minor, major, triple) = (parts.next(), parts.next(), parts.next());
    match (triple, major, minor) {
        (Some(triple), Some(major), Some(minor))
            if !major.is_empty()
                && major.bytes().all(|b| b.is_ascii_digit())
                && !minor.is_empty()
                && minor.bytes().all(|b| b.is_ascii_digit()) =>
        {
            if !triple.ends_with("-gnu") {
                return Err(format!(
                    "a glibc suffix needs a -gnu target triple: {target}"
                ));
            }
            Ok((
                triple.to_string(),
                Some(Release::parse(&format!("{major}.{minor}"))?),
            ))
        }
        _ => Ok((target.to_string(), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> Release {
        Release::parse(s).unwrap()
    }

    #[test]
    fn parses_release_strings() {
        assert_eq!(r("4.12.14-122.37-default"), r("4.12"));
        assert_eq!(r("v5.14"), r("5.14.0"));
        assert!(r("2.17") < r("2.22"));
        assert!(Release::parse("linux").is_err());
    }

    #[test]
    fn presets_parse() {
        let all = distros();
        assert_eq!(all.len(), 28);
        let sles = distro("sles-12-sp5").unwrap();
        assert_eq!((sles.glibc, sles.kernel), (r("2.22"), r("4.12")));
        assert_eq!(sles.image, "registry.suse.com/suse/sles12sp5:latest");
    }

    #[test]
    fn distro_sets_both_floors() {
        let s = Selection::resolve(&Request {
            distro: Some("sles-12-sp5"),
            ..Default::default()
        })
        .unwrap();
        assert_eq!((s.glibc, s.kernel), (Some(r("2.22")), Some(r("4.12"))));
        assert_eq!(s.image(), Some("registry.suse.com/suse/sles12sp5:latest"));
        assert_eq!(
            s.describe(),
            "glibc 2.22, kernel 4.12 (SUSE Linux Enterprise Server 12 SP5)"
        );
    }

    #[test]
    fn distro_excludes_explicit_floors() {
        for req in [
            Request {
                distro: Some("sles-12-sp5"),
                glibc: Some("2.28"),
                ..Default::default()
            },
            Request {
                distro: Some("sles-12-sp5"),
                kernel: Some("5.14"),
                ..Default::default()
            },
            Request {
                distro: Some("sles-12-sp5"),
                target: Some("x86_64-unknown-linux-gnu.2.17"),
                ..Default::default()
            },
        ] {
            assert!(Selection::resolve(&req).is_err(), "{req:?}");
        }
        assert!(Selection::resolve(&Request {
            distro: Some("nope"),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn missing_kernel_means_host_kernel() {
        let s = Selection::resolve(&Request {
            target: Some("x86_64-unknown-linux-gnu.2.17"),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(s.triple.as_deref(), Some("x86_64-unknown-linux-gnu"));
        assert_eq!(
            (s.glibc, s.kernel, s.image()),
            (Some(r("2.17")), None, None)
        );
        assert_eq!(s.describe(), "glibc 2.17, kernel not simulated (host)");
    }

    #[test]
    fn explicit_floors_combine() {
        let s = Selection::resolve(&Request {
            glibc: Some("2.22"),
            kernel: Some("4.12"),
            ..Default::default()
        })
        .unwrap();
        assert_eq!((s.glibc, s.kernel), (Some(r("2.22")), Some(r("4.12"))));
        let k = Selection::resolve(&Request {
            kernel: Some("4.12"),
            ..Default::default()
        })
        .unwrap();
        assert_eq!((k.glibc, k.kernel), (None, Some(r("4.12"))));
        assert_eq!(k.describe(), "glibc host, kernel 4.12");
    }

    #[test]
    fn versioned_target_must_agree_with_glibc() {
        let agree = Request {
            glibc: Some("2.17"),
            target: Some("x86_64-unknown-linux-gnu.2.17"),
            ..Default::default()
        };
        assert_eq!(Selection::resolve(&agree).unwrap().glibc, Some(r("2.17")));
        let conflict = Request {
            glibc: Some("2.22"),
            ..agree
        };
        assert!(Selection::resolve(&conflict).is_err());
    }

    #[test]
    fn splits_versioned_targets() {
        assert_eq!(
            split_versioned_target("x86_64-unknown-linux-gnu.2.17").unwrap(),
            ("x86_64-unknown-linux-gnu".to_string(), Some(r("2.17")))
        );
        assert_eq!(
            split_versioned_target("x86_64-unknown-linux-gnu").unwrap(),
            ("x86_64-unknown-linux-gnu".to_string(), None)
        );
        assert_eq!(
            split_versioned_target("thumbv7em-none-eabihf").unwrap().1,
            None
        );
        assert!(split_versioned_target("x86_64-unknown-linux-musl.1.2").is_err());
    }
}
