#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate bsh_rs;
extern crate docopt;
extern crate rustc_serialize;
extern crate rustyline;

use bsh_rs::errors::*;
use bsh_rs::{Job, Shell};
use docopt::Docopt;
use rustyline::error::ReadlineError;
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
fn execute_command(shell: &mut Shell, command: &str) -> Result<()> {
    let jobs = try!(Job::parse(command));
    for mut job in jobs {
        job = shell.expand_variables(&job);
        try!(shell.run(&mut job));
    }

    Ok(())
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("bsh version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let mut shell = Shell::new(HISTORY_CAPACITY).unwrap();
    if args.flag_c {
        if let Err(e) = execute_command(&mut shell, &args.arg_command.unwrap()) {
            println!("bsh: {}", e);
            shell.exit(Some(1))
        } else {
            shell.exit(Some(0))
        }
    }

    loop {
        // Check the status of background jobs, removing exited ones.
        shell.check_background_jobs();

        let mut input = match shell.prompt() {
            Ok(line) => line.trim().to_owned(),
            Err(Error(ErrorKind::ReadlineError(ReadlineError::Eof), _)) => break,
            _ => continue,
        };

        // Perform history substitutions and add user input to history.
        if let Err(e) = shell.expand_history(&mut input) {
            println!("bsh: {}", e);
            continue;
        }
        shell.add_history(&input);

        if let Err(e) = execute_command(&mut shell, &input) {
            println!("bsh: {}", e);
        }
    }

    shell.exit(None);
}
