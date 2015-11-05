//! BSH Parser

use std::result;

/// A specialized Result type for Parse operations.
///
/// This type is used because parsing can cause an error.
///
/// Like std::io::Result, users of this alias should generally use parse::Result instead of
/// importing this directly.
pub type Result<T> = result::Result<T, ParseError>;

quick_error! {
    #[derive(Debug)]
    /// Errors that can occur while parsing a bsh script
    pub enum ParseError {
        /// Generic syntax error if a more specific one does not exist
        SyntaxError(line: String) {
            description("unknown syntax error")
            display("-bsh: syntax error in line `{}`", line)
        }
    }
}

/// Represents a command and its associated variable parameters
#[derive(Debug)]
pub struct ParseCommand {
    command: String,
    variables: Vec<String>,
}

impl ParseCommand {
    fn new() -> ParseCommand {
        ParseCommand { command: String::new(), variables: Vec::new() }
    }
}

/// Represents all information associated with a user input
#[derive(Debug)]
pub struct ParseInfo {
    infile: Option<String>,
    outfile: Option<String>,
    background: bool,
    commands: Vec<ParseCommand>,
}

impl ParseInfo {
    fn new() -> ParseInfo {
        ParseInfo {
            infile: None,
            outfile: None,
            background: false,
            commands: Vec::new()
        }
    }
}


/// Parses input string into ParseInfo
pub fn parse(input: &str) -> Result<Option<ParseInfo>> {
    let argv: Vec<_> = input.trim().split_whitespace().collect();
    if argv.is_empty() {
        return Ok(None);
    }

    let mut info = ParseInfo::new();
    let mut cmd = ParseCommand::new();
    cmd.command = String::from(argv[0]);

    let mut infile = false;
    for &arg in &argv[1..] {
        if arg.starts_with("<") && !infile {
            if arg.len() > 1 {
                info.infile = Some(arg[1..].to_string());
            } else {
                infile = true;
            }
        } else if infile {
            info.infile = Some(String::from(arg));
            infile = false
        } else {
            cmd.variables.push(String::from(arg));
        }
    }
    info.commands.push(cmd);
    Ok(Some(info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_cmd() {
        let info = parse("echo").unwrap().unwrap();
        assert!(info.infile.is_none());
        assert!(info.outfile.is_none());
        assert_eq!(1, info.commands.len());
        assert_eq!("echo", info.commands[0].command);
        assert!(info.commands[0].variables.is_empty());
    }

    #[test]
    fn single_cmd_with_args() {
        let info = parse("echo hello brave new world").unwrap().unwrap();
        assert!(info.infile.is_none());
        assert!(info.outfile.is_none());
        assert_eq!(1, info.commands.len());
        assert_eq!("echo", info.commands[0].command);
        assert_eq!(4, info.commands[0].variables.len());
        assert_eq!("hello", info.commands[0].variables[0]);
        assert_eq!("brave", info.commands[0].variables[1]);
        assert_eq!("new", info.commands[0].variables[2]);
        assert_eq!("world", info.commands[0].variables[3]);
    }

    #[test]
    fn single_cmd_with_arg() {
        let info = parse("echo hello").unwrap().unwrap();
        assert!(info.infile.is_none());
        assert!(info.outfile.is_none());
        assert_eq!(1, info.commands.len());
        assert_eq!("echo", info.commands[0].command);
        assert_eq!(1, info.commands[0].variables.len());
        assert_eq!("hello", info.commands[0].variables[0]);
    }
}
