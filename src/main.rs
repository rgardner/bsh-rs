#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate bsh_rs;
extern crate docopt;
extern crate rustc_serialize;
extern crate rustyline;

use bsh_rs::errors::*;
use bsh_rs::{Shell, ShellConfig};
use docopt::Docopt;
use rustyline::error::ReadlineError;
use std::process;

const COMMAND_HISTORY_CAPACITY: usize = 10;
const EXIT_FAILURE: i32 = 1;

const USAGE: &'static str = "
bsh.

Usage:
    bsh
    bsh -c <command>
    bsh <file>
    bsh (-h | --help)
    bsh --version

Options:
    -h --help    Show this screen.
    --version    Show version.
    -c           If the -c option is present, then commands are read from the first non-option
                     argument command_string.
";

macro_rules! eprintln {
    ($($tt:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(&mut ::std::io::stderr(), $($tt)*);
    }}
}

/// Docopts input arguments.
#[derive(Debug, RustcDecodable)]
struct Args {
    flag_version: bool,
    flag_c: bool,
    arg_command: Option<String>,
    arg_file: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("bsh version {}", env!("CARGO_PKG_VERSION"));
    } else if args.flag_c || args.arg_file.is_some() {
        execute_from_command_string_or_file(&args);
    } else {
        execute_from_stdin();
    }
}

fn execute_from_command_string_or_file(args: &Args) -> ! {
    let shell_config = ShellConfig::noninteractive();
    let result = Shell::new(shell_config);
    if let Err(e) = result {
        eprintln!("bsh: {}", e);
        process::exit(EXIT_FAILURE);
    }

    let mut shell = result.unwrap();
    let result = if let Some(ref command_string) = args.arg_command {
        shell.execute_command_string(&command_string)
    } else if let Some(ref file_path) = args.arg_file {
        shell.execute_commands_from_file(&file_path)
    } else {
        Ok(())
    };

    display_error_and_exit(result, &mut shell);
}

fn execute_from_stdin() -> ! {
    let shell_config = ShellConfig::interactive(COMMAND_HISTORY_CAPACITY);
    let mut shell = Shell::new(shell_config).unwrap();

    loop {
        // Check the status of background jobs, removing exited ones.
        shell.check_background_jobs();

        let input = match shell.prompt() {
            Ok(line) => line.trim().to_owned(),
            Err(Error(ErrorKind::ReadlineError(ReadlineError::Eof), _)) => break,
            _ => continue,
        };

        if let Err(e) = shell.execute_command_string(&input) {
            eprintln!("bsh: {}", e);
        }
    }

    shell.exit(None)
}

fn display_error_and_exit(result: Result<()>, shell: &mut Shell) -> ! {
    if let Err(e) = result {
        eprintln!("bsh: {}", e);
        shell.exit(Some(EXIT_FAILURE));
    } else {
        shell.exit(None);
    }
}
