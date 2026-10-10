//! Experimental two-function link-only libc, not a general stub generator.
use libc_abi_tools::{elf, glibc::Kind, glibc::Snapshot, version::Version};
use std::{env, fs, path::PathBuf};

const SYMBOLS: [&str; 2] = ["__libc_start_main", "puts"];

fn render(snapshot: &Snapshot, evidence: &elf::Metadata) -> Result<(String, String), String> {
    if snapshot.release != "2.17".parse::<Version>()? {
        return Err("this experiment supports only the exact glibc 2.17 snapshot".into());
    }
    let libc = snapshot.libraries.get("c").ok_or("missing libc metadata")?;
    let version: Version = "2.2.5".parse()?;
    if evidence.soname != "libc.so.6" {
        return Err("expected ELF evidence from libc.so.6".into());
    }
    let mut bindings = std::collections::BTreeMap::new();
    for name in SYMBOLS {
        let entries: Vec<_> = libc.iter().filter(|entry| entry.name == name).collect();
        if entries.len() != 1 || entries[0].kind != Kind::Function || entries[0].version != version
        {
            return Err(format!("{name}: expected exactly one GLIBC_2.2.5 function"));
        }
        let definitions: Vec<_> = evidence
            .symbols
            .iter()
            .filter(|entry| entry.name == name)
            .collect();
        if definitions.len() != 1 {
            return Err(format!(
                "{name}: expected one ELF definition for this experiment"
            ));
        }
        let definition = definitions[0];
        if definition.version.as_deref() != Some("GLIBC_2.2.5")
            || definition.default_version != Some(true)
            || definition.symbol_type != 2
            || definition.visibility != 0
            || !matches!(definition.binding, 1 | 2)
        {
            return Err(format!(
                "{name}: unsupported ELF evidence for this experiment"
            ));
        }
        bindings.insert(
            name,
            if definition.binding == 2 {
                ".weak"
            } else {
                ".globl"
            },
        );
    }
    // Default versions, binding, and visibility for these two exports are
    // checked against real ELF files by the integration experiment.
    let mut assembly =
        String::from("# Link-only experiment: these functions trap if executed.\n.text\n");
    let mut versions = String::from("GLIBC_2.2.5 {\n  global:\n");
    for name in SYMBOLS {
        let binding = bindings.get(name).ok_or("missing validated ELF binding")?;
        assembly.push_str(&format!(
            "{binding} {name}\n.type {name}, @function\n{name}:\n  ud2\n.size {name}, .-{name}\n"
        ));
        versions.push_str(&format!("    {name};\n"));
    }
    assembly.push_str(".section .note.GNU-stack,\"\",@progbits\n");
    versions.push_str("  local: *;\n};\n");
    Ok((assembly, versions))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "usage: minimal_glibc_stub DATA_DIRECTORY REFERENCE_LIBC NEW_OUTPUT_DIRECTORY".into(),
        );
    }
    let snapshot = Snapshot::load(&PathBuf::from(&args[0]), "2.17".parse()?)?;
    let mut evidence = elf::Metadata::load(&PathBuf::from(&args[1]))?;
    let (assembly, versions) = render(&snapshot, &evidence)?;
    let output = PathBuf::from(&args[2]);
    fs::create_dir(&output).map_err(|error| format!("{}: {error}", output.display()))?;
    for (name, text) in [("libc.S", assembly), ("libc.map", versions)] {
        fs::write(output.join(name), text).map_err(|error| error.to_string())?;
    }
    let mut selected = snapshot;
    selected.libraries.retain(|name, _| name == "c");
    for symbols in selected.libraries.values_mut() {
        symbols.retain(|entry| SYMBOLS.contains(&entry.name.as_str()));
    }
    fs::write(output.join("selected-abi.tsv"), selected.to_tsv(None))
        .map_err(|error| error.to_string())?;
    evidence
        .symbols
        .retain(|entry| SYMBOLS.contains(&entry.name.as_str()));
    fs::write(output.join("selected-elf.tsv"), evidence.to_tsv())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("minimal_glibc_stub: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot::load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/glibc"),
            "2.17".parse().unwrap(),
        )
        .unwrap()
    }

    fn evidence() -> elf::Metadata {
        elf::Metadata {
            soname: "libc.so.6".into(),
            symbols: SYMBOLS
                .iter()
                .map(|name| elf::Symbol {
                    name: (*name).into(),
                    version: Some("GLIBC_2.2.5".into()),
                    default_version: Some(true),
                    symbol_type: 2,
                    binding: 1,
                    visibility: 0,
                    section: 1,
                    section_alignment: 1,
                    value: 0,
                    size: 1,
                    same_range: None,
                })
                .collect(),
        }
    }

    #[test]
    fn rejects_missing_or_ambiguous_exports() {
        let mut missing = snapshot();
        missing
            .libraries
            .get_mut("c")
            .unwrap()
            .retain(|entry| entry.name != "puts");
        assert!(render(&missing, &evidence()).is_err());
        let mut ambiguous = snapshot();
        let entries = ambiguous.libraries.get_mut("c").unwrap();
        let mut other = entries
            .iter()
            .find(|entry| entry.name == "puts")
            .unwrap()
            .clone();
        other.version = "2.17".parse().unwrap();
        entries.push(other);
        assert!(render(&ambiguous, &evidence()).is_err());
    }

    #[test]
    fn rejects_other_releases_and_symbol_kinds() {
        let mut wrong_release = snapshot();
        wrong_release.release = "2.22".parse().unwrap();
        assert!(render(&wrong_release, &evidence()).is_err());
        let mut wrong_kind = snapshot();
        wrong_kind
            .libraries
            .get_mut("c")
            .unwrap()
            .iter_mut()
            .find(|entry| entry.name == "puts")
            .unwrap()
            .kind = Kind::Object(8);
        assert!(render(&wrong_kind, &evidence()).is_err());
    }

    #[test]
    fn rejects_missing_nondefault_or_unsupported_elf_evidence() {
        let snapshot = snapshot();
        let mut missing = evidence();
        missing.symbols.clear();
        assert!(render(&snapshot, &missing).is_err());
        let mut nondefault = evidence();
        nondefault.symbols[0].default_version = Some(false);
        assert!(render(&snapshot, &nondefault).is_err());
        let mut ifunc = evidence();
        ifunc.symbols[0].symbol_type = 10;
        assert!(render(&snapshot, &ifunc).is_err());
        let mut protected = evidence();
        protected.symbols[0].visibility = 3;
        assert!(render(&snapshot, &protected).is_err());
        let mut wrong_library = evidence();
        wrong_library.soname = "libm.so.6".into();
        assert!(render(&snapshot, &wrong_library).is_err());
    }
}
