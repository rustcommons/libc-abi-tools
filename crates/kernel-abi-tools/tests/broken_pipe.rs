//! `kernel-seccomp ... | head` closes the pipe before kernel-seccomp is done
//! writing. It must stop quietly, not panic with "failed printing to stdout".

use std::process::{Command, Stdio};

#[test]
fn output_commands_stop_quietly_when_the_reader_has_gone() {
    for args in [
        &["missing", "--kernel", "3.10"][..],
        &["profile", "--kernel", "3.10"],
        &["distros"],
        &["version"],
        &["help"],
    ] {
        // A pipe whose read end is already closed: every write fails, as it
        // does once `head` has exited.
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let output = Command::new(env!("CARGO_BIN_EXE_kernel-seccomp"))
            .args(args)
            .stdout(writer)
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "{args:?}: {:?}: {stderr}",
            output.status
        );
        assert!(stderr.is_empty(), "{args:?}: {stderr}");
    }
}
