#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate bsh_rs;
extern crate docopt;
extern crate rustc_serialize;
extern crate rustyline;

use bsh_rs::errors::*;
use bsh_rs::{Job, Shell};
use docopt::Docopt;
use rustyline::error::ReadlineError;
use std::process;
use std::fs::File;

static HISTORY_CAPACITY: usize = 10;
static EXIT_SUCCESS: i32 = 0;
static EXIT_FAILURE: i32 = 1;

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
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("bsh version {}", env!("CARGO_PKG_VERSION"));
        process::exit(EXIT_SUCCESS);
    }

    let mut shell = Shell::new(HISTORY_CAPACITY).unwrap();
    let res = if args.flag_c {
        execute_command(&mut shell, &args.arg_command.unwrap())
    } else if args.arg_file.is_some() {
        execute_from_file(&mut shell, &args.arg_file.unwrap())
    } else {
        execute_from_stdin(shell);
    };

    if let Err(e) = res {
        eprintln!("bsh: {}", e);
        shell.exit(Some(EXIT_FAILURE));
    } else {
        shell.exit(None);
    }
}

fn execute_command(shell: &mut Shell, command: &str) -> Result<()> {
    let jobs = try!(Job::parse(command));
    for mut job in jobs {
        job = shell.expand_variables(&job);
        try!(shell.run(&mut job));
    }

    Ok(())
}

fn execute_from_file(shell: &mut Shell, filename: &str) -> Result<()> {
    use std::io::Read;
    let mut f = try!(File::open(filename));
    let mut buffer = String::new();
    try!(f.read_to_string(&mut buffer));

    for line in buffer.split('\n') {
        try!(execute_command(shell, &line));
    }

    Ok(())
}

fn execute_from_stdin(mut shell: Shell) -> ! {
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
            eprintln!("bsh: {}", e);
            continue;
        }
        shell.add_history(&input);

        if let Err(e) = execute_command(&mut shell, &input) {
            eprintln!("bsh: {}", e);
        }
    }

    shell.exit(None)
}
