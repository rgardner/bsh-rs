//! Integration Tests

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::path::PathBuf;
use workdir::WorkDir;

mod workdir;

struct ScriptData<'a> {
    pub stdout: &'a str,
    pub stderr: &'a str,
    pub exit_status: i32,
}

lazy_static! {
    static ref BSH_SCRIPTS_MAP: HashMap<&'static str, ScriptData<'static>> = {
        let mut map = HashMap::new();
        map.insert("simple_echo.bsh", ScriptData { stdout: "test\n", stderr: "", exit_status: 0 });
        map
    };
}

#[test]
fn test_all_bsh_scripts() {
    for entry in get_path_to_test_fixtures()
        .join("scripts")
        .read_dir()
        .expect("read_dir failed")
    {
        let entry = entry.expect("unable to open test script");
        let file_name = entry.file_name();
        let wd = WorkDir::new();
        let mut cmd = wd.command(&[entry.path()]);
        let expected_data = BSH_SCRIPTS_MAP
            .get(file_name.to_str().expect("foo"))
            .expect("bar");

        let actual_stdout: String = wd.stdout(&mut cmd);
        let actual_status = cmd.output().ok().and_then(|o| o.status.code()).expect(
            "failed to get exit status",
        );
        assert_eq!(actual_stdout, expected_data.stdout);
        assert_eq!(actual_status, expected_data.exit_status);
    }
}

fn get_path_to_test_fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}
