use builtins;
use builtins::prelude::*;
use std::process::Command;

pub struct Kill;

impl builtins::BuiltinCommand for Kill {
    const NAME: &'static str = builtins::KILL_NAME;

    const HELP: &'static str = "\
kill: kill pid | %jobspec
    Send a signal to a job.

    Send SIGTERM to the processes identified by JOBSPEC.

    Kill is a shell builtin for two reasons: it allows job IDs
    to be used instead of process IDs.

    Exit Status:
    Returns success unless an invalid option is given or an error occurs.";

    fn run(shell: &mut Shell, args: Vec<String>, stdout: &mut Write) -> Result<()> {
        if args.is_empty() {
            bail!(ErrorKind::BuiltinCommandError(Self::usage(), 2));
        }

        let arg = args.first().unwrap();
        if arg.starts_with('%') {
            match arg[1..].parse::<u32>() {
                Ok(n) => {
                    match shell.kill_background_job(n) {
                        Ok(Some(job)) => {
                            writeln!(stdout, "[{}]+\tTerminated: 15\t{}", n, job.command)?;
                            Ok(())
                        }
                        Ok(None) => {
                            bail!(ErrorKind::BuiltinCommandError(
                                format!("kill: {}: no such job", arg),
                                1,
                            ));
                        }
                        Err(e) => Err(e),
                    }
                }
                Err(_) => {
                    bail!(ErrorKind::BuiltinCommandError(
                        format!(
                            "kill: {}: arguments must be \
                             job IDs",
                            arg
                        ),
                        1,
                    ));
                }
            }
        } else {
            let output = Command::new("kill").args(&args).output()?;
            write!(stdout, "{}", String::from_utf8_lossy(&output.stdout))?;
            Ok(())
        }
    }
}
