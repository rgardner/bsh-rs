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
