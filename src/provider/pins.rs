//! What the `container` mode builds from, per release: the GNU archive's SHA-256,
//! the digest-pinned builder image, and the build script, exactly as CI's
//! provider workflow uses them (`examples/glibc-X.Y/`; 2.17's are in
//! `examples/glibc-two-paths/`).

/// One release's pinned container build.
pub struct Pins {
    pub release: &'static str,
    /// `INPUTS.toml`, with `source_sha256` and `builder_image`; empty for 2.17.
    inputs: &'static str,
    /// Runs inside the builder with the work directory mounted at `/work`.
    pub script: &'static str,
}

const PINS: [Pins; 28] = [
    Pins {
        release: "2.17",
        inputs: "",
        script: include_str!("../../examples/glibc-two-paths/build-inputs.sh"),
    },
    Pins {
        release: "2.18",
        inputs: include_str!("../../examples/glibc-2.18/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.18/build-inputs.sh"),
    },
    Pins {
        release: "2.19",
        inputs: include_str!("../../examples/glibc-2.19/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.19/build-inputs.sh"),
    },
    Pins {
        release: "2.20",
        inputs: include_str!("../../examples/glibc-2.20/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.20/build-inputs.sh"),
    },
    Pins {
        release: "2.21",
        inputs: include_str!("../../examples/glibc-2.21/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.21/build-inputs.sh"),
    },
    Pins {
        release: "2.22",
        inputs: include_str!("../../examples/glibc-2.22/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.22/build-inputs.sh"),
    },
    Pins {
        release: "2.23",
        inputs: include_str!("../../examples/glibc-2.23/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.23/build-inputs.sh"),
    },
    Pins {
        release: "2.24",
        inputs: include_str!("../../examples/glibc-2.24/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.24/build-inputs.sh"),
    },
    Pins {
        release: "2.25",
        inputs: include_str!("../../examples/glibc-2.25/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.25/build-inputs.sh"),
    },
    Pins {
        release: "2.26",
        inputs: include_str!("../../examples/glibc-2.26/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.26/build-inputs.sh"),
    },
    Pins {
        release: "2.27",
        inputs: include_str!("../../examples/glibc-2.27/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.27/build-inputs.sh"),
    },
    Pins {
        release: "2.28",
        inputs: include_str!("../../examples/glibc-2.28/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.28/build-inputs.sh"),
    },
    Pins {
        release: "2.29",
        inputs: include_str!("../../examples/glibc-2.29/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.29/build-inputs.sh"),
    },
    Pins {
        release: "2.30",
        inputs: include_str!("../../examples/glibc-2.30/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.30/build-inputs.sh"),
    },
    Pins {
        release: "2.31",
        inputs: include_str!("../../examples/glibc-2.31/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.31/build-inputs.sh"),
    },
    Pins {
        release: "2.32",
        inputs: include_str!("../../examples/glibc-2.32/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.32/build-inputs.sh"),
    },
    Pins {
        release: "2.33",
        inputs: include_str!("../../examples/glibc-2.33/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.33/build-inputs.sh"),
    },
    Pins {
        release: "2.34",
        inputs: include_str!("../../examples/glibc-2.34/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.34/build-inputs.sh"),
    },
    Pins {
        release: "2.35",
        inputs: include_str!("../../examples/glibc-2.35/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.35/build-inputs.sh"),
    },
    Pins {
        release: "2.36",
        inputs: include_str!("../../examples/glibc-2.36/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.36/build-inputs.sh"),
    },
    Pins {
        release: "2.37",
        inputs: include_str!("../../examples/glibc-2.37/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.37/build-inputs.sh"),
    },
    Pins {
        release: "2.38",
        inputs: include_str!("../../examples/glibc-2.38/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.38/build-inputs.sh"),
    },
    Pins {
        release: "2.39",
        inputs: include_str!("../../examples/glibc-2.39/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.39/build-inputs.sh"),
    },
    Pins {
        release: "2.40",
        inputs: include_str!("../../examples/glibc-2.40/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.40/build-inputs.sh"),
    },
    Pins {
        release: "2.41",
        inputs: include_str!("../../examples/glibc-2.41/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.41/build-inputs.sh"),
    },
    Pins {
        release: "2.42",
        inputs: include_str!("../../examples/glibc-2.42/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.42/build-inputs.sh"),
    },
    Pins {
        release: "2.43",
        inputs: include_str!("../../examples/glibc-2.43/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.43/build-inputs.sh"),
    },
    Pins {
        release: "2.44",
        inputs: include_str!("../../examples/glibc-2.44/INPUTS.toml"),
        script: include_str!("../../examples/glibc-2.44/build-inputs.sh"),
    },
];

/// 2.17's archive hash and image, from `examples/glibc-two-paths/run.sh`.
const ARCHIVE_2_17: &str = "6914e337401e0e0ade23694e1b2c52a5f09e4eda3270c67e7c3ba93a89b5b23e";
const IMAGE_2_17: &str = "docker.io/library/centos@sha256:dead07b4d8ed7e29e98de0f4504d87e8880d4347859d839686a31da35a3b532f";

impl Pins {
    fn value(&self, key: &str) -> Result<&'static str, String> {
        let prefix = format!("{key} = \"");
        self.inputs
            .lines()
            .find_map(|line| line.strip_prefix(&prefix)?.strip_suffix('"'))
            .ok_or_else(|| format!("glibc {}: INPUTS.toml lacks {key}", self.release))
    }

    /// The SHA-256 of `glibc-X.Y.tar.xz`.
    pub fn archive_sha256(&self) -> Result<&'static str, String> {
        let value = if self.release == "2.17" {
            ARCHIVE_2_17
        } else {
            self.value("source_sha256")?
        };
        if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("glibc {}: invalid archive SHA-256", self.release));
        }
        Ok(value)
    }

    /// The builder image, pinned by digest.
    pub fn image(&self) -> Result<&'static str, String> {
        let value = if self.release == "2.17" {
            IMAGE_2_17
        } else {
            self.value("builder_image")?
        };
        if !value.starts_with("docker.io/library/") || !value.contains("@sha256:") {
            return Err(format!(
                "glibc {}: the image must be pinned by digest",
                self.release
            ));
        }
        Ok(value)
    }

    /// The archive's file name.
    pub fn archive(&self) -> String {
        format!("glibc-{}.tar.xz", self.release)
    }
}

/// The pins of RELEASE.
pub fn pins(release: &str) -> Result<&'static Pins, String> {
    PINS.iter()
        .find(|p| p.release == release)
        .ok_or_else(|| format!("glibc {release} has no pinned container build"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_release_is_pinned() {
        assert_eq!(
            PINS.iter().map(|p| p.release).collect::<Vec<_>>(),
            super::super::RELEASES
        );
        for pins in &PINS {
            pins.archive_sha256().unwrap();
            pins.image().unwrap();
            assert!(pins
                .script
                .contains("make install_root=/work/source install"));
            if !pins.inputs.is_empty() {
                assert!(pins
                    .inputs
                    .contains(&format!("release = \"{}\"", pins.release)));
            }
        }
        let run = include_str!("../../examples/glibc-two-paths/run.sh");
        assert!(run.contains(&format!("fetch-glibc.sh\" 2.17 {ARCHIVE_2_17}")));
        assert!(run.contains(&format!("image={IMAGE_2_17}")));
        assert!(pins("2.16").is_err());
    }
}
