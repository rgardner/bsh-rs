use errors::*;
use builtins;
use std::process::Command;
use shell::Shell;

pub struct Kill;

impl builtins::BuiltinCommand for Kill {
    fn name() -> &'static str {
        builtins::KILL_NAME
    }

    fn help() -> &'static str {
        "\
kill: kill pid | %jobspec
    Send a signal to a job.

    Send SIGTERM to the processes identified by JOBSPEC.

    Kill is a shell builtin for two reasons: it allows job IDs
    to be used instead of process IDs.

    Exit Status:
    Returns success unless an invalid option is given or an error occurs."
    }

    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if let None = args.first() {
            println!("{}", Kill::help());
            bail!(ErrorKind::BuiltinCommandError(Kill::usage(), 2));
        }

        let arg = args.first().unwrap();
        if arg.starts_with("%") {
            match arg[1..].parse::<u32>() {
                Ok(n) => {
                    match shell.kill_background_job(n) {
                        Ok(Some(job)) => {
                            println!("[{}]+\tTerminated: 15\t{}", n, job.command);
                            Ok(())
                        }
                        Ok(None) => {
                            bail!(ErrorKind::BuiltinCommandError(format!("kill: {}: no such job",
                                                                         arg),
                                                                 1));
                        }
                        Err(e) => Err(e),
                    }
                }
                Err(_) => {
                    bail!(ErrorKind::BuiltinCommandError(format!("kill: {}: arguments must be \
                                                                  job IDs",
                                                                 arg),
                                                         1));
                }
            }
        } else {
            let output = try!(Command::new("kill").args(&args).output());
            print!("{}", String::from_utf8_lossy(&output.stdout));
            Ok(())
        }
    }
}
