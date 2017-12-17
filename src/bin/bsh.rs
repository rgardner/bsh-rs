#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate bsh_rs;
extern crate docopt;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate simplelog;

use bsh_rs::{BshExitStatusExt, Shell, ShellConfig};
use bsh_rs::errors::*;
use docopt::Docopt;
use simplelog::{WriteLogger, LogLevelFilter, Config};
use std::env;
use std::fs::OpenOptions;
use std::process::{self, ExitStatus};

const COMMAND_HISTORY_CAPACITY: usize = 10;
const LOG_FILE_NAME: &str = ".bsh_log";

const USAGE: &str = "
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

/// Docopts input arguments.
#[derive(Debug, Deserialize)]
struct Args {
    flag_version: bool,
    flag_c: bool,
    arg_command: Option<String>,
    arg_file: Option<String>,
}

fn main() {
    init_logger();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    debug!("{:?}", args);

    if args.flag_version {
        println!("bsh version {}", env!("CARGO_PKG_VERSION"));
    } else if args.flag_c || args.arg_file.is_some() {
        execute_from_command_string_or_file(&args);
    } else {
        execute_from_stdin();
    }
}

fn init_logger() {
    let mut log_path = env::home_dir().unwrap();
    log_path.push(LOG_FILE_NAME);
    let log_file = OpenOptions::new().create(true).append(true).open(log_path).unwrap();
    let _ = WriteLogger::init(LogLevelFilter::Trace, Config::default(), log_file);
}

fn execute_from_command_string_or_file(args: &Args) -> ! {
    let shell_config = ShellConfig::noninteractive();
    let mut shell = Shell::new(shell_config).unwrap_or_else(|e| display_error_and_exit(&e));

    let result = if let Some(ref command) = args.arg_command {
        shell.execute_command_string(command)
    } else if let Some(ref file_path) = args.arg_file {
        shell.execute_commands_from_file(&file_path)
    } else {
        unreachable!();
    };

    exit(result, &mut shell);
}

fn execute_from_stdin() -> ! {
    let shell_config = ShellConfig::interactive(COMMAND_HISTORY_CAPACITY);
    let mut shell = Shell::new(shell_config).unwrap_or_else(|e| display_error_and_exit(&e));
    shell.execute_from_stdin();
    shell.exit(None)
}

fn display_error_and_exit(error: &Error) -> ! {
    error!("failed to create shell: {}", error);
    eprintln!("bsh: {}", error);
    process::exit(ExitStatus::from_failure().code().unwrap());
}

fn exit(result: Result<()>, shell: &mut Shell) -> ! {
    if let Err(e) = result {
        eprintln!("bsh: {}", e);
        shell.exit(Some(ExitStatus::from_failure()));
    } else {
        shell.exit(None);
    }
}
