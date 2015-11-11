#![feature(plugin)]
#![plugin(docopt_macros)]

extern crate bsh_rs;
extern crate docopt;
extern crate rustc_serialize;

use bsh_rs::parse::ParseInfo;
use bsh_rs::shell::Shell;
use docopt::Docopt;
use std::process;

static HISTORY_CAPACITY: usize = 10;

docopt!(Args derive Debug, "
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
");

/// Execute a command string in the context of the shell.
fn execute_command(mut shell: Shell, command: &str) {
    let mut info = match ParseInfo::parse(command) {
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
    };
}

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("bsh version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let mut shell = Shell::new(HISTORY_CAPACITY);
    if args.flag_c {
        execute_command(shell, &args.arg_command);
        process::exit(0);
    }

    loop {
        shell.check_jobs();
        let mut input = String::new();
        match Shell::prompt(&mut input) {
            Ok(0) => {
                println!("exit");
                process::exit(0);
            }
            Err(_) => panic!("failed to read line."),
            _ => {}
        }
        let mut info = match ParseInfo::parse(&input) {
            Ok(Some(info)) => info,
            Err(err) => {
                println!("{:?}", err);
                continue;
            }
            _ => continue,
        };
        shell.add_history(&info);

        if let Err(e) = shell.run(&mut info) {
            println!("bsh: {}", e);
        }
    }
}
