use std::{fs, process::Command};

#[test]
fn cli_writes_the_libgcc_s_stub_sources_into_a_new_directory() {
    let output = std::env::temp_dir().join(format!("libgcc-stub-{}", std::process::id()));
    let _ = fs::remove_dir_all(&output);
    let run = |args: &[&std::ffi::OsStr]| {
        Command::new(env!("CARGO_BIN_EXE_libc-abi"))
            .args(args)
            .output()
            .unwrap()
    };
    let result = run(&[
        "libgcc".as_ref(),
        "stub".as_ref(),
        "--output".as_ref(),
        output.as_os_str(),
    ]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "libgcc_s.so.1: 138 definitions, the interface of GCC 4.8.5\n"
    );
    let map = fs::read_to_string(output.join("libgcc_s.map")).unwrap();
    assert!(map.starts_with("GCC_3.0 {"));
    assert!(fs::read_to_string(output.join("libgcc_s.S"))
        .unwrap()
        .contains("_Unwind_Resume@@GCC_3.0"));
    // The output directory must be new.
    let again = run(&[
        "libgcc".as_ref(),
        "stub".as_ref(),
        "--output".as_ref(),
        output.as_os_str(),
    ]);
    assert!(!again.status.success());
    fs::remove_dir_all(&output).unwrap();
    for bad in [
        &["libgcc"][..],
        &["libgcc", "stub"],
        &["libgcc", "list", "--output", "x"],
    ] {
        let args: Vec<&std::ffi::OsStr> = bad.iter().map(|a| a.as_ref()).collect();
        assert!(!run(&args).status.success(), "{bad:?}");
    }
}
