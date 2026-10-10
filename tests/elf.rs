use libc_abi_tools::elf::Metadata;
use std::process::Command;

const FIXTURE: &[u8] = include_bytes!("fixtures/elf/symbols.so");

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
fn section(kind: u32) -> usize {
    let table = u64_at(FIXTURE, 40) as usize;
    (0..usize::from(u16_at(FIXTURE, 60)))
        .map(|i| table + i * 64)
        .find(|&i| u32_at(FIXTURE, i + 4) == kind)
        .unwrap()
}

fn symbol_index(name: &str) -> usize {
    let dynsym = section(11);
    let symbols = u64_at(FIXTURE, dynsym + 24) as usize;
    let count = u64_at(FIXTURE, dynsym + 32) as usize / 24;
    let table = u64_at(FIXTURE, 40) as usize;
    let strings = table + u32_at(FIXTURE, dynsym + 40) as usize * 64;
    let strings_offset = u64_at(FIXTURE, strings + 24) as usize;
    (1..count)
        .find(|&index| {
            let offset = strings_offset + u32_at(FIXTURE, symbols + index * 24) as usize;
            let end = FIXTURE[offset..]
                .iter()
                .position(|&byte| byte == 0)
                .unwrap();
            &FIXTURE[offset..offset + end] == name.as_bytes()
        })
        .unwrap()
}

#[test]
fn reads_independent_version_binding_ifunc_tls_and_range_evidence() {
    let elf = Metadata::parse(FIXTURE).unwrap();
    assert_eq!(elf.soname, "libfixture.so.1");
    let foo: Vec<_> = elf.symbols.iter().filter(|s| s.name == "foo").collect();
    assert_eq!(foo.len(), 2);
    assert!(foo
        .iter()
        .any(|s| s.version.as_deref() == Some("GLIBC_2.2.5") && s.default_version == Some(false)));
    let current = foo
        .iter()
        .find(|s| s.default_version == Some(true))
        .unwrap();
    assert_eq!(current.version.as_deref(), Some("GLIBC_2.14"));
    let weak = elf
        .symbols
        .iter()
        .find(|s| s.name == "weak_function")
        .unwrap();
    assert_eq!((weak.binding, weak.visibility), (2, 3));
    assert!(current.same_range.is_some());
    assert_eq!(current.same_range, weak.same_range);
    let tls = elf.symbols.iter().find(|s| s.name == "tls_data").unwrap();
    assert_eq!((tls.symbol_type, tls.size), (6, 4));
    let object = elf
        .symbols
        .iter()
        .find(|s| s.name == "shared_data")
        .unwrap();
    assert_eq!((object.symbol_type, object.size), (1, 8));
    assert_eq!(
        elf.symbols
            .iter()
            .find(|s| s.name == "dispatch")
            .unwrap()
            .symbol_type,
        10
    );
    assert!(!elf
        .symbols
        .iter()
        .any(|s| s.name == "imported_symbol" || s.name.starts_with("GLIBC_")));
    assert_eq!(elf.to_tsv(), Metadata::parse(FIXTURE).unwrap().to_tsv());
}

#[test]
fn rejects_truncation_at_every_byte_without_panicking() {
    for end in 0..FIXTURE.len() {
        assert!(
            Metadata::parse(&FIXTURE[..end]).is_err(),
            "accepted truncation at {end}"
        );
    }
}

#[test]
fn rejects_corrupt_layout_links_and_versions() {
    let dynamic_symbols = section(11);
    let version_symbols = section(0x6fff_ffff);
    let definitions = section(0x6fff_fffd);
    let definitions_offset = u64_at(FIXTURE, definitions + 24) as usize;
    let symbols_offset = u64_at(FIXTURE, dynamic_symbols + 24) as usize;
    let versions_offset = u64_at(FIXTURE, version_symbols + 24) as usize;
    let object_index = symbol_index("shared_data");
    let object_section = u16_at(FIXTURE, symbols_offset + object_index * 24 + 6);
    let object_header = u64_at(FIXTURE, 40) as usize + usize::from(object_section) * 64;
    let changes: Vec<(usize, Vec<u8>)> = vec![
        (4, vec![1]),
        (5, vec![2]),
        (18, 183u16.to_le_bytes().to_vec()),
        (40, u64::MAX.to_le_bytes().to_vec()),
        (60, 0u16.to_le_bytes().to_vec()),
        (dynamic_symbols + 56, 23u64.to_le_bytes().to_vec()),
        (dynamic_symbols + 40, u32::MAX.to_le_bytes().to_vec()),
        (version_symbols + 32, 2u64.to_le_bytes().to_vec()),
        (definitions_offset, 2u16.to_le_bytes().to_vec()),
        (definitions_offset + 12, u32::MAX.to_le_bytes().to_vec()),
        (definitions_offset + 16, 1u32.to_le_bytes().to_vec()),
        (
            versions_offset + object_index * 2,
            0x7fffu16.to_le_bytes().to_vec(),
        ),
        (
            symbols_offset + object_index * 24,
            u32::MAX.to_le_bytes().to_vec(),
        ),
        (symbols_offset + object_index * 24 + 5, vec![0x80]),
        (object_header + 48, 3u64.to_le_bytes().to_vec()),
    ];
    for (offset, replacement) in changes {
        let mut bytes = FIXTURE.to_vec();
        bytes[offset..offset + replacement.len()].copy_from_slice(&replacement);
        assert!(
            Metadata::parse(&bytes).is_err(),
            "accepted corruption at {offset}"
        );
    }
}

#[test]
fn preserves_64_bit_symbol_sizes_without_reading_symbol_storage() {
    let mut bytes = FIXTURE.to_vec();
    let offset = u64_at(FIXTURE, section(11) + 24) as usize + symbol_index("shared_data") * 24 + 16;
    let size = u64::from(u32::MAX) + 1;
    bytes[offset..offset + 8].copy_from_slice(&size.to_le_bytes());
    assert_eq!(
        Metadata::parse(&bytes)
            .unwrap()
            .symbols
            .iter()
            .find(|s| s.name == "shared_data")
            .unwrap()
            .size,
        size
    );
}

#[test]
fn cli_collects_fixture_and_reports_invalid_input() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/elf/symbols.so");
    let output = Command::new(env!("CARGO_BIN_EXE_libc-abi"))
        .args(["glibc", "elf", "--input"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("libfixture.so.1\tfoo\tGLIBC_2.14\tyes\tFUNC"));
    let bad = Command::new(env!("CARGO_BIN_EXE_libc-abi"))
        .args(["glibc", "elf", "--input"])
        .arg(path.with_extension("S"))
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(String::from_utf8(bad.stderr)
        .unwrap()
        .contains("expected ELF64"));
}
