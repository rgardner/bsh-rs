use builtins;
use errors::*;
use parser::{self, ast};
use shell::Shell;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::ops::DerefMut;
use std::process::{Command, ExitStatus, Stdio};

/// note: rustfmt formatting makes function less readable
#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn execute_command(shell: &mut Shell, command: &mut parser::Command) -> (i32, Result<()>) {
    let mut current = &mut command.inner;
    loop {
        // restrict scope of borrowing `current` via `{current}` (new scope)
        // solves E0506 rustc error, "cannot assign to `current` because it is borrowed"
        current = match *{current} {
            ast::Command::Simple { ref words, ref redirects, .. } => {
                return execute_simple_command(shell, words, redirects);
            }
            ast::Command::Connection { ref mut first, ref mut second, .. } => {
                match *first.deref_mut() {
                    ast::Command::Simple { ref mut words, ref mut redirects, .. } => {
                        execute_simple_command(shell, words, redirects);
                    }
                    _ => unreachable!(),
                };
                &mut *second
            }
        };
    }
}

fn execute_simple_command<S>(
    shell: &mut Shell,
    words: &[S],
    redirects: &[ast::Redirect],
) -> (i32, Result<()>)
where
    S: AsRef<str> + AsRef<OsStr>,
{
    if builtins::is_builtin(words) {
        builtins::run(shell, words)
    } else {
        execute_external_command(words, redirects)
    }
}

fn execute_external_command<S>(words: &[S], redirects: &[ast::Redirect]) -> (i32, Result<()>)
where
    S: AsRef<OsStr>,
{
    let result = execute_external_command_internal(words, redirects);
    match result {
        Ok(exit_code) => (exit_code, Ok(())),
        Err(e) => (1, Err(e)),
    }
}

fn execute_external_command_internal<S>(words: &[S], redirects: &[ast::Redirect]) -> Result<(i32)>
where
    S: AsRef<OsStr>,
{
    let mut command = Command::new(&words[0]);
    command.args(words[1..].iter());

    if let Some(&ast::Redirectee::Filename(ref filename)) = get_stdin_redirect(redirects) {
        command.stdin(Stdio::from(File::open(filename)?));
    }

    if let Some(&ast::Redirectee::Filename(ref filename)) = get_stdout_redirect(redirects) {
        let file = OpenOptions::new().write(true).create(true).open(filename)?;
        command.stdout(Stdio::from(file));
    }

    let child = command.spawn()?;
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

/// Gets the last stdin redirect in `redirects`
fn get_stdin_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirectee> {
    redirects
        .iter()
        .filter(|r| {
            if (r.instruction != ast::RedirectInstruction::Input) || (r.redirector.is_some()) {
                return false;
            }

            match r.redirectee {
                ast::Redirectee::Filename(_) => true,
                _ => false,
            }
        })
        .last()
        .map(|r| &r.redirectee)
}

/// Gets the last stdout redirect in `redirects`
fn get_stdout_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirectee> {
    redirects
        .iter()
        .filter(|r| {
            if r.instruction != ast::RedirectInstruction::Output {
                return false;
            }

            match r.redirectee {
                ast::Redirectee::Filename(_) => true,
                _ => false,
            }
        })
        .last()
        .map(|r| &r.redirectee)
}
