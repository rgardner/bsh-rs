//! Integration Tests

extern crate assert_cli;
extern crate chrono;
#[macro_use]
extern crate lazy_static;
extern crate tempdir;

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use assert_cli::Assert;
use chrono::{DateTime, Local};
use tempdir::TempDir;

lazy_static! {
    static ref LOG_FILE_NAME: String = {
        let local: DateTime<Local> = Local::now();
        let log_name = local.format("%F.%H-%M-%S");
        format!("test-{}.log", log_name)
    };
}

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

fn bsh_assert() -> Assert {
    Assert::cargo_binary("bsh").with_args(&["--log", &LOG_FILE_NAME])
}

#[test]
fn test_simple_echo() {
    bsh_assert()
        .with_args(&["-c", "echo foo"])
        .stdout()
        .is("foo")
        .unwrap();
}

#[test]
fn test_exit_normal_large_negative() {
    bsh_assert()
        .with_args(&["-c", "exit 85"])
        .exit_status_is(85)
        .unwrap();

    bsh_assert()
        .with_args(&["-c", "exit 500"])
        .exit_status_is(244)
        .unwrap();

    bsh_assert()
        .with_args(&["-c", "exit -500"])
        .exit_status_is(12)
        .unwrap();
}

#[test]
fn test_simple_pipeline() {
    bsh_assert()
        .with_args(&["-c", "echo needle | grep needle"])
        .stdout()
        .is("needle\n")
        .unwrap();
}

#[test]
fn test_simple_redirects() {
    let temp_dir = generate_temp_directory().unwrap();
    let command = "echo 'test needle, please ignore' >outfile; grep <outfile 'needle'";
    let expected_stdout = "test needle, please ignore\n";
    bsh_assert()
        .current_dir(temp_dir.path())
        .with_args(&["-c", command])
        .stdout()
        .is(expected_stdout)
        .unwrap();
}

#[test]
fn test_stderr_redirect() {
    let temp_dir = generate_temp_directory().unwrap();
    let command = r#"python3 -c 'import sys; print("test needle", file=sys.stderr)' 2>errfile"#;
    bsh_assert()
        .current_dir(temp_dir.path())
        .with_args(&["-c", command])
        .succeeds()
        .unwrap();

    let mut file = File::open(temp_dir.path().join("errfile")).expect("unable to open errfile");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("failed to read errfile");
    assert_eq!(contents, "test needle\n");
}

#[test]
fn test_command_not_found() {
    let args = ["-c", "foo"];
    let expected_stderr = "bsh: foo: command not found\n";
    bsh_assert()
        .with_args(&args)
        .stderr()
        .is(expected_stderr)
        .exit_status_is(127)
        .unwrap();
}

#[test]
fn test_syntax_error() {
    let args = ["-c", ";"];
    let expected_stderr = "bsh: syntax error near: ;\n";
    bsh_assert()
        .with_args(&args)
        .stderr()
        .is(expected_stderr)
        .exit_status_is(2)
        .unwrap();
}

fn generate_temp_directory() -> io::Result<TempDir> {
    // Because of limitation in `assert_cli`, temporary directory must be
    // subdirectory of directory containing Cargo.toml
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests"].iter().collect();
    TempDir::new_in(root, "temp")
}
