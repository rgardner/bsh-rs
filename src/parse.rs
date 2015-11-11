//! BSH Parser

use std::result;
use std::process::Command;

/// The `Process` type acts a wrapper around `Commands`, facilitating testing.
#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    /// The program to execute.
    pub program: String,
    /// The arguments to the program.
    pub args: Vec<String>,
}

impl Process {
    /// Copies `command` and `args` into a `Command`.
    pub fn to_command(&self) -> Command {
        let mut cmd = Command::new(self.program.clone());
        cmd.args(&self.args.clone());
        cmd
    }
}

/// Builds Processes.
#[derive(Clone, Debug)]
pub struct ProcessBuilder {
    program: String,
    args: Vec<String>,
}

impl ProcessBuilder {
    /// Initializes a new ProcessBuilder with the given program and no arguments.
    pub fn new(program: &str) -> ProcessBuilder {
        ProcessBuilder {
            program: String::from(program),
            args: Vec::new(),
        }
    }

    /// Add an argument to pass to the program.
    pub fn arg(&mut self, arg: &str) -> &mut ProcessBuilder {
        self.args.push(String::from(arg));
        self
    }

    /// Add an argument to pass to the program.
    pub fn args(&mut self, args: &[&str]) -> &mut ProcessBuilder {
        self.args.extend(args.iter().map(|x| x.to_string()));
        self
    }

    /// Consumes the builder to build a Process.
    pub fn build(self) -> Process {
        Process {
            program: self.program,
            args: self.args,
        }
    }
}

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
#[derive(Clone, Debug, PartialEq)]
pub struct ParseInfo {
    /// Command line, used for messages
    pub command: String,
    /// The name of the input file, if one is specified
    pub infile: Option<String>,
    /// The file to write stdout to, if one is specified
    pub outfile: Option<String>,
    /// Run the command in the background, defaults to false
    pub background: bool,
    /// The commands to execute
    pub commands: Vec<Process>,
}

impl ParseInfo {
    /// Parses input string into ParseInfo
    pub fn parse(input: &str) -> Result<Option<ParseInfo>> {
        let argv: Vec<_> = input.trim().split_whitespace().collect();
        if argv.is_empty() {
            return Ok(None);
        }

        let mut info = ParseInfoBuilder::new(input);
        let mut cmd = ProcessBuilder::new(argv[0]);

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
        if infile || outfile {
            return Err(ParseError::SyntaxError(input.into()));
        }
        info.command(cmd.build());
        Ok(Some(info.build()))
    }
}

/// Build Parse Info
#[derive(Debug)]
pub struct ParseInfoBuilder {
    command: String,
    infile: Option<String>,
    outfile: Option<String>,
    background: bool,
    commands: Vec<Process>,
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
    pub fn new(command: &str) -> ParseInfoBuilder {
        ParseInfoBuilder {
            command: String::from(command),
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
    pub fn command(&mut self, command: Process) -> &mut ParseInfoBuilder {
        self.commands.push(command);
        self
    }

    /// Build the final job.
    pub fn build(self) -> ParseInfo {
        ParseInfo {
            command: self.command,
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

    #[test]
    fn empty() {
        assert!(ParseInfo::parse("").unwrap().is_none());
    }

    #[test]
    fn single_cmd() {
        let input = "cmd";
        let process = ProcessBuilder::new("cmd").build();
        let mut info = ParseInfoBuilder::new(input);
        info.command(process);
        assert_eq!(info.build(), ParseInfo::parse(input).unwrap().unwrap());
    }

    #[test]
    fn single_cmd_with_args() {
        let input = "cmd var1 var2 var3";
        let mut process = ProcessBuilder::new("cmd");
        process.args(&["var1", "var2", "var3"]);
        let mut info = ParseInfoBuilder::new(input);
        info.command(process.build());
        assert_eq!(info.build(), ParseInfo::parse(input).unwrap().unwrap());
    }

    #[test]
    fn single_cmd_with_arg() {
        let input = "cmd var1";
        let mut process = ProcessBuilder::new("cmd");
        process.arg("var1");
        let mut info = ParseInfoBuilder::new(input);
        info.command(process.build());
        assert_eq!(info.build(), ParseInfo::parse("cmd var1").unwrap().unwrap());
    }

    #[test]
    fn infile_valid() {
        let input_no_space = "cmd <infile";
        let input_with_space = "cmd < infile";
        let mut infob = ParseInfoBuilder::new(input_no_space);
        infob.command(ProcessBuilder::new("cmd").build());
        infob.infile("infile");
        let info = infob.build();
        assert_eq!(info.infile, ParseInfo::parse(input_no_space).unwrap().unwrap().infile);
        assert_eq!(info.infile, ParseInfo::parse(input_with_space).unwrap().unwrap().infile);
    }

    #[test]
    fn infile_invalid() {
        assert!(ParseInfo::parse("cmd <").is_err());
    }

    #[test]
    fn outfile_valid() {
        let input_no_space = "cmd >outfile";
        let input_with_space = "cmd > outfile";
        let mut infob = ParseInfoBuilder::new(input_no_space);
        infob.command(ProcessBuilder::new("cmd").build());
        infob.outfile("outfile");
        let info = infob.build();
        assert_eq!(info.outfile, ParseInfo::parse(input_no_space).unwrap().unwrap().outfile);
        assert_eq!(info.outfile, ParseInfo::parse(input_with_space).unwrap().unwrap().outfile);
    }

    #[test]
    fn outfile_invalid() {
        assert!(ParseInfo::parse("cmd >").is_err());
    }
}
