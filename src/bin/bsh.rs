use std::path::{Path, PathBuf};
use std::process::{self, ExitStatus};

use bsh::errors::*;
use bsh::{create_shell, BshExitStatusExt, Shell, ShellConfig};
use docopt::Docopt;
use log::{debug, error};
use serde_derive::Deserialize;

const COMMAND_HISTORY_CAPACITY: usize = 10;
const LOG_FILE_NAME: &str = ".bsh_log";

const USAGE: &str = "
bsh.

Usage:
    bsh [options]
    bsh [options] -c <command>
    bsh [options] <file>
    bsh (-h | --help)
    bsh --version

Options:
    -h --help       Show this screen.
    --version       Show version.
    -c              If the -c option is present, then commands are read from the first non-option
                        argument command_string.
    --log=<path>    File to write log to, defaults to ~/.bsh_log
";

/// Docopts input arguments.
#[derive(Debug, Deserialize)]
struct Args {
    arg_command: Option<String>,
    arg_file: Option<String>,
    flag_version: bool,
    flag_c: bool,
    flag_log: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    init_logger(&args.flag_log);
    debug!("{:?}", args);

    if args.flag_version {
        println!("bsh version {}", env!("CARGO_PKG_VERSION"));
    } else if args.flag_c || args.arg_file.is_some() {
        execute_from_command_string_or_file(&args);
    } else {
        execute_from_stdin();
    }
}

fn init_logger(path: &Option<String>) {
    let log_path = path
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(default_log_path);

    let pid = process::id();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}: {}",
                pid,
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("rustyline", log::LevelFilter::Info)
        .chain(fern::log_file(log_path).unwrap())
        .apply()
        .unwrap();
}

fn default_log_path() -> PathBuf {
    dirs::home_dir().unwrap().join(LOG_FILE_NAME)
}

fn execute_from_command_string_or_file(args: &Args) -> ! {
    let shell_config = ShellConfig::noninteractive();
    let mut shell = create_shell(shell_config).unwrap_or_else(|e| display_error_and_exit(&e));

    let result = if let Some(ref command) = args.arg_command {
        shell.execute_command_string(command)
    } else if let Some(ref file_path) = args.arg_file {
        shell.execute_commands_from_file(Path::new(file_path))
    } else {
        unreachable!();
    };

    exit(result, &mut *shell);
}

fn execute_from_stdin() -> ! {
    let shell_config = ShellConfig::interactive(COMMAND_HISTORY_CAPACITY);
    let mut shell = create_shell(shell_config).unwrap_or_else(|e| display_error_and_exit(&e));
    shell.execute_from_stdin();
    shell.exit(None)
}

fn display_error_and_exit(error: &Error) -> ! {
    error!("failed to create shell: {}", error);
    eprintln!("bsh: {}", error);
    process::exit(ExitStatus::from_failure().code().unwrap());
}

fn exit(result: Result<()>, shell: &mut dyn Shell) -> ! {
    if let Err(e) = result {
        eprintln!("bsh: {}", e);
        shell.exit(Some(ExitStatus::from_failure()));
    } else {
        shell.exit(None);
    }
}
