use builtins::{self, BuiltinCommand, dirs, env, exit, history, kill};
use builtins::prelude::*;

pub struct Help;

impl BuiltinCommand for Help {
    const NAME: &'static str = builtins::HELP_NAME;

    const HELP: &'static str = "\
HELP: HELP [command ...]
    Display HELPful information about builtin commands. If COMMAND is specified,
    gives detailed HELP on all commands matching COMMAND, otherwise a list of the
    builtins is printed.";

    fn run(_shell: &mut Shell, args: Vec<String>, stdout: &mut Write) -> Result<()> {
        if args.is_empty() {
            print_all_usage_strings(stdout)?;
        } else {
            let mut all_invalid = true;
            for arg in &args {
                let msg = match arg.as_str() {
                    builtins::CD_NAME => Some(dirs::Cd::HELP),
                    builtins::DECLARE_NAME => Some(env::Declare::HELP),
                    builtins::EXIT_NAME => Some(exit::Exit::HELP),
                    builtins::HELP_NAME => Some(Self::HELP),
                    builtins::HISTORY_NAME => Some(history::History::HELP),
                    builtins::KILL_NAME => Some(kill::Kill::HELP),
                    builtins::UNSET_NAME => Some(env::Unset::HELP),
                    _ => None,
                };
                if let Some(msg) = msg {
                    stdout.write_all(msg.as_bytes())?;
                    all_invalid = false;
                }
            }
            if all_invalid {
                let cmd = args.last().unwrap();
                bail!(ErrorKind::BuiltinCommandError(
                    format!("help: no help topics match {}", cmd),
                    1,
                ));
            }
        }
        Ok(())
    }
}

fn print_all_usage_strings(stdout: &mut Write) -> Result<()> {
    writeln!(stdout, "{}", dirs::Cd::usage())?;
    writeln!(stdout, "{}", env::Declare::usage())?;
    writeln!(stdout, "{}", exit::Exit::usage())?;
    writeln!(stdout, "{}", Help::usage())?;
    writeln!(stdout, "{}", history::History::usage())?;
    writeln!(stdout, "{}", kill::Kill::usage())?;
    writeln!(stdout, "{}", env::Unset::usage())?;
    Ok(())
}
