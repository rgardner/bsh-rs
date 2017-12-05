use builtins;
use errors::*;
use parser::ast;
use shell::Shell;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::FromRawFd;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct Process {
    argv: Vec<String>,
    /// `pid` is None when the process hasn't launched or the command is a Shell builtin
    pid: Option<u32>,
    status: ProcessStatus,
    status_code: Option<i32>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Completed,
}

impl Process {
    pub fn new_builtin(argv: &[String]) -> Process {
        Process {
            argv: argv.to_vec(),
            ..Default::default()
        }
    }

    pub fn new_external(argv: &[String], pid: u32) -> Process {
        Process {
            argv: argv.to_vec(),
            pid: Some(pid),
            ..Default::default()
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn status(&self) -> ProcessStatus {
        self.status
    }

    pub fn set_status(&mut self, status: ProcessStatus) {
        self.status = status
    }

    pub fn status_code(&self) -> Option<i32> {
        self.status_code
    }

    pub fn set_status_code(&mut self, status_code: i32) {
        self.status_code = Some(status_code);
    }
}

impl Default for Process {
    fn default() -> Self {
        Process {
            argv: Vec::new(),
            pid: None,
            status: ProcessStatus::Running,
            status_code: None,
        }
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
) -> Result<Process> {
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

    if builtins::is_builtin(words) {
        let mut process = Process::new_builtin(words);
        let (status_code, _) = if let Some(mut stdout) = stdout {
            builtins::run(shell, words, &mut stdout)
        } else {
            builtins::run(shell, words, &mut io::stdout())
        };
        process.set_status_code(status_code);
        Ok(process)
    } else {
        run_external_command(words, stdin.map(Into::into), stdout.map(Into::into))
    }
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

fn run_external_command(
    words: &Vec<String>,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
) -> Result<Process> {
    let mut command = Command::new(&words[0]);
    command.args(words[1..].iter());

    if let Some(stdin) = stdin {
        command.stdin(stdin);
    }
    if let Some(stdout) = stdout {
        command.stdout(stdout);
    }

    let child = command.spawn()?;

    Ok(Process::new_external(words, child.id()))
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
