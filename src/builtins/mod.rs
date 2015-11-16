//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the commands conform to their standard Bash counterparts.

pub use self::dirs::*;

use error::{self, Result};
use parse::ParseCommand;
use shell::Shell;
use std::process;

mod dirs;

const CD: &'static str = "cd";
const EXIT: &'static str = "exit";
const HELP: &'static str = "help";
const HISTORY: &'static str = "history";

quick_error! {
    #[derive(Debug)]
    /// Errors that can occur while parsing a bsh script
    pub enum Error {
        /// Generic builtin error.
        InvalidArgs(message: String, code: i32) {
            description(message)
        }
    }
}

/// Represents a Bsh builtin command such as cd or help.
pub trait BuiltinCommand {
    /// The name of the command.
    fn name() -> String;
    /// The help string used for displaying to the user.
    fn help() -> String;
    /// Runs the command with the given arguments in the `shell` environment.
    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()>;
}

pub fn is_builtin(program: &str) -> bool {
    [CD, HELP, HISTORY, EXIT].contains(&program)
}

/// precondition: process is a builtin.
pub fn run(shell: &mut Shell, process: &ParseCommand) -> Result<()> {
    match &*process.program {
        CD => Cd::run(shell, process.args.clone()),
        EXIT => Exit::run(shell, process.args.clone()),
        HELP => Help::run(shell, process.args.clone()),
        HISTORY => History::run(shell, process.args.clone()),
        _ => unreachable!(),
    }
}

struct Help;

impl BuiltinCommand for Help {
    fn name() -> String {
        String::from("help")
    }

    fn help() -> String {
        String::from("\
help: help [pattern ...]
    Display helpful information about builtin commands. If PATTERN is specified,
    gives detailed help on all commands matching PATTERN, otherwise a list of the
    builtins is printed.")
    }

    fn run(_shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if args.is_empty() {
            println!("{}", Help::help());
        } else {
            let mut all_invalid = true;
            for arg in &args {
                let msg = match (*arg).as_ref() {
                    CD => Some(Cd::help()),
                    EXIT => Some(Exit::help()),
                    HELP => Some(Help::help()),
                    HISTORY => Some(History::help()),
                    _ => None,
                };
                if let Some(msg) = msg {
                    println!("{}", msg);
                    all_invalid = false;
                }
            }
            if all_invalid {
                let cmd = args.last().unwrap();
                let msg = format!("help: no help topics match {}", cmd);
                return Err(error::Error::BuiltinError(Error::InvalidArgs(msg, 1)));
            }
        }
        Ok(())
    }
}

struct Exit;

impl BuiltinCommand for Exit {
    fn name() -> String {
        String::from("exit")
    }

    fn help() -> String {
        String::from("\
exit: exit [n]
    Exit the shell with a status of N. If N is omitted, the exit status
    is 0.")
    }

    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if shell.has_background_jobs() {
            println!("There are stopped jobs.");
            return Ok(());
        }
        println!("exit");
        if let Some(code) = args.get(0) {
            let code: i32 = match code.parse() {
                Ok(num) => num,
                Err(_) => {
                    println!("bsh: exit: {}: numeric argument required", code);
                    2
                }
            };
            process::exit(code);
        } else {
            // TODO(rgardner): the exit code should be the last child's exit code.
            process::exit(0);
        }
    }
}


struct History;

impl BuiltinCommand for History {
    fn name() -> String {
        String::from("history")
    }

    fn help() -> String {
        String::from("\
history: history [-c] [n]
    Display the history list with line numbers. Argument of N
    says to list only the last N lines. The `-c' option causes
    the history list to be cleared by deleting all of the entries.")
    }

    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if let None = args.first() {
            println!("{}", shell.history);
            return Ok(());
        }
        let arg = args.first().unwrap();
        match &**arg {
            "-c" => shell.history.clear(),
            s => match s.parse::<usize>() {
                Ok(num) => println!("{}", shell.history.display(num)),
                Err(_) => {
                    let msg = format!("history: {}: nonnegative numeric argument required", s);
                    return Err(error::Error::BuiltinError(Error::InvalidArgs(msg, 1)));
                }
            },
        }
        Ok(())
    }
}
