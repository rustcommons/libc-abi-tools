//! The compiler runtime `libgcc_s.so.1` as a link-only stub.
//!
//! Providers link every release against the interface of GCC 4.8.5's
//! `libgcc_s.so.1` (CentOS 7), recorded in `libgcc_s-gcc-4.8.5.tsv` by
//! `scripts/record-libgcc-s.sh`. An executable linked against the stub imports
//! only symbol versions that runtime defines (`GCC_3.0` through `GCC_4.8.0`), so
//! it loads with GCC 4.8.5's runtime and every later one. The stub has no code:
//! function bodies trap, and it must never be loaded at run time.
use crate::{elf, stub, version::Version};
use std::collections::BTreeSet;

/// The compiler runtime's SONAME.
pub const SONAME: &str = "libgcc_s.so.1";
/// GCC release whose `libgcc_s.so.1` interface the stub reproduces.
pub const GCC_RELEASE: &str = "4.8.5";
/// The recorded interface: symbol, version, default, type, size, alignment.
pub const INTERFACE: &str = include_str!("libgcc_s-gcc-4.8.5.tsv");
const HEADER: &str = "symbol\tversion\tdefault\ttype\tsize\talignment";

/// One symbol definition of the recorded `libgcc_s.so.1` interface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    /// The symbol name.
    pub name: String,
    /// Its version node, such as `GCC_3.0`.
    pub version: String,
    /// False for a compatibility (non-default) version, which links can no longer select.
    pub default: bool,
    /// True for a data object, false for a function.
    pub object: bool,
    /// The object's size in bytes; zero for functions.
    pub size: u64,
    /// Alignment of an object's definition; one for functions.
    pub alignment: u64,
}

/// Parses and validates the recorded interface.
pub fn definitions() -> Result<Vec<Definition>, String> {
    parse(INTERFACE)
}

fn parse(text: &str) -> Result<Vec<Definition>, String> {
    let mut lines = text.lines().filter(|l| !l.starts_with('#'));
    if lines.next() != Some(HEADER) {
        return Err("libgcc_s interface: unexpected header".into());
    }
    let mut result = Vec::new();
    let mut keys = BTreeSet::new();
    let mut defaults = BTreeSet::new();
    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        let [name, version, default, kind, size, alignment] = fields[..] else {
            return Err(format!("libgcc_s interface: malformed row {line:?}"));
        };
        let numeric = version
            .strip_prefix("GCC_")
            .ok_or_else(|| format!("{name}: version {version} is not a GCC version"))?;
        numeric.parse::<Version>()?;
        let default = match default {
            "yes" => true,
            "no" => false,
            _ => return Err(format!("{name}@{version}: default must be yes or no")),
        };
        let object = match kind {
            "FUNC" => false,
            "OBJECT" => true,
            _ => return Err(format!("{name}@{version}: unsupported type {kind}")),
        };
        let size: u64 = size
            .parse()
            .map_err(|_| format!("{name}@{version}: invalid size"))?;
        let alignment = match (object, alignment) {
            (false, "-") => 1,
            (true, text) => text
                .parse::<u64>()
                .ok()
                .filter(|a| a.is_power_of_two() && *a <= 4096)
                .ok_or_else(|| format!("{name}@{version}: invalid alignment"))?,
            _ => return Err(format!("{name}@{version}: alignment is for objects only")),
        };
        if object && size == 0 {
            return Err(format!("{name}@{version}: empty object"));
        }
        if !keys.insert((name.to_string(), version.to_string()))
            || (default && !defaults.insert(name.to_string()))
        {
            return Err(format!("{name}@{version}: duplicate definition or default"));
        }
        result.push(Definition {
            name: name.into(),
            version: version.into(),
            default,
            object,
            size,
            alignment,
        });
    }
    if result.is_empty() {
        return Err("libgcc_s interface is empty".into());
    }
    Ok(result)
}

/// The version nodes, oldest first; each inherits the one before it.
pub fn versions() -> Result<Vec<String>, String> {
    let mut versions: Vec<(Version, String)> = definitions()?
        .into_iter()
        .map(|d| {
            (
                d.version["GCC_".len()..].parse().expect("validated"),
                d.version,
            )
        })
        .collect();
    versions.sort();
    versions.dedup();
    Ok(versions.into_iter().map(|(_, v)| v).collect())
}

/// Whether GCC 4.8.5's runtime defines `name@version`, so that an executable
/// importing it loads there.
pub fn provides(name: &str, version: &str) -> Result<bool, String> {
    Ok(definitions()?
        .iter()
        .any(|d| d.name == name && d.version == version))
}

/// Assembly and version script for the stub `libgcc_s.so.1`.
pub fn stub() -> Result<stub::Sources, String> {
    let definitions = definitions()?;
    // Give every definition its own storage: functions in one section, objects in
    // another, each at an offset aligned for it.
    let mut ends = [0u64; 2];
    let mut symbols = Vec::new();
    for d in &definitions {
        let (section, alignment) = if d.object { (2, d.alignment) } else { (1, 16) };
        let end = &mut ends[usize::from(d.object)];
        let value = end.next_multiple_of(alignment);
        *end = value + d.size.max(1);
        symbols.push(elf::Symbol {
            name: d.name.clone(),
            version: Some(d.version.clone()),
            default_version: Some(d.default),
            symbol_type: if d.object { 1 } else { 2 },
            binding: 1,
            visibility: 0,
            section,
            section_alignment: alignment,
            value,
            size: d.size,
            same_range: None,
        });
    }
    let reference = elf::Metadata {
        soname: SONAME.into(),
        symbols,
    };
    let names: BTreeSet<&str> = definitions.iter().map(|d| d.name.as_str()).collect();
    stub::render_versions(
        &reference,
        &names.into_iter().collect::<Vec<_>>(),
        "GCC_",
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_interface_is_gcc_4_8_5() {
        let definitions = definitions().unwrap();
        assert_eq!(definitions.len(), 138);
        assert_eq!(definitions.iter().filter(|d| !d.default).count(), 6);
        assert_eq!(
            versions().unwrap(),
            [
                "GCC_3.0",
                "GCC_3.3",
                "GCC_3.3.1",
                "GCC_3.4",
                "GCC_3.4.2",
                "GCC_3.4.4",
                "GCC_4.0.0",
                "GCC_4.2.0",
                "GCC_4.3.0",
                "GCC_4.7.0",
                "GCC_4.8.0"
            ]
        );
        let objects: Vec<_> = definitions.iter().filter(|d| d.object).collect();
        assert_eq!(objects.len(), 1);
        assert_eq!(
            (
                objects[0].name.as_str(),
                objects[0].size,
                objects[0].alignment
            ),
            ("__cpu_model", 16, 16)
        );
        assert!(provides("_Unwind_Resume", "GCC_3.0").unwrap());
        assert!(provides("__gttf2", "GCC_3.0").unwrap());
        // Added by later GCC releases.
        assert!(!provides("__divmodti4", "GCC_7.0.0").unwrap());
        assert!(!provides("_Unwind_Resume", "GCC_7.0.0").unwrap());
    }

    #[test]
    fn stub_defines_each_version_once_in_a_chain() {
        let sources = stub().unwrap();
        assert_eq!(sources.selected.symbols.len(), 138);
        assert!(sources.versions.starts_with("GCC_3.0 {\n  global:\n"));
        assert!(sources.versions.ends_with("} GCC_4.7.0;\n"));
        for (child, parent) in [("GCC_3.3", "GCC_3.0"), ("GCC_4.8.0", "GCC_4.7.0")] {
            let node = &sources.versions[sources.versions.find(&format!("{child} {{")).unwrap()..];
            let close = &node[node.find('}').unwrap()..];
            assert!(close.starts_with(&format!("}} {parent};")), "{child}");
        }
        assert!(sources
            .assembly
            .contains(".symver __abi_stub_0,_Unwind_Backtrace@@GCC_3.3"));
        assert!(sources.assembly.contains("__gttf2@GCC_3.0\n"));
        assert!(sources.assembly.contains("__gttf2@@GCC_4.3.0\n"));
        assert!(sources.assembly.contains(".type __abi_stub_"));
        assert!(sources.assembly.contains(",@object\n.size __abi_stub_"));
        assert!(!sources.assembly.contains("GLIBC_"));
    }

    #[test]
    fn rejects_malformed_interfaces() {
        let good = format!("{HEADER}\nf\tGCC_3.0\tyes\tFUNC\t8\t-\n");
        assert!(parse(&good).is_ok());
        for bad in [
            good.replace("GCC_3.0", "GLIBC_2.2.5"),
            good.replace("GCC_3.0", "GCC_x"),
            good.replace("\tyes", "\tmaybe"),
            good.replace("FUNC", "TLS"),
            good.replace("\t-\n", "\t8\n"),
            good.replace("FUNC\t8\t-", "OBJECT\t8\t3"),
            format!("{good}f\tGCC_3.3\tyes\tFUNC\t8\t-\n"),
            format!("{good}f\tGCC_3.0\tno\tFUNC\t8\t-\n"),
            good.replace(HEADER, "symbol\tversion"),
            HEADER.to_string(),
        ] {
            assert!(parse(&bad).is_err(), "{bad}");
        }
    }
}
