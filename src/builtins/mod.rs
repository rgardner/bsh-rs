//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the commands conform to their standard Bash counterparts.

use docopt::Docopt;
use failure::Fail;
use serde;

use self::prelude::*;

use self::dirs::Cd;
use self::env::{Declare, Unset};
use self::exit::Exit;
use self::help::Help;
use self::history::History;
use self::jobs::{Bg, Fg, Jobs};
use self::kill::Kill;

pub mod prelude {
    pub use std::io::Write;
    pub use std::process::ExitStatus;

    pub use failure::ResultExt;

    pub use super::parse_args;
    pub use errors::{Error, ErrorKind, Result};
    pub use shell::Shell;
    pub use util::BshExitStatusExt;
}

mod dirs;
mod env;
mod exit;
mod help;
mod history;
mod jobs;
mod kill;

const BG_NAME: &str = "bg";
const CD_NAME: &str = "cd";
const DECLARE_NAME: &str = "declare";
const EXIT_NAME: &str = "exit";
const FG_NAME: &str = "fg";
const HELP_NAME: &str = "help";
const HISTORY_NAME: &str = "history";
const JOBS_NAME: &str = "jobs";
const KILL_NAME: &str = "kill";
const UNSET_NAME: &str = "unset";

/// Represents a Bsh builtin command such as cd or help.
pub trait BuiltinCommand {
    /// The NAME of the command.
    const NAME: &'static str;
    /// The help string to display to the user.
    const HELP: &'static str;
    /// The usage string to display to the user.
    fn usage() -> String {
        Self::HELP.lines().nth(0).unwrap().to_owned()
    }
    /// Runs the command with the given arguments in the `shell` environment.
    fn run(shell: &mut Shell, args: Vec<String>, stdout: &mut Write) -> Result<()>;
}

pub fn is_builtin<T: AsRef<str>>(argv: &[T]) -> bool {
    [
        BG_NAME,
        CD_NAME,
        DECLARE_NAME,
        EXIT_NAME,
        FG_NAME,
        HELP_NAME,
        HISTORY_NAME,
        KILL_NAME,
        JOBS_NAME,
        UNSET_NAME,
    ]
        .contains(&(program(argv).as_str()))
}

/// precondition: command is a builtin.
/// Returns (`exit_status_code`, `builtin_result`)
pub fn run<T: AsRef<str>>(
    shell: &mut Shell,
    argv: &[T],
    stdout: &mut Write,
) -> (ExitStatus, Result<()>) {
    let result = match &*program(argv) {
        BG_NAME => Bg::run(shell, get_argv(argv), stdout),
        CD_NAME => Cd::run(shell, args(argv), stdout),
        DECLARE_NAME => Declare::run(shell, args(argv), stdout),
        EXIT_NAME => Exit::run(shell, args(argv), stdout),
        FG_NAME => Fg::run(shell, get_argv(argv), stdout),
        HELP_NAME => Help::run(shell, args(argv), stdout),
        HISTORY_NAME => History::run(shell, args(argv), stdout),
        JOBS_NAME => Jobs::run(shell, get_argv(argv), stdout),
        KILL_NAME => Kill::run(shell, args(argv), stdout),
        UNSET_NAME => Unset::run(shell, args(argv), stdout),
        _ => unreachable!(),
    };

    let exit_status = get_builtin_exit_status(&result);
    (exit_status, result)
}

fn get_builtin_exit_status(result: &Result<()>) -> ExitStatus {
    let status = if let Err(ref e) = *result {
        match *e.kind() {
            ErrorKind::BuiltinCommand { code, .. } => code,
            _ => 1,
        }
    } else {
        0
    };

    ExitStatus::from_status(status)
}

fn program<T: AsRef<str>>(argv: &[T]) -> String {
    argv[0].as_ref().to_string()
}

fn get_argv<T: AsRef<str>>(argv: &[T]) -> Vec<String> {
    argv.iter().map(|s| s.as_ref().to_string()).collect()
}

fn args<T: AsRef<str>>(argv: &[T]) -> Vec<String> {
    get_argv(argv)[1..].to_vec()
}

pub fn parse_args<'a, 'de: 'a, D>(usage: &str, argv: &[String]) -> Result<D>
where
    D: serde::Deserialize<'de>,
{
    Docopt::new(usage)
        .unwrap()
        .argv(argv)
        .deserialize()
        .map_err(|e| e.context(ErrorKind::Docopt).into())
}
