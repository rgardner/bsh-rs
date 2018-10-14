use std::process::ExitStatus;

use crate::builtins::{self, prelude::*};

pub struct Exit;

impl builtins::BuiltinCommand for Exit {
    const NAME: &'static str = builtins::EXIT_NAME;

    const HELP: &'static str = "\
exit: exit [n]
    Exit the shell with a status of N. If N is omitted, the exit status
    is 0.";

    fn run<T: AsRef<str>>(
        shell: &mut dyn Shell,
        args: &[T],
        _stdout: &mut dyn Write,
    ) -> Result<()> {
        if shell.has_background_jobs() {
            return Err(Error::builtin_command("There are stopped jobs.", 1));
        }
        let status_code = args
            .get(0)
            .map(|arg| {
                arg.as_ref().parse::<i32>().unwrap_or_else(|_| {
                    eprintln!("bsh: exit: {}: numeric argument required", arg.as_ref());
                    2
                })
            }).map(ExitStatus::from_status);
        shell.exit(status_code);
    }
}
