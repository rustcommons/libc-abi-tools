//! Export a trusted local descriptor only after validating all eight stub DSOs.
use libc_abi_tools::{
    elf,
    glibc::Snapshot,
    provider::{Provider, FORMAT, SONAMES},
    stub,
};
use std::{env, fs, path::PathBuf};
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 6 {
        return Err(
            "usage: provider_2_17 DATA SYSROOT STUBS GCC_STARTUP COMPILER_RUNTIME NEW_DESCRIPTOR"
                .into(),
        );
    }
    let roots: Vec<_> = args[1..5]
        .iter()
        .map(|p| fs::canonicalize(p).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let mut text = format!("{FORMAT}\ntarget\tx86_64-unknown-linux-gnu\nglibc\t2.17\n");
    for (key, path) in ["sysroot", "stubs", "gcc-startup", "compiler-runtime"]
        .iter()
        .zip(&roots)
    {
        text.push_str(&format!(
            "{key}\t{}\n",
            path.to_str().ok_or("provider paths must be UTF-8")?
        ));
    }
    let provider = Provider::parse(&text)?;
    provider.validate_files()?;
    let snapshot = Snapshot::load(&PathBuf::from(&args[0]), "2.17".parse()?)?;
    for (library, soname) in ["c", "m", "ld", "resolv", "pthread", "dl", "rt", "util"]
        .iter()
        .zip(SONAMES)
    {
        let metadata = elf::Metadata::load(&provider.stubs.join(soname))?;
        stub::render_glibc(&snapshot, library, &metadata)?;
    }
    use std::io::Write;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[5])
        .and_then(|mut f| f.write_all(text.as_bytes()))
        .map_err(|e| e.to_string())?;
    println!(
        "Exported experimental glibc 2.17 provider: {}",
        PathBuf::from(&args[5]).display()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("provider_2_17: {e}");
        std::process::exit(1);
    }
}
