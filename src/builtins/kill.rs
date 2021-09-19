use std::ffi::OsStr;
use std::process::Command;

use crate::builtins::{self, prelude::*};

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

    fn run<T: AsRef<str>>(shell: &mut dyn Shell, args: &[T], stdout: &mut dyn Write) -> Result<()> {
        if args.is_empty() {
            return Err(Error::builtin_command(Self::usage(), 2));
        }

        let arg = args.first().unwrap();
        if arg.as_ref().starts_with('%') {
            match arg.as_ref()[1..].parse::<u32>() {
                Ok(n) => match shell.kill_background_job(n) {
                    Ok(Some(job)) => {
                        writeln!(stdout, "[{}]+\tTerminated: 15\t{}", n, job.input())
                            .context(ErrorKind::Io)?;
                        Ok(())
                    }
                    Ok(None) => Err(Error::builtin_command(
                        format!("kill: {}: no such job", arg.as_ref()),
                        1,
                    )),
                    Err(e) => Err(e),
                },
                Err(_) => Err(Error::builtin_command(
                    format!(
                        "kill: {}: arguments must be \
                         job IDs",
                        arg.as_ref()
                    ),
                    1,
                )),
            }
        } else {
            let output = Command::new("kill")
                .args(args.iter().map(AsRef::as_ref).map(OsStr::new))
                .output()
                .context(ErrorKind::Io)?;
            write!(stdout, "{}", String::from_utf8_lossy(&output.stdout)).context(ErrorKind::Io)?;
            Ok(())
        }
    }
}
