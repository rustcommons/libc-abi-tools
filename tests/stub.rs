use libc_abi_tools::{elf, stub};

fn fixture() -> elf::Metadata {
    elf::Metadata::parse(include_bytes!("fixtures/elf/symbols.so")).unwrap()
}

#[test]
fn renders_all_versions_and_preserves_storage_properties_deterministically() {
    let reference = fixture();
    let names = [
        "foo",
        "dispatch",
        "weak_function",
        "shared_data",
        "weak_data",
        "tls_data",
    ];
    let sources = stub::render(&reference, &names).unwrap();
    assert_eq!(sources, stub::render(&reference, &names).unwrap());
    assert_eq!(sources.selected.symbols.len(), 7);
    assert!(sources.assembly.contains("foo@GLIBC_2.2.5"));
    assert!(sources.assembly.contains("foo@@GLIBC_2.14"));
    assert!(sources.assembly.contains("@gnu_indirect_function"));
    assert!(sources.assembly.contains(".protected"));
    assert!(sources.assembly.contains(".tbss"));
    assert!(sources.assembly.contains(".zero 4"));
    assert!(sources.assembly.contains(".zero 8"));
}

#[test]
fn rejects_unsafe_names_missing_evidence_and_unsupported_properties() {
    let reference = fixture();
    for names in [
        vec![],
        vec!["foo", "foo"],
        vec!["absent"],
        vec!["bad;ret"],
        vec!["__abi_stub_0"],
    ] {
        assert!(stub::render(&reference, &names).is_err());
    }
    for change in 0..7 {
        let mut reference = fixture();
        let symbol = reference
            .symbols
            .iter_mut()
            .find(|s| s.name == "tls_data")
            .unwrap();
        match change {
            0 => symbol.default_version = None,
            1 => symbol.visibility = 2,
            2 => symbol.binding = 10,
            3 => symbol.size = 0,
            4 => symbol.size = 17 * 1024 * 1024,
            5 => symbol.section_alignment = 3,
            _ => symbol.version = Some("GLIBC_PRIVATE".into()),
        }
        assert!(stub::render(&reference, &["tls_data"]).is_err());
    }
}

#[test]
fn rejects_duplicate_defaults_and_preserves_data_views() {
    let mut reference = fixture();
    for symbol in reference.symbols.iter_mut().filter(|s| s.name == "foo") {
        symbol.default_version = Some(true);
    }
    assert!(stub::render(&reference, &["foo"]).is_err());
    let mut reference = fixture();
    let shared = reference
        .symbols
        .iter()
        .find(|s| s.name == "shared_data")
        .unwrap()
        .clone();
    let other = reference
        .symbols
        .iter_mut()
        .find(|s| s.name == "weak_data")
        .unwrap();
    other.value = shared.value + 1;
    let sources = stub::render(&reference, &["shared_data", "weak_data"]).unwrap();
    assert!(sources.assembly.contains(".zero 9"));
    assert!(sources.assembly.contains(".Ldata0+1"));
}

#[test]
fn rejects_unbounded_alignment_in_a_shared_backing_range() {
    let mut reference = fixture();
    reference
        .symbols
        .iter_mut()
        .find(|s| s.name == "weak_data")
        .unwrap()
        .section_alignment = u64::MAX;
    assert!(stub::render(&reference, &["shared_data", "weak_data"]).is_err());
}

#[test]
fn glibc_join_checks_membership_identity_and_sizes() {
    use libc_abi_tools::glibc::{parse_abilist, Snapshot};
    let abi = parse_abilist("GLIBC_2.2.5 foo F\nGLIBC_2.14 foo F\nGLIBC_2.2.5 dispatch F\nGLIBC_2.2.5 weak_function F\nGLIBC_2.2.5 shared_data D 8\nGLIBC_2.2.5 weak_data D 8\nGLIBC_2.2.5 tls_data T 4\n").unwrap();
    let snapshot = Snapshot {
        release: "2.17".parse().unwrap(),
        libraries: [("c".into(), abi)].into(),
    };
    let mut reference = fixture();
    reference.soname = "libc.so.6".into();
    assert_eq!(
        stub::render_glibc(&snapshot, "c", &reference)
            .unwrap()
            .selected
            .symbols
            .len(),
        7
    );
    assert!(stub::render_glibc(&snapshot, "m", &reference).is_err());
    let mut duplicate = snapshot.clone();
    let entries = duplicate.libraries.get_mut("c").unwrap();
    entries[0] = entries[1].clone();
    assert!(stub::render_glibc(&duplicate, "c", &reference).is_err());
    let mut wrong_size = reference.clone();
    wrong_size
        .symbols
        .iter_mut()
        .find(|s| s.name == "shared_data")
        .unwrap()
        .size = 9;
    assert!(stub::render_glibc(&snapshot, "c", &wrong_size).is_err());
    let mut missing = reference.clone();
    missing.symbols.retain(|s| s.name != "tls_data");
    assert!(stub::render_glibc(&snapshot, "c", &missing).is_err());
    let mut private = reference.symbols[0].clone();
    private.name = "private_helper".into();
    private.version = Some("GLIBC_PRIVATE".into());
    reference.symbols.push(private);
    assert_eq!(
        stub::render_glibc(&snapshot, "c", &reference)
            .unwrap()
            .selected
            .symbols
            .len(),
        7
    );
}

/// Every release's recorded library table (glibc-provider-sources) renders all
/// eight stubs against that release's ABI lists, as source mode does.
#[cfg(all(feature = "bundled-glibc-data", feature = "provider-sources"))]
#[test]
fn recorded_library_tables_render_every_release() {
    let sources = glibc_provider_sources::Sources::bundled().unwrap();
    for &release in libc_abi_tools::provider::RELEASES {
        let snapshot = libc_abi_tools::glibc::Snapshot::bundled(release.parse().unwrap()).unwrap();
        let table = sources
            .release(release)
            .unwrap()
            .table("libraries.tsv")
            .unwrap();
        let stubs = libc_abi_tools::stub::glibc_from_table(&snapshot, table).unwrap();
        assert_eq!(stubs.len(), 8, "{release}");
        let definitions: usize = stubs.iter().map(|(_, s)| s.selected.symbols.len()).sum();
        assert_eq!(definitions + 1, table.lines().count(), "{release}");
    }
}
