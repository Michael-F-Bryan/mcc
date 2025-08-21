use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn run_test_compiler() {
    let compiler_driver = PathBuf::from(env!("CARGO_BIN_EXE_test_driver"));
    let test_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("writing-a-c-compiler-tests");

    let tc = test_root.join("test_compiler");
    let mut cmd = command(&tc);
    cmd.arg(compiler_driver).arg("--chapter=2");
    println!("Running: {cmd:?}");

    let output = cmd.output().unwrap();

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

fn command(test_compiler: &Path) -> Command {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        // On Apple Silicon, we need to run the test compiler through rosetta to
        // get it to run our x86_64 code
        let mut cmd = Command::new("arch");
        cmd.arg("-x86_64").arg(test_compiler);
        cmd
    } else {
        Command::new(test_compiler)
    }
}
