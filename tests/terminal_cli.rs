#![cfg(unix)]
#[test]
fn service_cli_errors_preserve_pipe_and_real_terminal_behavior() {
    let output = std::process::Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/dependencies/terminal.py"))
        .args([env!("CARGO_BIN_EXE_hbbs"), env!("CARGO_BIN_EXE_hbbr")])
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
