//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the commands conform to their standard Bash counterparts.

use errors::*;
use parser::Command;
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

pub fn is_builtin(program: &str) -> bool {
    [
        CD_NAME,
        DECLARE_NAME,
        EXIT_NAME,
        HELP_NAME,
        HISTORY_NAME,
        KILL_NAME,
        UNSET_NAME,
    ].contains(&program)
}

/// precondition: command is a builtin.
pub fn run(shell: &mut Shell, command: &mut Command) -> Result<()> {
    assert!(is_builtin(&command.program()));
    let result = match &*command.program() {
        CD_NAME => Cd::run(shell, command.args().clone()),
        DECLARE_NAME => Declare::run(shell, command.args().clone()),
        EXIT_NAME => Exit::run(shell, command.args().clone()),
        HELP_NAME => Help::run(shell, command.args().clone()),
        HISTORY_NAME => History::run(shell, command.args().clone()),
        KILL_NAME => Kill::run(shell, command.args().clone()),
        UNSET_NAME => Unset::run(shell, command.args().clone()),
        _ => unreachable!(),
    };

    command.status = get_builtin_exit_status(&result);
    result
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
