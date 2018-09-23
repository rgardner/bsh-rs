//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the commands conform to their standard Bash counterparts.

use std::iter;

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
    pub use shell::shell::Shell;
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
    fn run<T: AsRef<str>>(shell: &mut Shell, args: &[T], stdout: &mut Write) -> Result<()>;
}

pub fn is_builtin<T: AsRef<str>>(program: T) -> bool {
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
        .contains(&program.as_ref())
}

/// precondition: command is a builtin.
/// Returns (`exit_status_code`, `builtin_result`)
pub fn run<S1, S2>(
    shell: &mut Shell,
    program: S1,
    args: &[S2],
    stdout: &mut Write,
) -> (ExitStatus, Result<()>)
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    debug_assert!(is_builtin(&program));

    let result = match program.as_ref() {
        BG_NAME => Bg::run(shell, args, stdout),
        CD_NAME => Cd::run(shell, args, stdout),
        DECLARE_NAME => Declare::run(shell, args, stdout),
        EXIT_NAME => Exit::run(shell, args, stdout),
        FG_NAME => Fg::run(shell, args, stdout),
        HELP_NAME => Help::run(shell, args, stdout),
        HISTORY_NAME => History::run(shell, args, stdout),
        JOBS_NAME => Jobs::run(shell, args, stdout),
        KILL_NAME => Kill::run(shell, args, stdout),
        UNSET_NAME => Unset::run(shell, args, stdout),
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

pub fn parse_args<'a, 'de: 'a, D, S, I>(usage: &str, program: S, args: I) -> Result<D>
where
    D: serde::Deserialize<'de>,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    Docopt::new(usage)
        .unwrap()
        .argv(iter::once(program).chain(args))
        .deserialize()
        .map_err(|e| e.context(ErrorKind::Docopt).into())
}
