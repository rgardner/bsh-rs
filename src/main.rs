#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate bsh_rs;
extern crate docopt;
extern crate rustc_serialize;

use bsh_rs::{ParseJob, Shell};
use docopt::Docopt;
use std::process;

static HISTORY_CAPACITY: usize = 10;

const USAGE: &'static str = "
bsh.

Usage:
    bsh
    bsh -c <command>
    bsh (-h | --help)
    bsh --version

Options:
    -h --help    Show this screen.
    --version    Show version.
    -c           If the -c option is present, then commands are read from the first non-option
                     argument command_string.
";

/// Docopts input arguments.
#[derive(Debug, RustcDecodable)]
struct Args {
    flag_version: bool,
    flag_c: bool,
    arg_command: Option<String>,
}

/// Execute a command string in the context of the shell.
fn execute_command(mut shell: Shell, command: &str) {
    let mut info = match ParseJob::parse(command) {
        Ok(Some(info)) => info,
        Err(err) => {
            println!("{:?}", err);
            process::exit(1);
        }
        _ => process::exit(1),
    };

    if let Err(e) = shell.run(&mut info) {
        println!("bsh: {}", e);
        process::exit(1);
    }
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("bsh version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let mut shell = Shell::new(HISTORY_CAPACITY);
    if args.flag_c {
        execute_command(shell, &args.arg_command.unwrap());
        process::exit(0);
    }

    loop {
        shell.check_jobs();
        let mut input = String::new();
        match shell.prompt(&mut input) {
            Ok(0) => shell.exit(None),
            Err(_) => panic!("failed to read line."),
            _ => {}
        }

        input = input.trim().to_owned();

        if let Err(e) = shell.expand_history(&mut input) {
            println!("bsh: {}", e);
            continue;
        }
        shell.add_history(&input);

        let mut info = match ParseJob::parse(&input) {
            Ok(Some(info)) => info,
            Err(err) => {
                println!("{:?}", err);
                continue;
            }
            _ => continue,
        };

        if let Err(e) = shell.run(&mut info) {
            println!("bsh: {}", e);
        }
    }
}
