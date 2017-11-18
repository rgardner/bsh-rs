use builtins;
use errors::*;
use parser::{self, ast};
use shell::Shell;
use std::ffi::OsStr;
use std::ops::DerefMut;
use std::process::{Command, ExitStatus};

pub fn execute_command(shell: &mut Shell, command: &mut parser::Command) -> (i32, Result<()>) {
    let result = execute_command_internal(shell, command);
    // TODO
    (0, result)
}

/// note: rustfmt formatting makes function less readable
#[cfg_attr(rustfmt, rustfmt_skip)]
fn execute_command_internal(shell: &mut Shell, command: &mut parser::Command) -> Result<()> {
    let mut current = &mut command.inner;
    loop {
        // restrict scope of borrowing `current` via `{current}` (new scope)
        // solves E0506 rustc error, "cannot assign to `current` because it is borrowed"
        current = match *{current} {
            ast::Command::Simple { ref mut words, ref mut redirects, .. } => {
                execute_simple_command(shell, words);
                break;
            }
            ast::Command::Connection { ref mut first, ref mut second, .. } => {
                match *first.deref_mut() {
                    ast::Command::Simple { ref mut words, ref mut redirects, .. } => {
                        execute_simple_command(shell, words);
                    }
                    _ => unreachable!(),
                };
                &mut *second
            }
        };
    }

    Ok(())
}

fn execute_simple_command<S>(shell: &mut Shell, words: &[S]) -> (i32, Result<()>)
where
    S: AsRef<str> + AsRef<OsStr>,
{
    if builtins::is_builtin(words) {
        builtins::run(shell, words)
    } else {
        execute_external_command(words)
    }
}

fn execute_external_command<S: AsRef<OsStr>>(words: &[S]) -> (i32, Result<()>) {
    let result = execute_external_command_internal(words);
    match result {
        Ok(exit_code) => (exit_code, Ok(())),
        Err(e) => (1, Err(e)),
    }
}

fn execute_external_command_internal<S: AsRef<OsStr>>(words: &[S]) -> Result<(i32)> {
    let child = Command::new(&words[0]).args(words[1..].iter()).spawn()?;
    let output = child.wait_with_output()?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(get_status_code(&output.status))
}

#[cfg(unix)]
fn get_status_code(exit_status: &ExitStatus) -> i32 {
    match exit_status.code() {
        Some(code) => code,
        None => {
            use std::os::unix::process::ExitStatusExt;
            128 + exit_status.signal().unwrap()
        }
    }
}
