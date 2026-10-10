//! Release bundle descriptor (`kernel-abi-provider-v1`) and offline integrity
//! verification. See docs/PROVIDER-CONTRACT.md.

use crate::PROVIDER_FORMAT;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// File name of the descriptor at the root of a bundle.
pub const DESCRIPTOR_FILE: &str = "kernel-abi-provider.txt";
/// Largest accepted descriptor.
pub const DESCRIPTOR_MAX: usize = 16 * 1024;

const PATH_KEYS: &[&str] = &[
    "kernel-seccomp",
    "syscall-probe",
    "profiles",
    "distros",
    "inventory",
];
const VALUE_KEYS: &[&str] = &["version", "arch", "commit"];

/// A parsed descriptor. Paths are relative to the descriptor's directory.
#[derive(Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub version: String,
    pub arch: String,
    pub commit: String,
    pub kernel_seccomp: PathBuf,
    pub syscall_probe: PathBuf,
    pub profiles: PathBuf,
    pub distros: PathBuf,
    pub inventory: PathBuf,
}

impl Descriptor {
    /// The descriptor text, in the field order the packager writes.
    pub fn render(&self) -> String {
        let p = |p: &Path| p.to_string_lossy().replace('\\', "/");
        format!(
            "{PROVIDER_FORMAT}\nversion\t{}\narch\t{}\ncommit\t{}\nkernel-seccomp\t{}\n\
             syscall-probe\t{}\nprofiles\t{}\ndistros\t{}\ninventory\t{}\n",
            self.version,
            self.arch,
            self.commit,
            p(&self.kernel_seccomp),
            p(&self.syscall_probe),
            p(&self.profiles),
            p(&self.distros),
            p(&self.inventory),
        )
    }
}

/// Parses a descriptor. The text must be at most 16 KiB of UTF-8 with LF line
/// endings and a final newline; the first line is the format name and every
/// other line is `key<TAB>value`. Unknown, duplicate, missing, or empty
/// fields, control characters, and paths that are absolute or contain `..`
/// are rejected.
pub fn parse_descriptor(text: &str) -> Result<Descriptor, String> {
    if text.len() > DESCRIPTOR_MAX {
        return Err(format!("descriptor exceeds {DESCRIPTOR_MAX} bytes"));
    }
    let body = text
        .strip_suffix('\n')
        .ok_or("descriptor must end with a newline")?;
    let mut lines = body.split('\n');
    match lines.next() {
        Some(first) if first == PROVIDER_FORMAT => {}
        Some(first) => return Err(format!("unsupported descriptor format {first:?}")),
        None => return Err("empty descriptor".to_string()),
    }
    let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
    for line in lines {
        if line.chars().any(|c| c.is_control() && c != '\t') {
            return Err(format!("control character in line {line:?}"));
        }
        let (key, value) = line
            .split_once('\t')
            .ok_or_else(|| format!("line without a tab: {line:?}"))?;
        if !PATH_KEYS.contains(&key) && !VALUE_KEYS.contains(&key) {
            return Err(format!("unknown field {key:?}"));
        }
        if value.is_empty() || value.contains('\t') {
            return Err(format!("field {key:?} needs exactly one non-empty value"));
        }
        if fields.insert(key, value).is_some() {
            return Err(format!("duplicate field {key:?}"));
        }
    }
    let get = |key: &str| {
        fields
            .get(key)
            .map(|v| v.to_string())
            .ok_or_else(|| format!("missing field {key:?}"))
    };
    let path = |key: &str| -> Result<PathBuf, String> {
        let value = get(key)?;
        let p = PathBuf::from(&value);
        let plain = p.components().all(|c| matches!(c, Component::Normal(_)));
        if !plain {
            return Err(format!(
                "field {key:?} must be a relative path without `..`: {value:?}"
            ));
        }
        Ok(p)
    };
    Ok(Descriptor {
        version: get("version")?,
        arch: get("arch")?,
        commit: get("commit")?,
        kernel_seccomp: path("kernel-seccomp")?,
        syscall_probe: path("syscall-probe")?,
        profiles: path("profiles")?,
        distros: path("distros")?,
        inventory: path("inventory")?,
    })
}

/// What `verify_bundle` checked.
#[derive(Debug)]
pub struct BundleReport {
    pub descriptor: Descriptor,
    pub files_verified: usize,
}

/// Verifies a bundle offline: the descriptor parses, every path it names
/// exists inside the bundle, the inventory lists every regular file in the
/// bundle except itself exactly once, every hash matches, and the bundle
/// contains no symlinks or special files.
pub fn verify_bundle(root: &Path) -> Result<BundleReport, String> {
    let read = |p: &Path| fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let text = String::from_utf8(read(&root.join(DESCRIPTOR_FILE))?)
        .map_err(|_| "descriptor is not UTF-8".to_string())?;
    let descriptor = parse_descriptor(&text)?;
    for (key, rel, dir) in [
        ("kernel-seccomp", &descriptor.kernel_seccomp, false),
        ("syscall-probe", &descriptor.syscall_probe, false),
        ("profiles", &descriptor.profiles, true),
        ("distros", &descriptor.distros, false),
        ("inventory", &descriptor.inventory, false),
    ] {
        let meta = fs::symlink_metadata(root.join(rel))
            .map_err(|e| format!("{key} {}: {e}", rel.display()))?;
        if meta.is_dir() != dir || meta.file_type().is_symlink() {
            let kind = if dir { "directory" } else { "regular file" };
            return Err(format!("{key} {} is not a {kind}", rel.display()));
        }
    }

    let inventory_text = String::from_utf8(read(&root.join(&descriptor.inventory))?)
        .map_err(|_| "inventory is not UTF-8".to_string())?;
    let mut listed: BTreeMap<String, String> = BTreeMap::new();
    for line in inventory_text.lines() {
        let (hash, name) = line
            .split_once("  ")
            .ok_or_else(|| format!("inventory line is not `HASH  PATH`: {line:?}"))?;
        let name = name.strip_prefix("./").unwrap_or(name);
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("bad hash for {name}"));
        }
        if !Path::new(name)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        {
            return Err(format!("inventory path must be relative: {name:?}"));
        }
        if listed
            .insert(name.to_string(), hash.to_ascii_lowercase())
            .is_some()
        {
            return Err(format!("inventory lists {name} twice"));
        }
    }

    let inventory_name = descriptor.inventory.to_string_lossy().replace('\\', "/");
    let mut present = Vec::new();
    walk(root, root, &mut present)?;
    present.retain(|p| *p != inventory_name);
    for name in &present {
        if !listed.contains_key(name) {
            return Err(format!("{name} is not in the inventory"));
        }
    }
    for (name, expected) in &listed {
        if !present.contains(name) {
            return Err(format!("inventory lists missing file {name}"));
        }
        let actual = hex(&sha256(&read(&root.join(name))?));
        if &actual != expected {
            return Err(format!("hash mismatch for {name}"));
        }
    }
    Ok(BundleReport {
        descriptor,
        files_verified: listed.len(),
    })
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let rel = path
            .strip_prefix(root)
            .expect("walk stays under root")
            .to_string_lossy()
            .replace('\\', "/");
        if kind.is_dir() {
            walk(root, &path, out)?;
        } else if kind.is_file() {
            out.push(rel);
        } else {
            return Err(format!("{rel} is a symlink or special file"));
        }
    }
    Ok(())
}

/// Lowercase hexadecimal.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 (FIPS 180-4).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    for block in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(v);
        }
    }
    let mut out = [0u8; 32];
    for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(h) {
        *chunk = word.to_be_bytes();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Descriptor {
        Descriptor {
            version: "0.1.0".into(),
            arch: "x86_64".into(),
            commit: "0123abc".into(),
            kernel_seccomp: "bin/kernel-seccomp".into(),
            syscall_probe: "bin/syscall-probe".into(),
            profiles: "profiles".into(),
            distros: "data/distros.tsv".into(),
            inventory: "SHA256SUMS".into(),
        }
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let long = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(
            hex(&sha256(long)),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn descriptor_round_trips() {
        let d = sample();
        assert_eq!(parse_descriptor(&d.render()).unwrap(), d);
    }

    #[test]
    fn descriptor_rejects_bad_input() {
        let good = sample().render();
        let cases = [
            good.trim_end().to_string(),
            good.replace(PROVIDER_FORMAT, "kernel-abi-provider-v2"),
            good.replace("arch\tx86_64\n", ""),
            good.replace("arch\tx86_64\n", "arch\tx86_64\narch\tx86_64\n"),
            good.replace("arch\tx86_64\n", "colour\tblue\n"),
            good.replace("profiles\tprofiles", "profiles\t/abs/profiles"),
            good.replace("profiles\tprofiles", "profiles\t../profiles"),
            good.replace("version\t0.1.0", "version\t"),
            good.replace('\n', "\r\n"),
        ];
        for bad in cases {
            assert!(parse_descriptor(&bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn verifies_a_bundle_and_detects_tampering() {
        let root = std::env::temp_dir().join(format!("kat-bundle-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let files = [
            ("bin/kernel-seccomp", "gen"),
            ("bin/syscall-probe", "probe"),
            ("profiles/kernel/linux-4.12.json", "{}"),
            ("data/distros.tsv", "# none\n"),
            (DESCRIPTOR_FILE, ""),
        ];
        for (name, body) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let body = if name == DESCRIPTOR_FILE {
                sample().render()
            } else {
                body.to_string()
            };
            fs::write(path, body).unwrap();
        }
        let sums: String = files
            .iter()
            .map(|(name, _)| {
                let bytes = fs::read(root.join(name)).unwrap();
                format!("{}  {name}\n", hex(&sha256(&bytes)))
            })
            .collect();
        fs::write(root.join("SHA256SUMS"), &sums).unwrap();
        assert_eq!(verify_bundle(&root).unwrap().files_verified, files.len());

        fs::write(root.join("profiles/kernel/linux-4.12.json"), "{ }").unwrap();
        assert!(verify_bundle(&root).unwrap_err().contains("hash mismatch"));
        fs::write(root.join("profiles/kernel/linux-4.12.json"), "{}").unwrap();
        fs::write(root.join("extra.txt"), "x").unwrap();
        assert!(verify_bundle(&root)
            .unwrap_err()
            .contains("not in the inventory"));
        fs::remove_dir_all(&root).unwrap();
    }
}
