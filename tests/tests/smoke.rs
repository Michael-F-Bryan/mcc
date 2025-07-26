use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn run_test_compiler() {
    let compiler_driver = PathBuf::from(env!("CARGO_BIN_EXE_test_driver"));
    let test_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("writing-a-c-compiler-tests");

    let mut cmd = Command::new(test_root.join("test_compiler"));
    cmd.arg(compiler_driver).arg("--chapter=1");
    println!("Running: {:?}", cmd);

    let output = cmd.output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        println!("---- STDOUT ----");
        println!("{}", stdout);
        println!("---- END STDOUT ----");
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        println!("---- STDERR ----");
        println!("{}", stderr);
        println!("---- END STDERR ----");
    }

    assert!(output.status.success());
}
