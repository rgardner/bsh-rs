//! Bsh builtins
//!
//! This module includes the implementations of common shell builtin commands.
//! Where possible the
//! commands conform to their standard Bash counterparts.

use error::BshError;
use parse::Process;
use std::env;
use std::path::Path;
use std::process;
use std::result;

const CD: &'static str = "cd";
const EXIT: &'static str = "exit";
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

pub fn is_builtin(program: &str) -> bool {
    [CD, HISTORY, EXIT].contains(&program)
}

/// precondition: process is a builtin.
pub fn run(process: &Process) -> Result<()> {
    match &*process.program {
        CD => cd(process.args.clone()),
        EXIT => exit(process.args.clone()),
        HISTORY => history(process.args.clone()),
        _ => unreachable!(),
    }
}

fn cd(args: Vec<String>) -> Result<()> {
    let dir = match args.get(0).map(|x| &x[..]) {
        Some("~") | None =>
            try!(env::home_dir().ok_or(Error::InvalidArgs(String::from("cd: HOME not set"), 1))),
        Some("-") => match env::var_os("OLDPWD") {
            Some(val) => {
                println!("{}", val.to_str().unwrap());
                Path::new(val.as_os_str()).to_path_buf()
            }
            None => {
                return Err(BshError::BuiltinError(Error::InvalidArgs(String::from("cd: OLDPWD not set"), 1)));
            }
        },
        Some(val) => Path::new(val).to_path_buf(),
    };
    env::set_var("OLDPWD", try!(env::current_dir()));
    try!(env::set_current_dir(dir));
    Ok(())
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
