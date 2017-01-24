use std::process;

#[derive(Debug, PartialEq)]
pub struct Command {
    pub argv: Vec<String>,
    pub infile: Option<String>,
    pub outfile: Option<String>,
}

impl Command {
    pub fn new() -> Command {
        Command {
            argv: vec![],
            infile: None,
            outfile: None,
        }
    }

    pub fn program(&self) -> String {
        self.argv.first().unwrap().to_string()
    }

    pub fn args(&self) -> Vec<String> {
        self.argv.iter().skip(1).cloned().collect()
    }

    /// Copies `command` and `args` into a `std::Command`.
    pub fn to_command(&self) -> process::Command {
        let mut cmd = process::Command::new(self.program());
        cmd.args(&self.args());
        cmd
    }
}

#[derive(Debug, PartialEq)]
pub struct Job {
    pub commands: Vec<Command>,
    pub background: bool,
}
