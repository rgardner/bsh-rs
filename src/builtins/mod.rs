//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the
//! commands conform to their standard Bash counterparts.

pub use self::dirs::*;

use error::BshError;
use parse::Process;
use std::process;
use std::result;

mod dirs;

const CD: &'static str = "cd";
const EXIT: &'static str = "exit";
const HELP: &'static str = "help";
const HISTORY: &'static str = "history";

/// A specialized Result type for Parse operations.
///
/// This type is used because parsing can cause an error.
///
/// Like std::io::Result, users of this alias should generally use parse::Result instead of
/// importing this directly.
pub type Result<T> = result::Result<T, BshError>;

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
    fn name() -> String;
    fn help() -> String;
    fn run(args: Vec<String>) -> Result<()>;
}

pub fn is_builtin(program: &str) -> bool {
    [CD, HELP, HISTORY, EXIT].contains(&program)
}

/// precondition: process is a builtin.
pub fn run(process: &Process) -> Result<()> {
    match &*process.program {
        CD => Cd::run(process.args.clone()),
        EXIT => exit(process.args.clone()),
        HELP => Help::run(process.args.clone()),
        HISTORY => history(process.args.clone()),
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

    fn run(args: Vec<String>) -> Result<()> {
        if args.is_empty() {
            println!("{}", Help::help());
        } else {
            let mut all_invalid = true;
            for arg in &args {
                let msg = match (*arg).as_ref() {
                    CD => Some(Cd::help()),
                    EXIT => Some(String::new()),
                    HELP => Some(Help::help()),
                    HISTORY => Some(String::new()),
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
                return Err(BshError::BuiltinError(Error::InvalidArgs(msg, 1)));
            }
        }
        Ok(())
    }
}

fn exit(args: Vec<String>) -> Result<()> {
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
        process::exit(0);
    }
}

fn history(_args: Vec<String>) -> Result<()> {
    Ok(())
}
