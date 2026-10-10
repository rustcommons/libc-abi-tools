//! Experimental selected exports, not a complete link environment.
use libc_abi_tools::{
    elf,
    glibc::{Kind, Snapshot},
    stub,
};
use std::{env, fs, path::PathBuf};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "usage: expanded_glibc_stub DATA_DIRECTORY REFERENCE_LIBC NEW_OUTPUT_DIRECTORY".into(),
        );
    }
    let names = [
        "__libc_start_main",
        "puts",
        "memcpy",
        "stdout",
        "fwrite",
        "fflush",
    ];
    let snapshot = Snapshot::load(&PathBuf::from(&args[0]), "2.17".parse()?)?;
    let reference = elf::Metadata::load(&PathBuf::from(&args[1]))?;
    if reference.soname != "libc.so.6" {
        return Err("expected libc.so.6 evidence".into());
    }
    let sources = stub::render(&reference, &names)?;
    let abi = snapshot.libraries.get("c").ok_or("missing libc ABI")?;
    for name in names {
        let expected: Vec<_> = abi.iter().filter(|s| s.name == name).collect();
        let actual: Vec<_> = sources
            .selected
            .symbols
            .iter()
            .filter(|s| s.name == name)
            .collect();
        if actual.len() != expected.len() {
            return Err(format!("{name}: ELF/2.17 version membership differs"));
        }
        for entry in expected {
            let version = format!("GLIBC_{}", entry.version);
            let symbol = actual
                .iter()
                .find(|s| s.version.as_deref() == Some(&version))
                .ok_or("missing ABI version in ELF")?;
            let matches = match entry.kind {
                Kind::Function => matches!(symbol.symbol_type, 2 | 10),
                Kind::Object(size) => symbol.symbol_type == 1 && symbol.size == u64::from(size),
                Kind::Tls(size) => symbol.symbol_type == 6 && symbol.size == u64::from(size),
            };
            if !matches {
                return Err(format!("{name}: ELF/2.17 kind or size differs"));
            }
        }
    }
    let output = PathBuf::from(&args[2]);
    fs::create_dir(&output).map_err(|e| e.to_string())?;
    for (name, text) in [
        ("libc.S", sources.assembly),
        ("libc.map", sources.versions),
        ("selected-elf.tsv", sources.selected.to_tsv()),
    ] {
        fs::write(output.join(name), text).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("expanded_glibc_stub: {error}");
        std::process::exit(1);
    }
}
