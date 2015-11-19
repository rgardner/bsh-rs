//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the commands conform to their standard Bash counterparts.

pub use self::dirs::*;

use error::{self, Result};
use parse::ParseCommand;
use shell::Shell;
use std::process::{self, Command};

mod dirs;

const CD: &'static str = "cd";
const EXIT: &'static str = "exit";
const HELP: &'static str = "help";
const HISTORY: &'static str = "history";
const KILL: &'static str = "kill";

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
    [CD, EXIT, HELP, HISTORY, KILL].contains(&program)
}

/// precondition: process is a builtin.
pub fn run(shell: &mut Shell, process: &ParseCommand) -> Result<()> {
    match &*process.program {
        CD => Cd::run(shell, process.args.clone()),
        EXIT => Exit::run(shell, process.args.clone()),
        HELP => Help::run(shell, process.args.clone()),
        HISTORY => History::run(shell, process.args.clone()),
        KILL => Kill::run(shell, process.args.clone()),
        _ => unreachable!(),
    }
}

/// Returns the first line of a command's help string.
pub fn usage(help: &str) -> String {
    help.lines().nth(0).unwrap().to_owned()
}

struct Help;

impl BuiltinCommand for Help {
    fn name() -> String {
        String::from("help")
    }

    fn help() -> String {
        String::from("\
help: help [command ...]
    Display helpful information about builtin commands. If COMMAND is specified,
    gives detailed help on all commands matching COMMAND, otherwise a list of the
    builtins is printed.")
    }

    fn run(_shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if args.is_empty() {
            println!("{}", usage(&Cd::help()));
            println!("{}", usage(&Exit::help()));
            println!("{}", usage(&Help::help()));
            println!("{}", usage(&History::help()));
            println!("{}", usage(&Kill::help()));
        } else {
            let mut all_invalid = true;
            for arg in &args {
                let msg = match (*arg).as_ref() {
                    CD => Some(Cd::help()),
                    EXIT => Some(Exit::help()),
                    HELP => Some(Help::help()),
                    HISTORY => Some(History::help()),
                    KILL => Some(Kill::help()),
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
history: history [-c] [-s size] [n]
    Display the history list with line numbers. Argument of N
    says to list only the last N lines. The `-c' option causes
    the history list to be cleared by deleting all of the entries.
    The `-s' option sets the size of the history list.")
    }

    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if let None = args.first() {
            println!("{}", shell.history);
            return Ok(());
        }
        let arg = args.first().unwrap();
        match &**arg {
            "-c" => shell.history.clear(),
            "-s" => {
                if let Some(s) = args.get(2) {
                    if let Ok(n) = s.parse::<usize>() {
                        shell.history.set_size(n);
                    }
                }
            }
            s => match s.parse::<usize>() {
                Ok(n) => println!("{}", shell.history.display(n)),
                Err(_) => {
                    let msg = format!("history: {}: nonnegative numeric argument required", s);
                    return Err(error::Error::BuiltinError(Error::InvalidArgs(msg, 1)));
                }
            },
        }
        Ok(())
    }
}

struct Kill;

impl BuiltinCommand for Kill {
    fn name() -> String {
        String::from("kill")
    }

    fn help() -> String {
        String::from("\
kill: kill pid | %jobspec
    Send a signal to a job.

    Send SIGTERM to the processes identified by JOBSPEC.

    Kill is a shell builtin for two reasons: it allows job IDs
    to be used instead of process IDs.

    Exit Status:
    Returns success unless an invalid option is given or an error occurs.")
    }

    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if let None = args.first() {
            println!("{}", Kill::help());
            let msg = usage(&Kill::help());
            return Err(error::Error::BuiltinError(Error::InvalidArgs(msg, 2)));
        }

        let arg = args.first().unwrap();
        if arg.starts_with("%") {
            match arg[1..].parse::<u32>() {
                Ok(n) => match shell.kill_job(n) {
                    Ok(Some(job)) => {
                        println!("[{}]+\tTerminated: 15\t{}", n, job.command);
                        Ok(())
                    }
                    Ok(None) => {
                        let msg = format!("kill: {}: no such job", arg);
                        Err(error::Error::BuiltinError(Error::InvalidArgs(msg, 1)))
                    }
                    Err(e) => Err(e),
                },
                Err(_) => {
                    let msg = format!("kill: {}: arguments must be job IDs", arg);
                    Err(error::Error::BuiltinError(Error::InvalidArgs(msg, 1)))
                }
            }
        } else {
            match Command::new("kill").args(&args).output() {
                Ok(output) => {
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                    Ok(())
                }
                Err(e) => Err(error::Error::Io(e)),
            }
        }
    }
}
