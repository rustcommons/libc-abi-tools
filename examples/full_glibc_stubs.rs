//! Experimental complete pinned numeric GLIBC ABI for one exact reference tree.
use libc_abi_tools::{
    elf,
    glibc::{self, Snapshot},
    stub,
};
use std::{env, fs, path::PathBuf};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "usage: full_glibc_stubs DATA_DIRECTORY REFERENCE_LIB64 NEW_OUTPUT_DIRECTORY".into(),
        );
    }
    let snapshot = Snapshot::load(&PathBuf::from(&args[0]), "2.17".parse()?)?;
    let root = PathBuf::from(&args[1]);
    let mut results = Vec::new();
    let mut errors = Vec::new();
    for library in glibc::LIBRARIES {
        let file = match library {
            "c" => "libc.so.6",
            "m" => "libm.so.6",
            "ld" => "ld-linux-x86-64.so.2",
            "resolv" => "libresolv.so.2",
            "pthread" => "libpthread.so.0",
            "dl" => "libdl.so.2",
            "rt" => "librt.so.1",
            _ => "libutil.so.1",
        };
        let reference = elf::Metadata::load(&root.join(file))?;
        match stub::render_glibc(&snapshot, library, &reference) {
            Ok(sources) => {
                println!(
                    "{library}: {} public definitions",
                    sources.selected.symbols.len()
                );
                results.push((library, sources));
            }
            Err(error) => errors.push(format!("{library}: {error}")),
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let output = PathBuf::from(&args[2]);
    fs::create_dir(&output).map_err(|e| e.to_string())?;
    for (library, sources) in results {
        for (suffix, text) in [
            ("S", sources.assembly),
            ("map", sources.versions),
            ("elf.tsv", sources.selected.to_tsv()),
        ] {
            fs::write(output.join(format!("{library}.{suffix}")), text)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("full_glibc_stubs: {error}");
        std::process::exit(1);
    }
}
