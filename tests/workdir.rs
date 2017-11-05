use std::env;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::process;
use std::str::FromStr;
use std::ffi::OsStr;

/// WorkDir represents a directory in which tests are run.
#[derive(Debug)]
pub struct WorkDir {
    /// The directory in which this text executable is running.
    root: PathBuf,
    /// The directory in which the test will run.
    dir: PathBuf,
}

impl WorkDir {
    /// Finds tests directory and generates a unique path to a cache file.
    pub fn new() -> WorkDir {
        WorkDir {
            root: env::current_exe()
                .unwrap()
                .parent()
                .expect("executable's directory")
                .to_path_buf(),
            dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests"),
        }
    }

    /// Builds a new command to run in this working directory with its unique cache file.
    pub fn command<I, S>(&self, args: I) -> process::Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(&self.bin());
        cmd.current_dir(&self.dir);
        cmd.args(args);
        cmd
    }

    /// Returns path to executable.
    #[cfg(not(windows))]
    fn bin(&self) -> PathBuf {
        self.root.join("../bsh")
    }

    /// Executes the command and collects its stdout.
    ///
    /// Panics if the return type cannot be created from a string.
    pub fn stdout<E: fmt::Debug, T: FromStr<Err = E>>(&self, cmd: &mut process::Command) -> T {
        let o = self.output(cmd);
        let stdout = String::from_utf8_lossy(&o.stdout);
        match stdout.parse() {
            Ok(t) => t,
            Err(err) => {
                panic!("could not convert from string: {:?}\n\n{}", err, stdout);
            }
        }
    }

    /// Executes the command and collects its output.
    ///
    /// Panic if the command fails.
    pub fn output(&self, cmd: &mut process::Command) -> process::Output {
        let o = cmd.output().unwrap();
        if !o.status.success() {
            let suggest = if o.stderr.is_empty() {
                "\n\nDid your search end up with no results?".to_string()
            } else {
                "".to_string()
            };

            panic!(
                "\n\n==========\n\
                 command failed but expected success!\
                 {}\
                 \n\ncommand: {:?}\
                 \ncwd: {}\
                 \n\nstatus: {}\
                 \n\nstdout: {}\
                 \n\nstderr: {}\
                 \n\n==========\n",
                suggest,
                cmd,
                self.dir.display(),
                o.status,
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
        }
        o
    }
}
