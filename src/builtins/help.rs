use errors::*;
use builtins::{self, BuiltinCommand, Cd, Exit, History, Kill};
use shell::Shell;

pub struct Help;

impl BuiltinCommand for Help {
    fn name() -> &'static str {
        "help"
    }

    fn help() -> &'static str {
        "\
help: help [command ...]
    Display helpful information about builtin commands. If COMMAND is specified,
    gives detailed help on all commands matching COMMAND, otherwise a list of the
    builtins is printed."
    }

    fn run(_shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if args.is_empty() {
            print_all_usage_strings();
        } else {
            let mut all_invalid = true;
            for arg in &args {
                let msg = match arg.as_str() {
                    builtins::CD_NAME => Some(Cd::help()),
                    builtins::EXIT_NAME => Some(Exit::help()),
                    builtins::HELP_NAME => Some(Help::help()),
                    builtins::HISTORY_NAME => Some(History::help()),
                    builtins::KILL_NAME => Some(Kill::help()),
                    _ => None,
                };
                if let Some(msg) = msg {
                    println!("{}", msg);
                    all_invalid = false;
                }
            }
            if all_invalid {
                let cmd = args.last().unwrap();
                bail!(ErrorKind::BuiltinCommandError(format!("help: no help topics match {}",
                                                             cmd),
                                                     1));
            }
        }
        Ok(())
    }
}

fn print_all_usage_strings() {
    println!("{}", Cd::usage());
    println!("{}", Exit::usage());
    println!("{}", Help::usage());
    println!("{}", History::usage());
    println!("{}", Kill::usage());
}
