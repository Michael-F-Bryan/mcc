use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Context;

#[test]
fn run_test_compiler() {
    let compiler_driver = PathBuf::from(env!("CARGO_BIN_EXE_test_driver"));
    let test_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("writing-a-c-compiler-tests");

    let mut cmd = Command::new("python3");
    cmd.arg(test_root.join("test_compiler"))
        .arg(compiler_driver)
        .arg("--chapter=2");
    println!("Running: {cmd:?}");

    let output = cmd
        .output()
        .with_context(|| format!("Failed to run: {cmd:?}"))
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("---- STDOUT ----");
        println!("{stdout}");
        println!("---- END STDOUT ----");
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        println!("---- STDERR ----");
        println!("{stderr}");
        println!("---- END STDERR ----");
    }

    assert!(output.status.success());
}
