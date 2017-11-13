//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the commands conform to their standard Bash counterparts.

use errors::*;
use shell::Shell;

use self::dirs::Cd;
use self::env::Declare;
use self::env::Unset;
use self::exit::Exit;
use self::help::Help;
use self::history::History;
use self::kill::Kill;

mod dirs;
mod env;
mod exit;
mod help;
mod history;
mod kill;

const CD_NAME: &str = "cd";
const DECLARE_NAME: &str = "declare";
const EXIT_NAME: &str = "exit";
const HELP_NAME: &str = "help";
const HISTORY_NAME: &str = "history";
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
    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()>;
}

pub fn is_builtin<T: AsRef<str>>(argv: &[T]) -> bool {
    [
        CD_NAME,
        DECLARE_NAME,
        EXIT_NAME,
        HELP_NAME,
        HISTORY_NAME,
        KILL_NAME,
        UNSET_NAME,
    ].contains(&(program(argv).as_str()))
}

/// precondition: command is a builtin.
/// Returns (`exit_status_code`, `builtin_result`)
pub fn run<T: AsRef<str>>(shell: &mut Shell, argv: &[T]) -> (i32, Result<()>) {
    assert!(is_builtin(argv));
    let result = match &*program(argv) {
        CD_NAME => Cd::run(shell, args(argv)),
        DECLARE_NAME => Declare::run(shell, args(argv)),
        EXIT_NAME => Exit::run(shell, args(argv)),
        HELP_NAME => Help::run(shell, args(argv)),
        HISTORY_NAME => History::run(shell, args(argv)),
        KILL_NAME => Kill::run(shell, args(argv)),
        UNSET_NAME => Unset::run(shell, args(argv)),
        _ => unreachable!(),
    };

    let exit_status = get_builtin_exit_status(&result);
    (exit_status, result)
}

fn get_builtin_exit_status(result: &Result<()>) -> i32 {
    if let Err(ref e) = *result {
        match *e {
            Error(ErrorKind::BuiltinCommandError(_, code), _) => code,
            Error(ErrorKind::Msg(_), _) => 2,
            Error(_, _) => 1,
        }
    } else {
        0
    }
}

fn program<T: AsRef<str>>(argv: &[T]) -> String {
    argv[0].as_ref().to_string()
}

fn args<T: AsRef<str>>(argv: &[T]) -> Vec<String> {
    argv[1..].iter().map(|s| s.as_ref().to_string()).collect()
}
