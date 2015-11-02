//! BSH Parser
use std::process::Command;

/// Parses input string into Command
pub fn parse(input: &str) -> Option<Command> {
    let argv: Vec<_> = input.split_whitespace().collect();
    if argv.is_empty() {
        None
    } else {
        let mut cmd = Command::new(argv[0]);
        cmd.args(&argv[1..]);
        Some(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Output};

    fn echo_nothing() -> Output {
        Command::new("echo").output().unwrap()
    }

    fn echo_hello() -> Output {
        Command::new("echo").arg("hello").output().unwrap()
    }

    fn echo_multi() -> Output {
        Command::new("echo").args(&vec!["hello", "brave", "new", "world"]).output().unwrap()
    }

    #[test]
    fn single_cmd() {
        let mut cmd = parse("echo").unwrap();
        assert_eq!(echo_nothing().stdout, cmd.output().unwrap().stdout);
    }

    #[test]
    fn single_cmd_with_args() {
        let mut cmd = parse("echo hello brave new world").unwrap();
        assert_eq!(echo_multi().stdout, cmd.output().unwrap().stdout);
    }

    #[test]
    fn single_cmd_with_arg() {
        let mut cmd = parse("echo hello").unwrap();
        assert_eq!(echo_hello().stdout, cmd.output().unwrap().stdout);
    }
}
