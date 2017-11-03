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

const CD_NAME: &'static str = "cd";
const DECLARE_NAME: &'static str = "declare";
const EXIT_NAME: &'static str = "exit";
const HELP_NAME: &'static str = "help";
const HISTORY_NAME: &'static str = "history";
const KILL_NAME: &'static str = "kill";
const UNSET_NAME: &'static str = "unset";

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

/// precondition: process is a builtin.
pub fn run(shell: &mut Shell, process: &Command) -> Result<()> {
    assert!(is_builtin(&process.program()));
    match &*process.program() {
        CD_NAME => Cd::run(shell, process.args().clone()),
        DECLARE_NAME => Declare::run(shell, process.args().clone()),
        EXIT_NAME => Exit::run(shell, process.args().clone()),
        HELP_NAME => Help::run(shell, process.args().clone()),
        HISTORY_NAME => History::run(shell, process.args().clone()),
        KILL_NAME => Kill::run(shell, process.args().clone()),
        UNSET_NAME => Unset::run(shell, process.args().clone()),
        _ => unreachable!(),
    }
}
