//! Integration Tests

extern crate assert_cli;
extern crate tempdir;

use assert_cli::Assert;
use std::io;
use std::path::PathBuf;
use tempdir::TempDir;

trait AssertExt {
    fn exit_status_is(self, exit_status: i32) -> Self;
}

impl AssertExt for Assert {
    fn exit_status_is(self, exit_status: i32) -> Self {
        if exit_status == 0 {
            self.succeeds()
        } else {
            self.fails_with(exit_status)
        }
    }
}

#[test]
fn test_simple_echo() {
    let args = ["-c", "echo foo"];
    let expected_stdout = "foo";
    Assert::cargo_binary("bsh")
        .with_args(&args)
        .stdout()
        .is(expected_stdout)
        .unwrap();
}

#[test]
fn test_exit_normal_large_negative() {
    let args = ["-c", "exit 85"];
    Assert::cargo_binary("bsh")
        .with_args(&args)
        .exit_status_is(85)
        .unwrap();

    let args = ["-c", "exit 500"];
    Assert::cargo_binary("bsh")
        .with_args(&args)
        .exit_status_is(244)
        .unwrap();

    let args = ["-c", "exit -500"];
    Assert::cargo_binary("bsh")
        .with_args(&args)
        .exit_status_is(12)
        .unwrap();
}

#[test]
fn test_simple_pipeline() {
    let args = ["-c", "echo needle | grep needle"];
    let expected_stdout = "needle\n";
    Assert::cargo_binary("bsh")
        .with_args(&args)
        .stdout()
        .is(expected_stdout)
        .unwrap();
}

#[test]
fn test_simple_redirects() {
    let temp_dir = generate_temp_directory().unwrap();
    let command = "echo 'test needle, please ignore' >outfile; grep <outfile 'needle'";
    let args = ["-c", command];
    let expected_stdout = "test needle, please ignore\n";
    Assert::cargo_binary("bsh")
        .current_dir(temp_dir.path())
        .with_args(&args)
        .stdout()
        .is(expected_stdout)
        .unwrap();
}

#[test]
#[ignore]
fn test_command_not_found() {
    let args = ["-c", "foo"];
    let expected_stderr = "bsh: foo: command not found\n";
    Assert::cargo_binary("bsh")
        .with_args(&args)
        .stderr()
        .is(expected_stderr)
        .exit_status_is(127)
        .unwrap();
}

fn generate_temp_directory() -> io::Result<TempDir> {
    // Because of limitation in `assert_cli`, temporary directory must be
    // subdirectory of directory containing Cargo.toml
    let temp_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    TempDir::new_in(temp_root, "temp")
}
