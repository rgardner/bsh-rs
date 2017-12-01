//! Integration Tests

extern crate assert_cli;
#[macro_use]
extern crate lazy_static;
extern crate tempdir;

use assert_cli::Assert;
use std::collections::HashMap;
use std::fs::DirEntry;
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

struct ScriptData<'a> {
    pub stdout: &'a str,
    pub stderr: &'a str,
    pub exit_status: i32,
}

lazy_static! {
    static ref BSH_SCRIPTS_MAP: HashMap<&'static str, ScriptData<'static>> = {
        let mut map = HashMap::new();
        map.insert("simple_echo.bsh", ScriptData { stdout: "test\n", stderr: "", exit_status: 0 });
        map.insert("simple_redirects.bsh", ScriptData {
            stdout: "test output, please ignore",
            stderr: "",
            exit_status: 0
        });
        map.insert("simple_pipeline.bsh", ScriptData {
            stdout: "needle\n",
            stderr: "",
            exit_status: 0
        });
        map.insert("simple_exit_error.bsh", ScriptData {
            stdout: "",
            stderr: "",
            exit_status: 85
        });
        map.insert("simple_exit_large.bsh", ScriptData {
            stdout: "",
            stderr: "",
            exit_status: 244
        });
        map.insert("simple_exit_negative.bsh", ScriptData {
            stdout: "",
            stderr: "",
            exit_status: 12
        });
        map
    };
}

#[test]
#[ignore]
fn test_all_simple_bsh_scripts() {
    let simple_scripts = get_path_to_test_scripts()
        .read_dir()
        .expect("read_dir failed")
        .map(|entry| entry.expect("filename should be valid Unicode"))
        .filter(|entry| is_simple_bsh_script(entry));

    for entry in simple_scripts {
        let temp_dir = generate_temp_directory().expect("unable to generate temp dir");
        let file_path = entry.path();
        let unicode_file_path = file_path.to_str().expect(
            "file path should be valid Unicode",
        );

        let filename = entry.file_name();
        let expected_data =
            BSH_SCRIPTS_MAP
                .get(filename.to_str().expect("filename should be valid Unicode"))
                .expect("simple script should have matching data in BSH_SCRIPTS_MAP");

        Assert::cargo_binary("bsh")
            .current_dir(temp_dir.path())
            .with_args(&[unicode_file_path])
            .stdout()
            .is(expected_data.stdout)
            .exit_status_is(expected_data.exit_status)
            .unwrap();
    }
}

fn get_path_to_test_scripts() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scripts")
}

/// Does filename start with 'simple' and end with '.bsh'?
fn is_simple_bsh_script(entry: &DirEntry) -> bool {
    let filename = entry.file_name();
    let unicode_filename = filename.to_str().expect("filename should be valid Unicode");
    unicode_filename.starts_with("simple") && unicode_filename.ends_with(".bsh")
}

fn generate_temp_directory() -> io::Result<TempDir> {
    // Because of limitation in `assert_cli`, temporary directory must be
    // subdirectory of directory containing Cargo.toml
    let temp_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    TempDir::new_in(temp_root, "temp")
}
