//! Exercise the expanded renderer against original multi-version/TLS ELF input.
use libc_abi_tools::{elf, stub};
use std::{env, fs, path::PathBuf};
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: fixture_stub REFERENCE_ELF NEW_OUTPUT_DIRECTORY".into());
    }
    let reference = elf::Metadata::load(&PathBuf::from(&args[0]))?;
    let sources = stub::render(
        &reference,
        &[
            "foo",
            "dispatch",
            "weak_function",
            "shared_data",
            "weak_data",
            "tls_data",
        ],
    )?;
    let output = PathBuf::from(&args[1]);
    fs::create_dir(&output).map_err(|e| e.to_string())?;
    for (name, text) in [
        ("fixture.S", sources.assembly),
        ("fixture.map", sources.versions),
        ("selected-elf.tsv", sources.selected.to_tsv()),
    ] {
        fs::write(output.join(name), text).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("fixture_stub: {error}");
        std::process::exit(1);
    }
}
