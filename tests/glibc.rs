#![cfg(feature = "bundled-glibc-data")]

use libc_abi_tools::{
    glibc::{self, Kind, Snapshot},
    version::Version,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

/// The bundled glibc-abilists data, exported once per test run so the
/// directory code path is exercised too.
fn data() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("glibc-data-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        glibc::export_bundled(&dir).unwrap();
        dir
    })
    .clone()
}

fn snapshot(release: &str) -> Snapshot {
    Snapshot::load(&data(), release.parse().unwrap()).unwrap()
}

#[test]
fn historical_and_modern_formats_have_identical_meaning() {
    let old = "GLIBC_2.2.5\r\n GLIBC_2.2.5 A\r\n memcpy F\r\n stderr D 0x8\r\nGLIBC_2.14\r\n memcpy F\r\n";
    let new = "GLIBC_2.2.5 GLIBC_2.2.5 A\nGLIBC_2.2.5 memcpy F\nGLIBC_2.2.5 stderr D 8\nGLIBC_2.14 memcpy F\n";
    assert_eq!(
        glibc::parse_abilist(old).unwrap(),
        glibc::parse_abilist(new).unwrap()
    );
}

#[test]
fn preserves_versions_sizes_and_explicit_tls() {
    let parsed =
        glibc::parse_abilist("GLIBC_2.2.5 x D 0x10\nGLIBC_2.17 y T 4\nGLIBC_2.34 x D 0x20\n")
            .unwrap();
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].kind, Kind::Object(16));
    assert_eq!(parsed[1].kind, Kind::Object(32));
    assert_eq!(parsed[2].kind, Kind::Tls(4));
    assert_eq!(parsed[0].version, "2.2.5".parse().unwrap());
}

#[test]
fn malformed_input_is_rejected_with_a_line_number() {
    for bad in [
        " name F",
        "GLIBC_PRIVATE secret F",
        "GLIBC_2.2.5 x Q",
        "GLIBC_2.17 x D",
        "GLIBC_2.17 x D 65536",
        "GLIBC_2.17 x F extra",
        "GLIBC_2.17 x D -4",
        "GLIBC_2.17 x D +4",
        "GLIBC_2.17 x D 0x",
        "GLIBC_2.17 x F\nGLIBC_2.17 x D 8",
        "GLIBC_2.17 x\0y F",
        "GLIBC_2.17 x F\n orphan F",
        "GLIBC_2.17.0.1 x F",
    ] {
        let err = glibc::parse_abilist(bad).unwrap_err();
        assert!(err.starts_with("line "), "{bad:?}: {err}");
    }
}

#[test]
fn absolute_nodes_and_gcc_versions_are_not_exported() {
    assert!(
        glibc::parse_abilist("GLIBC_2.17 GLIBC_2.17 A\nGCC_3.0 unwind F\n")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn all_28_pinned_releases_load_and_export_deterministically() {
    for minor in 17..=44 {
        let abi = snapshot(&format!("2.{minor}"));
        assert_eq!(abi.libraries.len(), 8);
        assert!(abi.libraries["c"].len() > 1500);
        let tsv = abi.to_tsv(None);
        assert!(tsv.starts_with("release\ttarget\tlibrary\tsymbol\tversion\tkind\tsize\n"));
        assert_eq!(tsv, snapshot(&format!("2.{minor}")).to_tsv(None));
    }
}

#[test]
fn oldest_release_retains_older_abi_tags_and_memcpy_aliases() {
    let abi = snapshot("2.17");
    let memcpy: Vec<_> = abi.libraries["c"]
        .iter()
        .filter(|s| s.name == "memcpy")
        .map(|s| s.version.to_string())
        .collect();
    assert_eq!(memcpy, ["2.2.5", "2.14"]);
    let startup = abi.libraries["c"]
        .iter()
        .find(|s| s.name == "__libc_start_main")
        .unwrap();
    assert_eq!(startup.version, "2.2.5".parse().unwrap());
}

#[test]
fn snapshot_selection_does_not_backdate_library_migrations() {
    let before = snapshot("2.31");
    let after = snapshot("2.32");
    assert!(before.libraries["pthread"]
        .iter()
        .any(|s| s.name == "pthread_sigmask"));
    assert!(!before.libraries["c"]
        .iter()
        .any(|s| s.name == "pthread_sigmask"));
    assert!(after.libraries["c"]
        .iter()
        .any(|s| s.name == "pthread_sigmask"));
    assert!(!snapshot("2.22").libraries["c"]
        .iter()
        .any(|s| s.name == "__libc_start_main" && s.version == "2.34".parse().unwrap()));
    assert!(snapshot("2.34").libraries["c"]
        .iter()
        .any(|s| s.name == "__libc_start_main" && s.version == "2.34".parse().unwrap()));
}

#[test]
fn unsupported_or_missing_releases_fail_without_fallback() {
    for version in ["2.16", "3.0", "2.22.1", "2.99"] {
        assert!(Snapshot::load(&data(), version.parse().unwrap()).is_err());
    }
    for version in ["../2.22", "2.22-suffix", "2", "2.22.3.4", "4294967296.1"] {
        assert!(version.parse::<Version>().is_err());
    }
}

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "libc-abi-test-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(path: &std::path::Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn collection_checks_tags_cleanliness_and_preserves_bytes_without_overwrite() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    let abi = source.join("sysdeps/unix/sysv/linux/x86_64/64");
    fs::create_dir_all(&abi).unwrap();
    for library in glibc::LIBRARIES {
        let name = if library == "ld" {
            "ld.abilist".into()
        } else {
            format!("lib{library}.abilist")
        };
        fs::write(abi.join(name), b"GLIBC_2.2.5 example F\n").unwrap();
    }
    fs::write(source.join("COPYING"), "fixture license\n").unwrap();
    fs::write(source.join("COPYING.LIB"), "fixture library license\n").unwrap();
    git(&source, &["init", "-q"]);
    git(&source, &["add", "."]);
    git(
        &source,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "fixture",
        ],
    );
    let output = temp.0.join("output");
    assert!(glibc::collect(&source, &output).is_err());
    assert!(!output.exists());
    git(&source, &["tag", "glibc-2.22"]);
    fs::write(source.join("untracked"), "dirty").unwrap();
    assert!(glibc::collect(&source, &output).is_err());
    fs::remove_file(source.join("untracked")).unwrap();
    let destination = glibc::collect(&source, &output).unwrap();
    assert_eq!(
        Snapshot::load(&output, "2.22".parse().unwrap())
            .unwrap()
            .libraries
            .len(),
        8
    );
    assert_eq!(
        fs::read(destination.join("sysdeps/unix/sysv/linux/x86_64/64/libc.abilist")).unwrap(),
        b"GLIBC_2.2.5 example F\n"
    );
    assert!(destination.join("SOURCE.txt").is_file());
    assert!(destination.join("COPYING.LIB").is_file());
    assert!(glibc::collect(&source, &output).is_err());
}

#[test]
fn cli_queries_real_sles_baseline_data_and_rejects_ambiguous_options() {
    let exe = env!("CARGO_BIN_EXE_libc-abi");
    let out = Command::new(exe)
        .args(["glibc", "snapshot", "--data"])
        .arg(data())
        .args(["--release", "2.22", "--symbol", "memcpy"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tsv = String::from_utf8(out.stdout).unwrap();
    assert_eq!(tsv.lines().count(), 3);
    assert!(tsv.contains("\tc\tmemcpy\tGLIBC_2.2.5\tF\t-\n"));
    for args in [
        vec!["glibc", "snapshot", "--data"],
        vec![
            "glibc",
            "snapshot",
            "--release",
            "2.22",
            "--release",
            "2.17",
        ],
        vec!["musl", "snapshot"],
        vec!["glibc", "snapshot", "--unknown", "x"],
    ] {
        assert!(!Command::new(exe)
            .args(args)
            .output()
            .unwrap()
            .status
            .success());
    }
}
