//! Integration Tests

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use assert_cmd::prelude::*;
use chrono::{DateTime, Local};
use lazy_static::lazy_static;
use predicates::prelude::*;
use tempfile::TempDir;

lazy_static! {
    static ref LOG_FILE_NAME: PathBuf = {
        let local: DateTime<Local> = Local::now();
        let log_name = local.format("%F.%H-%M-%S");
        [
            env!("CARGO_MANIFEST_DIR"),
            "log",
            &format!("test-{}.log", log_name),
        ]
            .iter()
            .collect()
    };
    static ref BIN_UNDER_TEST: escargot::CargoRun = escargot::CargoBuild::new()
        .bin("bsh")
        .run()
        .expect("failed to build `cargo run` command");
}

#[test]
fn test_simple_echo() {
    BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&["-c", "echo foo"])
        .unwrap()
        .assert()
        .stdout(predicates::str::similar("foo\n").from_utf8());
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_logical_or_pipeline() {
    BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&["-c", "echo 1 || echo 2"])
        .unwrap()
        .assert()
        .stdout(predicates::str::similar("1\n").from_utf8());
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_logical_and_pipeline() {
    BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&["-c", "echo 1 && echo 2"])
        .unwrap()
        .assert()
        .stdout(predicates::str::similar("1\n2\n").from_utf8());
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_exit_normal_large_negative() {
    let err = BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&["-c", "exit 85"])
        .unwrap_err();
    let output = err.as_output().unwrap();
    output.clone().assert().code(predicate::eq(85));

    let err = BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&["-c", "exit 500"])
        .unwrap_err();
    let output = err.as_output().unwrap();
    output.clone().assert().code(predicate::eq(244));

    let err = BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&["-c", "exit -500"])
        .unwrap_err();
    let output = err.as_output().unwrap();
    output.clone().assert().code(predicate::eq(12));
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_simple_pipeline() {
    BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&["-c", "echo needle | grep needle"])
        .unwrap()
        .assert()
        .stdout(predicates::str::similar("needle\n").from_utf8());
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_simple_redirects() {
    let temp_dir = generate_temp_directory().unwrap();
    let command = "echo 'test needle, please ignore' >outfile; grep <outfile 'needle'";
    let expected_stdout = "test needle, please ignore\n";
    BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .current_dir(temp_dir.path())
        .args(&["-c", command])
        .unwrap()
        .assert()
        .stdout(predicates::str::similar(expected_stdout).from_utf8());
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_stderr_redirect() {
    let temp_dir = generate_temp_directory().unwrap();
    let command = "2>errfile >&2 echo needle";
    BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .current_dir(temp_dir.path())
        .args(&["-c", command])
        .unwrap()
        .assert()
        .success();

    let mut file = File::open(temp_dir.path().join("errfile")).expect("unable to open errfile");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("failed to read errfile");
    assert_eq!(contents, "needle\n");
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_command_not_found() {
    let args = ["-c", "foo"];
    let expected_stderr = "bsh: foo: command not found\n";
    let err = BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&args)
        .unwrap_err();
    let output = err.as_output().unwrap();
    output
        .clone()
        .assert()
        .stderr(predicates::str::similar(expected_stderr).from_utf8())
        .code(predicate::eq(127));
}

#[test]
#[cfg(unix)] // TODO (#22): Support Windows
fn test_syntax_error() {
    let args = ["-c", ";"];
    let expected_stderr = "bsh: syntax error near: ;\n";
    let err = BIN_UNDER_TEST
        .command()
        .args(&[OsStr::new("--log"), LOG_FILE_NAME.as_os_str()])
        .args(&args)
        .unwrap_err();

    let output = err.as_output().unwrap();
    output
        .clone()
        .assert()
        .stderr(predicates::str::similar(expected_stderr).from_utf8())
        .code(predicate::eq(2));
}

fn generate_temp_directory() -> io::Result<TempDir> {
    // Because of limitation in `assert_cli`, temporary directory must be
    // subdirectory of directory containing Cargo.toml
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests"].iter().collect();
    tempfile::tempdir_in(root)
}
