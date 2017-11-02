use errors::*;
use builtins::{self, dirs, env, exit, history, kill, BuiltinCommand};
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
                    builtins::CD_NAME => Some(dirs::Cd::help()),
                    builtins::DECLARE_NAME => Some(env::Declare::help()),
                    builtins::EXIT_NAME => Some(exit::Exit::help()),
                    builtins::HELP_NAME => Some(Help::help()),
                    builtins::HISTORY_NAME => Some(history::History::help()),
                    builtins::KILL_NAME => Some(kill::Kill::help()),
                    builtins::UNSET_NAME => Some(env::Unset::help()),
                    _ => None,
                };
                if let Some(msg) = msg {
                    println!("{}", msg);
                    all_invalid = false;
                }
            }
            if all_invalid {
                let cmd = args.last().unwrap();
                bail!(ErrorKind::BuiltinCommandError(
                    format!("help: no help topics match {}", cmd),
                    1
                ));
            }
        }
        Ok(())
    }
}

fn print_all_usage_strings() {
    println!("{}", dirs::Cd::usage());
    println!("{}", env::Declare::usage());
    println!("{}", exit::Exit::usage());
    println!("{}", Help::usage());
    println!("{}", history::History::usage());
    println!("{}", kill::Kill::usage());
    println!("{}", env::Unset::usage());
}
