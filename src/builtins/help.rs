use builtins::{self, dirs, env, exit, history, jobs, kill, prelude::*, BuiltinCommand};

pub struct Help;

impl BuiltinCommand for Help {
    const NAME: &'static str = builtins::HELP_NAME;

    const HELP: &'static str = "\
help: help [command ...]
    Display helpful information about builtin commands. If COMMAND is specified,
    gives detailed help on all commands matching COMMAND, otherwise a list of the
    builtins is printed.";

    fn run<T: AsRef<str>>(_shell: &mut Shell, args: &[T], stdout: &mut Write) -> Result<()> {
        if args.is_empty() {
            print_all_usage_strings(stdout)?;
        } else {
            let mut all_invalid = true;
            for arg in args {
                let msg = match arg.as_ref() {
                    builtins::BG_NAME => Some(jobs::Bg::HELP),
                    builtins::CD_NAME => Some(dirs::Cd::HELP),
                    builtins::DECLARE_NAME => Some(env::Declare::HELP),
                    builtins::EXIT_NAME => Some(exit::Exit::HELP),
                    builtins::FG_NAME => Some(jobs::Fg::HELP),
                    builtins::HELP_NAME => Some(Self::HELP),
                    builtins::HISTORY_NAME => Some(history::History::HELP),
                    builtins::JOBS_NAME => Some(jobs::Jobs::HELP),
                    builtins::KILL_NAME => Some(kill::Kill::HELP),
                    builtins::UNSET_NAME => Some(env::Unset::HELP),
                    _ => None,
                };
                if let Some(msg) = msg {
                    writeln!(stdout, "{}", msg).context(ErrorKind::Io)?;
                    all_invalid = false;
                }
            }
            if all_invalid {
                let cmd = args.last().unwrap();
                return Err(Error::builtin_command(
                    format!("help: no help topics match {}", cmd.as_ref()),
                    1,
                ));
            }
        }
        Ok(())
    }
}

fn print_all_usage_strings(writer: &mut Write) -> Result<()> {
    writeln!(writer, "{}", jobs::Bg::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", dirs::Cd::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", env::Declare::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", exit::Exit::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", jobs::Fg::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", Help::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", history::History::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", jobs::Jobs::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", kill::Kill::usage()).context(ErrorKind::Io)?;
    writeln!(writer, "{}", env::Unset::usage()).context(ErrorKind::Io)?;
    Ok(())
}
