//! BSH Parser

use std::process::Command;

/// The `ParseCommand` type acts a wrapper around `Commands`, facilitating testing.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseCommand {
    /// The program to execute.
    pub program: String,
    /// The arguments to the program.
    pub args: Vec<String>,
}

impl ParseCommand {
    /// Copies `command` and `args` into a `Command`.
    pub fn to_command(&self) -> Command {
        let mut cmd = Command::new(self.program.clone());
        cmd.args(&self.args.clone());
        cmd
    }
}

/// Builds ParseCommandes.
#[derive(Clone, Debug)]
pub struct ParseCommandBuilder {
    program: String,
    args: Vec<String>,
}

impl ParseCommandBuilder {
    /// Initializes a new ParseCommandBuilder with the given program and no arguments.
    pub fn new(program: &str) -> ParseCommandBuilder {
        ParseCommandBuilder {
            program: String::from(program),
            args: Vec::new(),
        }
    }

    /// Add an argument to pass to the program.
    pub fn arg(&mut self, arg: &str) -> &mut ParseCommandBuilder {
        self.args.push(String::from(arg));
        self
    }

    /// Add an argument to pass to the program.
    pub fn args(&mut self, args: &[&str]) -> &mut ParseCommandBuilder {
        self.args.extend(args.iter().map(|x| (*x).to_owned()));
        self
    }

    /// Consumes the builder to build a ParseCommand.
    pub fn build(self) -> ParseCommand {
        ParseCommand {
            program: self.program,
            args: self.args,
        }
    }
}

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
pub struct ParseJob {
    /// Command line, used for messages
    pub command: String,
    /// The name of the input file, if one is specified
    pub infile: Option<String>,
    /// The file to write stdout to, if one is specified
    pub outfile: Option<String>,
    /// Run the command in the background, defaults to false
    pub background: bool,
    /// The commands to execute
    pub commands: Vec<ParseCommand>,
}

impl ParseJob {
    /// Parses input string into ParseJob
    ///
    /// # Examples
    ///
    /// ```
    /// use bsh_rs::parse::{ParseJob, ParseCommandBuilder};
    ///
    /// let job = ParseJob::parse("echo test").unwrap().unwrap();
    /// assert_eq!(job.command, "echo test");
    /// assert!(job.infile.is_none());
    /// assert!(job.outfile.is_none());
    /// assert!(!job.background);
    ///
    /// let mut expected_command = ParseCommandBuilder::new("echo");
    /// expected_command.arg("test");
    /// assert_eq!(job.commands, vec![expected_command.build()]);
    /// ```
    pub fn parse(input: &str) -> Result<Option<ParseJob>, ParseError> {
        let input_trimmed = input.trim();
        let argv: Vec<_> = input_trimmed.split_whitespace().collect();
        if argv.is_empty() {
            return Ok(None);
        }

        let mut info = ParseJobBuilder::new(input_trimmed);
        let mut cmd = ParseCommandBuilder::new(argv[0]);

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
pub struct ParseJobBuilder {
    command: String,
    infile: Option<String>,
    outfile: Option<String>,
    background: bool,
    commands: Vec<ParseCommand>,
}

impl ParseJobBuilder {
    /// Construct a new `ParseJobBuilder` for parsing commands, with the following default
    /// configuration:
    ///
    /// * No input/output redirection
    /// * Runs in foreground
    /// * No program or commands
    ///
    /// Builder methods are provided to change these defaults and otherwise configure the job.
    pub fn new(command: &str) -> ParseJobBuilder {
        ParseJobBuilder {
            command: String::from(command),
            infile: None,
            outfile: None,
            background: false,
            commands: Vec::new(),
        }
    }

    /// Add input redirection from the specified filename.
    pub fn infile(&mut self, filename: &str) -> &mut ParseJobBuilder {
        self.infile = Some(String::from(filename));
        self
    }

    /// Add output redirection to the specified filename.
    pub fn outfile(&mut self, filename: &str) -> &mut ParseJobBuilder {
        self.outfile = Some(String::from(filename));
        self
    }

    /// Configure job to run in the background.
    pub fn background(&mut self, background: bool) -> &mut ParseJobBuilder {
        self.background = background;
        self
    }

    /// Add a new command.
    pub fn command(&mut self, command: ParseCommand) -> &mut ParseJobBuilder {
        self.commands.push(command);
        self
    }

    /// Build the final job.
    pub fn build(self) -> ParseJob {
        ParseJob {
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
        assert!(ParseJob::parse("").unwrap().is_none());
    }

    #[test]
    fn single_cmd() {
        let input = "cmd";
        let process = ParseCommandBuilder::new("cmd").build();
        let mut info = ParseJobBuilder::new(input);
        info.command(process);
        assert_eq!(info.build(), ParseJob::parse(input).unwrap().unwrap());
    }

    #[test]
    fn single_cmd_with_args() {
        let input = "cmd var1 var2 var3";
        let mut process = ParseCommandBuilder::new("cmd");
        process.args(&["var1", "var2", "var3"]);
        let mut info = ParseJobBuilder::new(input);
        info.command(process.build());
        assert_eq!(info.build(), ParseJob::parse(input).unwrap().unwrap());
    }

    #[test]
    fn single_cmd_with_arg() {
        let input = "cmd var1";
        let mut process = ParseCommandBuilder::new("cmd");
        process.arg("var1");
        let mut info = ParseJobBuilder::new(input);
        info.command(process.build());
        assert_eq!(info.build(), ParseJob::parse("cmd var1").unwrap().unwrap());
    }

    #[test]
    fn infile_valid() {
        let input_no_space = "cmd <infile";
        let input_with_space = "cmd < infile";
        let mut infob = ParseJobBuilder::new(input_no_space);
        infob.command(ParseCommandBuilder::new("cmd").build());
        infob.infile("infile");
        let info = infob.build();
        assert_eq!(info.infile,
                   ParseJob::parse(input_no_space).unwrap().unwrap().infile);
        assert_eq!(info.infile,
                   ParseJob::parse(input_with_space).unwrap().unwrap().infile);
    }

    #[test]
    fn infile_invalid() {
        assert!(ParseJob::parse("cmd <").is_err());
    }

    #[test]
    fn outfile_valid() {
        let input_no_space = "cmd >outfile";
        let input_with_space = "cmd > outfile";
        let mut infob = ParseJobBuilder::new(input_no_space);
        infob.command(ParseCommandBuilder::new("cmd").build());
        infob.outfile("outfile");
        let info = infob.build();
        assert_eq!(info.outfile,
                   ParseJob::parse(input_no_space).unwrap().unwrap().outfile);
        assert_eq!(info.outfile,
                   ParseJob::parse(input_with_space).unwrap().unwrap().outfile);
    }

    #[test]
    fn outfile_invalid() {
        assert!(ParseJob::parse("cmd >").is_err());
    }
}
