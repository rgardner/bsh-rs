//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the commands conform to their standard Bash counterparts.

use errors::*;
use parser::Command;
use shell::Shell;

use self::dirs::Cd;
use self::exit::Exit;
use self::help::Help;
use self::history::History;
use self::kill::Kill;

mod dirs;
mod exit;
mod help;
mod history;
mod kill;

const CD_NAME: &'static str = "cd";
const EXIT_NAME: &'static str = "exit";
const HELP_NAME: &'static str = "help";
const HISTORY_NAME: &'static str = "history";
const KILL_NAME: &'static str = "kill";

/// Represents a Bsh builtin command such as cd or help.
pub trait BuiltinCommand {
    /// The name of the command.
    fn name() -> &'static str;
    /// The help string to display to the user.
    fn help() -> &'static str;
    /// The usage string to display to the user.
    fn usage() -> String {
        Self::help().lines().nth(0).unwrap().to_owned()
    }
    /// Runs the command with the given arguments in the `shell` environment.
    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()>;
}

pub fn is_builtin(program: &str) -> bool {
    [CD_NAME, EXIT_NAME, HELP_NAME, HISTORY_NAME, KILL_NAME].contains(&program)
}

/// precondition: process is a builtin.
pub fn run(shell: &mut Shell, process: &Command) -> Result<()> {
    assert!(is_builtin(&process.program()));
    match &*process.program() {
        CD_NAME => Cd::run(shell, process.args().clone()),
        EXIT_NAME => Exit::run(shell, process.args().clone()),
        HELP_NAME => Help::run(shell, process.args().clone()),
        HISTORY_NAME => History::run(shell, process.args().clone()),
        KILL_NAME => Kill::run(shell, process.args().clone()),
        _ => unreachable!(),
    }
}
