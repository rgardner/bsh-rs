//! BSH Parser

use std::result;
use std::process::Command;

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

/// Represents all information associated with a user input
#[derive(Debug)]
pub struct ParseInfo {
    infile: Option<String>,
    outfile: Option<String>,
    background: bool,
    commands: Vec<Command>,
}

impl ParseInfo {
    /// Parses input string into ParseInfo
    pub fn parse(input: &str) -> Result<Option<ParseInfo>> {
        let argv: Vec<_> = input.trim().split_whitespace().collect();
        if argv.is_empty() {
            return Ok(None);
        }

        let mut info = ParseInfoBuilder::new();
        let mut cmd = Command::new(argv[0]);

        let mut infile = false;
        let mut outfile = false;
        for &arg in &argv[1..] {
            if arg.starts_with("<") && !infile {
                if arg.len() > 1 {
                    info.infile(&arg[1..]);
                } else {
                    infile = true;
                }
            } else if infile {
                info.infile(arg);
                infile = false;
            } else if arg.starts_with(">") && !outfile {
                if arg.len() > 1 {
                    info.outfile(&arg[1..]);
                } else {
                    outfile = true;
                }
            } else if outfile {
                info.outfile(arg);
                outfile = false;
            } else if arg.starts_with("&") {
                info.background(true);
            } else {
                cmd.arg(arg);
            }
        }
        info.command(cmd);
        Ok(Some(info.build()))
    }
}

/// Build Parse Info
#[derive(Debug)]
pub struct ParseInfoBuilder {
    infile: Option<String>,
    outfile: Option<String>,
    background: bool,
    commands: Vec<Command>,
}

impl ParseInfoBuilder {
    /// Construct a new `ParseInfoBuilder` for parsing commands, with the following default
    /// configuration:
    ///
    /// * No input/output redirection
    /// * Runs in foreground
    /// * No program or commands
    ///
    /// Builder methods are provided to change these defaults and otherwise configure the job.
    pub fn new() -> ParseInfoBuilder {
        ParseInfoBuilder {
            infile: None,
            outfile: None,
            background: false,
            commands: Vec::new(),
        }
    }

    /// Add input redirection from the specified filename.
    pub fn infile(&mut self, filename: &str) -> &mut ParseInfoBuilder {
        self.infile = Some(String::from(filename));
        self
    }

    /// Add output redirection to the specified filename.
    pub fn outfile(&mut self, filename: &str) -> &mut ParseInfoBuilder {
        self.outfile = Some(String::from(filename));
        self
    }

    /// Configure job to run in the background.
    pub fn background(&mut self, background: bool) -> &mut ParseInfoBuilder {
        self.background = background;
        self
    }

    /// Add a new command.
    pub fn command(&mut self, command: Command) -> &mut ParseInfoBuilder {
        self.commands.push(command);
        self
    }

    /// Build the final job.
    pub fn build(self) -> ParseInfo {
        ParseInfo {
            infile: self.infile,
            outfile: self.outfile,
            background: self.background,
            commands: self.commands,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn empty() {
        assert!(ParseInfo::parse("").unwrap().is_none());
    }

    #[test]
    #[ignore]
    fn single_cmd() {
        let info = ParseInfoBuilder::new().command(Command::new("cmd")).build();
        assert_eq!(info, ParseInfo::parse("cmd").unwrap().unwrap());
    }

    #[test]
    #[ignore]
    fn single_cmd_with_args() {
        let mut cmd = Command::new("cmd");
        cmd.args(&vec!["var1", "var2", "var3"]);
        let info = ParseInfoBuilder::new().command(cmd).build();
        assert_eq!(info, ParseInfo::parse("cmd var1 var2 var3").unwrap().unwrap());
    }

    #[test]
    #[ignore]
    fn single_cmd_with_arg() {
        let mut cmd = Command::new("cmd");
        cmd.arg("var1");
        let info = ParseInfoBuilder::new().command(cmd).build();
        assert_eq!(info, ParseInfo::parse("cmd var1").unwrap().unwrap());
    }

    #[test]
    fn infile_valid() {
        let info = ParseInfoBuilder::new().command(Command::new("cmd")).infile("infile").build();
        assert_eq!(info, ParseInfo::parse("cmd <infile").unwrap().unwrap());
        assert_eq!(info, ParseInfo::parse("cmd < infile").unwrap().unwrap());
    }

    #[test]
    fn infile_invalid() {
        assert!(ParseInfo::parse("cmd <").unwrap().is_none());
    }

    #[test]
    fn outfile_valid() {
        let info = ParseInfoBuilder::new().command(Command::new("cmd")).outfile("outfile").build();
        assert_eq!(info, ParseInfo::parse("cmd >outfile").unwrap().unwrap());
        assert_eq!(info, ParseInfo::parse("cmd < outfile").unwrap().unwrap());
    }

    #[test]
    fn outfile_invalid() {
        assert!(ParseInfo::parse("cmd >").unwrap().is_none());
    }
}
