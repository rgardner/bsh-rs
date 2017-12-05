use builtins;
use errors::*;
use parser::ast;
use shell::Shell;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::FromRawFd;
use std::process::{Command, ExitStatus, Stdio};

pub struct Process {
    argv: Vec<String>,
    completed: bool,
    stopped: bool,
    status_code: Option<i32>,
}

impl Process {
    pub fn new(argv: &Vec<String>) -> Process {
        Process {
            argv: argv.clone(),
            completed: false,
            stopped: false,
            status_code: None,
        }
    }

    pub fn status_code(&self) -> Option<i32> {
        self.status_code
    }
    pub fn set_status_code(&mut self, status_code: i32) {
        self.status_code = Some(status_code);
    }

}

/// note: rustfmt formatting makes function less readable
#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn spawn_processes(shell: &mut Shell, command: &ast::Command) -> Result<Vec<Process>> {
    // restrict scope of borrowing `current` via `{current}` (new scope)
    // solves E0506 rustc error, "cannot assign to `current` because it is borrowed"
    match *{command} {
        ast::Command::Simple { ref words, ref redirects, .. } => {
            Ok(vec![run_simple_command(shell, words, redirects)?])
        }
        ast::Command::Connection { ref first, ref second, ref connector } => {
            run_connection_command(shell, first, second, connector)
        }
    }
}

fn run_simple_command(
    shell: &mut Shell,
    words: &Vec<String>,
    redirects: &[ast::Redirect],
) -> Result<Process>
{
    let stdin_redirect = get_stdin_redirect(redirects);
    let stdout_redirect = get_stdout_redirect(redirects);

    // convert stdin and stdout to Stdio and return if either fails
    // i.e. Option<&Redirect> -> Option<Result<Stdio>>
    //                        -> Result<Option<Stdio>>
    //                        -> Option<Stdio>

    let stdin = stdin_redirect
        .map(to_file)
        .map_or(Ok(None), |v| v.map(Some))
        .unwrap();
    let stdout = stdout_redirect
        .map(to_file)
        .map_or(Ok(None), |v| v.map(Some))
        .unwrap();

    let mut process = Process::new(&words);
    if builtins::is_builtin(words) {
        let (status_code, _) = if let Some(mut stdout) = stdout {
            builtins::run(shell, words, &mut stdout)
        } else {
            builtins::run(shell, words, &mut io::stdout())
        };
        process.set_status_code(status_code);
    } else {
        let status_code = run_external_command(words, stdin.map(Into::into), stdout.map(Into::into))?;
        process.set_status_code(status_code);
    }

    Ok(process)
}

fn run_connection_command(
    shell: &mut Shell,
    first: &ast::Command,
    second: &ast::Command,
    connector: &ast::Connector,
) -> Result<Vec<Process>> {
    let mut result = spawn_processes(shell, first)?;
    result.extend(spawn_processes(shell, second)?);
    Ok(result)
}

fn run_external_command<S>(
    words: &[S],
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
) -> Result<i32>
where
    S: AsRef<OsStr>,
{
    let mut command = Command::new(&words[0]);
    command.args(words[1..].iter());

    if let Some(stdin) = stdin {
        command.stdin(stdin);
    }

    if let Some(stdout) = stdout {
        command.stdout(stdout);
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
fn get_stdin_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirect> {
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
}

/// Gets the last stdout redirect in `redirects`
fn get_stdout_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirect> {
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
}

fn to_file(redirect: &ast::Redirect) -> Result<File> {
    match redirect.redirectee {
        ast::Redirectee::FileDescriptor(fd) => unsafe { Ok(File::from_raw_fd(fd)) },
        ast::Redirectee::Filename(ref filename) => {
            match redirect.instruction {
                ast::RedirectInstruction::Output => {
                    Ok(OpenOptions::new().write(true).create(true).open(filename)?)
                }
                ast::RedirectInstruction::Input => Ok(File::open(filename)?),
            }
        }
    }
}
