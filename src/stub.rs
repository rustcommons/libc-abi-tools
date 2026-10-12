//! Experimental evidence-driven x86_64 link-only assembly, separate from collection.
//! No code, initialized data, IFUNC implementations, or TLS initialization is copied.
use crate::{
    elf,
    glibc::{Kind, Snapshot},
    version::Version,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

#[derive(Debug, Eq, PartialEq)]
pub struct Sources {
    pub assembly: String,
    pub versions: String,
    pub selected: elf::Metadata,
}

/// Join an exact snapshot with all public numeric GLIBC definitions in one DSO.
/// Reject mismatched membership/kind/size; private and unversioned definitions
/// are intentionally outside the abilist contract.
pub fn render_glibc(
    snapshot: &Snapshot,
    library: &str,
    reference: &elf::Metadata,
) -> Result<Sources, String> {
    snapshot.release.validate_release()?;
    let soname = glibc_soname(library).ok_or("unsupported glibc library identity")?;
    if reference.soname != soname {
        return Err(format!("expected SONAME {soname}"));
    }
    let expected = snapshot
        .libraries
        .get(library)
        .ok_or("missing library ABI metadata")?;
    let public = elf::Metadata {
        soname: reference.soname.clone(),
        symbols: reference
            .symbols
            .iter()
            .filter(|s| {
                matches!(s.visibility, 0 | 3)
                    && s.version
                        .as_deref()
                        .and_then(|v| v.strip_prefix("GLIBC_"))
                        .is_some_and(|v| v.parse::<Version>().is_ok())
            })
            .cloned()
            .collect(),
    };
    if public.symbols.len() != expected.len() {
        return Err("ELF/snapshot public version membership differs".into());
    }
    let mut actual = BTreeMap::new();
    for s in &public.symbols {
        if actual
            .insert((s.name.as_str(), s.version.as_deref()), s)
            .is_some()
        {
            return Err("duplicate ELF public definition".into());
        }
    }
    let mut expected_keys = BTreeSet::new();
    for entry in expected {
        if !expected_keys.insert((entry.name.as_str(), entry.version)) {
            return Err("duplicate ABI definition".into());
        }
        let version = format!("GLIBC_{}", entry.version);
        let s = actual
            .get(&(entry.name.as_str(), Some(version.as_str())))
            .ok_or_else(|| format!("{}@{version}: missing ELF definition", entry.name))?;
        let matches = match entry.kind {
            Kind::Function => matches!(s.symbol_type, 2 | 10),
            Kind::Object(size) => s.symbol_type == 1 && s.size == u64::from(size),
            Kind::Tls(size) => s.symbol_type == 6 && s.size == u64::from(size),
        };
        if !matches {
            return Err(format!(
                "{}@{version}: ELF/snapshot kind or size differs",
                entry.name
            ));
        }
    }
    let names: BTreeSet<_> = expected.iter().map(|s| s.name.as_str()).collect();
    render(&public, &names.into_iter().collect::<Vec<_>>())
}

/// The SONAME of each glibc library, in [`crate::glibc::LIBRARIES`]' names.
pub fn glibc_soname(library: &str) -> Option<&'static str> {
    Some(match library {
        "c" => "libc.so.6",
        "m" => "libm.so.6",
        "ld" => "ld-linux-x86-64.so.2",
        "resolv" => "libresolv.so.2",
        "pthread" => "libpthread.so.0",
        "dl" => "libdl.so.2",
        "rt" => "librt.so.1",
        "util" => "libutil.so.1",
        _ => return None,
    })
}

/// The stub evidence of all eight libraries of a release, from a directory with
/// the real libraries (an installed `lib64`), as one [`elf::Metadata::to_tsv`]
/// table: exactly the public definitions [`render_glibc`] uses, each library
/// checked against the snapshot first. [`glibc_from_table`] renders stubs from it
/// without the libraries.
pub fn glibc_table(snapshot: &Snapshot, lib64: &std::path::Path) -> Result<String, String> {
    let mut table = String::new();
    for library in crate::glibc::LIBRARIES {
        let soname = glibc_soname(library).expect("known library");
        let reference = elf::Metadata::load(&lib64.join(soname))?;
        let text = render_glibc(snapshot, library, &reference)
            .map_err(|e| format!("{soname}: {e}"))?
            .selected
            .to_tsv();
        let rows = if table.is_empty() {
            &text[..]
        } else {
            text.split_once('\n').map_or("", |(_, rows)| rows)
        };
        table.push_str(rows);
    }
    Ok(table)
}

/// Stubs for all eight libraries from a [`glibc_table`], by library name.
pub fn glibc_from_table(
    snapshot: &Snapshot,
    table: &str,
) -> Result<Vec<(&'static str, Sources)>, String> {
    let libraries = elf::Metadata::from_tsv(table)?;
    if libraries.len() != crate::glibc::LIBRARIES.len() {
        return Err("the library table must have exactly the eight glibc libraries".into());
    }
    crate::glibc::LIBRARIES
        .iter()
        .map(|&library| {
            let soname = glibc_soname(library).expect("known library");
            let reference = libraries
                .iter()
                .find(|m| m.soname == soname)
                .ok_or_else(|| format!("the library table lacks {soname}"))?;
            let sources =
                render_glibc(snapshot, library, reference).map_err(|e| format!("{soname}: {e}"))?;
            Ok((library, sources))
        })
        .collect()
}

fn identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !name.starts_with("__abi_stub_")
        && name.len() <= 1024
}

struct DataRange {
    start: u64,
    end: u64,
    alignment: u64,
    section: u16,
    kind: u8,
}

fn data_layout(
    symbols: &[elf::Symbol],
) -> Result<(Vec<DataRange>, BTreeMap<usize, usize>), String> {
    let mut by_section = BTreeMap::<(u16, u8), Vec<usize>>::new();
    for (index, s) in symbols
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.symbol_type, 1 | 6))
    {
        let alignment = s.section_alignment.max(1);
        if alignment > 4096 || !alignment.is_power_of_two() {
            return Err("unsupported data/TLS section alignment".into());
        }
        if s.size == 0 || s.size > 16 * 1024 * 1024 {
            return Err("unsupported zero or oversized data/TLS symbol".into());
        }
        s.value
            .checked_add(s.size)
            .ok_or("ELF data range overflow")?;
        by_section
            .entry((s.section, s.symbol_type))
            .or_default()
            .push(index);
    }
    let mut groups: Vec<DataRange> = Vec::new();
    let mut membership = BTreeMap::new();
    for ((section, kind), mut indexes) in by_section {
        indexes.sort_by_key(|&index| (symbols[index].value, symbols[index].size));
        let mut active: Option<usize> = None;
        for index in indexes {
            let s = &symbols[index];
            let end = s.value + s.size; // checked above
            let group = match active {
                Some(group) if s.value < groups[group].end => {
                    groups[group].end = groups[group].end.max(end);
                    groups[group].alignment =
                        groups[group].alignment.max(s.section_alignment.max(1));
                    group
                }
                _ => {
                    let group = groups.len();
                    groups.push(DataRange {
                        start: s.value,
                        end,
                        alignment: s.section_alignment.max(1),
                        section,
                        kind,
                    });
                    active = Some(group);
                    group
                }
            };
            if groups[group].end - groups[group].start > 16 * 1024 * 1024 {
                return Err("data/TLS backing range exceeds 16 MiB".into());
            }
            membership.insert(index, group);
        }
    }
    Ok((groups, membership))
}

/// Emit every observed version of each requested name. GNU version parents are
/// not reconstructed. Matching physical ranges share storage without asserting
/// a semantic alias relationship. Overlapping data/TLS views retain relative
/// offsets in a common backing range; incompatible/code overlaps are rejected.
pub fn render(reference: &elf::Metadata, names: &[&str]) -> Result<Sources, String> {
    render_versions(reference, names, "GLIBC_", false)
}

/// [`render`] for any numeric version family, such as libgcc_s's `GCC_3.0`.
/// Each version keeps its observed spelling (`GCC_4.0.0`); GLIBC versions must
/// be canonical. With `chained`, each version node inherits the previous one,
/// as GCC's version nodes do.
pub fn render_versions(
    reference: &elf::Metadata,
    names: &[&str],
    prefix: &str,
    chained: bool,
) -> Result<Sources, String> {
    if names.is_empty() || names.len() > 10000 || reference.symbols.len() > 100000 {
        return Err("stub requires 1..10000 names and at most 100000 ELF definitions".into());
    }
    let requested: BTreeSet<_> = names.iter().copied().collect();
    if requested.len() != names.len() || requested.iter().any(|name| !identifier(name)) {
        return Err("stub names must be unique C identifiers outside the reserved prefix".into());
    }
    let mut selected = elf::Metadata {
        soname: reference.soname.clone(),
        symbols: reference
            .symbols
            .iter()
            .filter(|symbol| requested.contains(symbol.name.as_str()))
            .cloned()
            .collect(),
    };
    selected.symbols.sort();
    let observed: BTreeSet<_> = selected.symbols.iter().map(|s| s.name.as_str()).collect();
    if observed != requested {
        return Err("requested stub symbol is absent from ELF evidence".into());
    }
    let mut keys = BTreeSet::new();
    let mut defaults = BTreeSet::new();
    let mut nodes = BTreeMap::<Version, BTreeSet<String>>::new();
    let mut spellings = BTreeMap::<Version, String>::new();
    let mut assembly = String::from("# Experimental link-only library; function bodies trap.\n");
    let mut storage = BTreeMap::new();
    let mut ranges = BTreeMap::<u16, BTreeMap<u64, u64>>::new();
    let mut allocated = 0u64;
    let (data_ranges, data_membership) = data_layout(&selected.symbols)?;
    let mut emitted_data = BTreeSet::new();
    for (index, symbol) in selected.symbols.iter().enumerate() {
        if !matches!(symbol.binding, 1 | 2)
            || !matches!(symbol.visibility, 0 | 3)
            || !matches!(symbol.symbol_type, 1 | 2 | 6 | 10)
            || symbol.section == 0
            || symbol.section >= 0xff00
        {
            return Err(format!(
                "{}: unsupported binding, visibility, type, or section",
                symbol.name
            ));
        }
        let alignment = symbol.section_alignment.max(1);
        if alignment > 4096 || !alignment.is_power_of_two() {
            return Err(format!("{}: unsupported alignment", symbol.name));
        }
        let version = symbol
            .version
            .as_deref()
            .ok_or("unversioned stub definitions unsupported")?;
        let numeric: Version = version
            .strip_prefix(prefix)
            .ok_or_else(|| format!("stub requires numeric {prefix} versions"))?
            .parse()?;
        // Reject spellings that cannot be reproduced by the numeric version type.
        if prefix == "GLIBC_" && format!("GLIBC_{numeric}") != version {
            return Err("noncanonical GLIBC version".into());
        }
        if *spellings
            .entry(numeric)
            .or_insert_with(|| version.to_string())
            != version
        {
            return Err(format!("{version}: one version spelled two ways"));
        }
        let default = symbol
            .default_version
            .ok_or("missing default-version evidence")?;
        if !keys.insert((&symbol.name, version)) || (default && !defaults.insert(&symbol.name)) {
            return Err("duplicate definition or multiple default stub versions".into());
        }
        nodes
            .entry(numeric)
            .or_default()
            .insert(symbol.name.clone());
        let key = (
            symbol.section,
            symbol.value,
            symbol.symbol_type,
            symbol.size,
        );
        let emitted_size = if matches!(symbol.symbol_type, 2 | 10) {
            2
        } else {
            symbol.size
        };
        if emitted_size == 0 || emitted_size > 16 * 1024 * 1024 {
            return Err(format!(
                "{}: unsupported zero or oversized storage",
                symbol.name
            ));
        }
        let old_storage = if symbol.size == 0 {
            None
        } else {
            storage.get(&key).copied()
        };
        let location = if let Some(&group) = data_membership.get(&index) {
            let range = &data_ranges[group];
            if emitted_data.insert(group) {
                let bytes = range.end - range.start;
                let location = ranges.entry(range.section).or_default();
                let overlaps_before = location
                    .range(..=range.start)
                    .next_back()
                    .is_some_and(|(_, &end)| end > range.start);
                let overlaps_after = location
                    .range(range.start..)
                    .next()
                    .is_some_and(|(&start, _)| start < range.end);
                if overlaps_before || overlaps_after {
                    return Err("incompatible overlapping storage types".into());
                }
                location.insert(range.start, range.end);
                allocated = allocated
                    .checked_add(bytes + range.alignment - 1)
                    .ok_or("stub allocation overflow")?;
                if allocated > 64 * 1024 * 1024 {
                    return Err("stub storage budget exceeds 64 MiB".into());
                }
                let section = if range.kind == 6 {
                    ".section .tbss,\"awT\",@nobits"
                } else {
                    ".section .bss,\"aw\",@nobits"
                };
                writeln!(
                    assembly,
                    "{section}\n.balign {}\n.Ldata{group}:\n  .zero {bytes}",
                    range.alignment
                )
                .expect("String write");
            }
            format!(".Ldata{group}+{}", symbol.value - range.start)
        } else {
            let slot = if let Some(slot) = old_storage {
                slot
            } else {
                // Check original ranges, excluding identical ranges handled above.
                let end = symbol
                    .value
                    .checked_add(symbol.size)
                    .ok_or("ELF symbol range overflow")?;
                let location = ranges.entry(symbol.section).or_default();
                let overlaps_before = location
                    .range(..=symbol.value)
                    .next_back()
                    .is_some_and(|(_, &old_end)| old_end > symbol.value);
                let overlaps_after = location
                    .range(symbol.value..)
                    .next()
                    .is_some_and(|(&start, _)| start < end);
                if symbol.size != 0 && (overlaps_before || overlaps_after) {
                    return Err(format!(
                    "{}@{version}: partially overlapping symbol ranges unsupported at {}:{:#x}+{}",
                    symbol.name, symbol.section, symbol.value, symbol.size
                ));
                }
                if symbol.size != 0 {
                    location.insert(symbol.value, end);
                }
                allocated = allocated
                    .checked_add(emitted_size + alignment - 1)
                    .ok_or("stub allocation overflow")?;
                if allocated > 64 * 1024 * 1024 {
                    return Err("stub storage budget exceeds 64 MiB".into());
                }
                let slot = index;
                if symbol.size != 0 {
                    storage.insert(key, slot);
                }
                let section = match symbol.symbol_type {
                    1 => ".section .bss,\"aw\",@nobits",
                    6 => ".section .tbss,\"awT\",@nobits",
                    _ => ".text",
                };
                writeln!(assembly, "{section}\n.balign {alignment}\n.Lstorage{slot}:")
                    .expect("String write");
                if matches!(symbol.symbol_type, 2 | 10) {
                    assembly.push_str("  ud2\n");
                } else {
                    writeln!(assembly, "  .zero {emitted_size}").expect("String write");
                }
                slot
            };
            format!(".Lstorage{slot}")
        };
        let private = format!("__abi_stub_{index}");
        let binding = if symbol.binding == 2 {
            ".weak"
        } else {
            ".globl"
        };
        let kind = match symbol.symbol_type {
            1 => "object",
            6 => "tls_object",
            10 => "gnu_indirect_function",
            _ => "function",
        };
        writeln!(assembly, "{binding} {private}\n.set {private},{location}\n.type {private},@{kind}\n.size {private},{emitted_size}").expect("String write");
        if symbol.visibility == 3 {
            writeln!(assembly, ".protected {private}").expect("String write");
        }
        let separator = if default { "@@" } else { "@" };
        writeln!(
            assembly,
            ".symver {private},{}{separator}{version}",
            symbol.name
        )
        .expect("String write");
    }
    assembly.push_str(".section .note.GNU-stack,\"\",@progbits\n");
    let mut versions = String::new();
    let mut previous: Option<&str> = None;
    for (index, (version, names)) in nodes.iter().enumerate() {
        let spelled = &spellings[version];
        writeln!(versions, "{spelled} {{\n  global:").expect("String write");
        for name in names {
            writeln!(versions, "    {name};").expect("String write");
        }
        if index == 0 {
            versions.push_str("  local: *;\n");
        }
        match previous {
            Some(parent) if chained => writeln!(versions, "}} {parent};").expect("String write"),
            _ => versions.push_str("};\n"),
        }
        previous = Some(spelled);
    }
    Ok(Sources {
        assembly,
        versions,
        selected,
    })
}
