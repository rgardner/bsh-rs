//! Integration Tests

extern crate assert_cli;
#[macro_use]
extern crate lazy_static;

use assert_cli::Assert;
use std::collections::HashMap;
use std::path::PathBuf;

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
        map.insert("simple_pipeline.bsh", ScriptData {
            stdout: "needle\n",
            stderr: "",
            exit_status: 0
        });
        map.insert("simple_exit_error.bsh", ScriptData { stdout: "", stderr: "", exit_status: 85 });
        map
    };
}

#[test]
#[ignore]
fn test_all_bsh_scripts() {
    for entry in get_path_to_test_fixtures()
        .join("scripts")
        .read_dir()
        .expect("read_dir failed")
    {
        let entry = entry.expect("unable to open test script");
        let filename = entry.file_name();
        let expected_data = BSH_SCRIPTS_MAP
            .get(filename.to_str().expect("filename should be valid Unicode"))
            .expect("bar");

        let file_path = entry.path();
        let unicode_file_path = file_path.to_str().expect(
            "file path should be valid Unicode",
        );
        Assert::cargo_binary("bsh")
            .with_args(&[unicode_file_path])
            .stdout()
            .is(expected_data.stdout)
            .exit_status_is(expected_data.exit_status)
            .unwrap();
    }
}

fn get_path_to_test_fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}
