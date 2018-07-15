use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::{ChildStdout, Command, ExitStatus, Stdio};

use failure::{Fail, ResultExt};
use nix::libc;
use nix::sys::signal::{self, SigHandler, Signal};
use nix::unistd::{self, Pid};

use builtins;
use errors::{Error, ErrorKind, Result};
use parser::ast;
use shell::Shell;
use util::{self, BshExitStatusExt};

#[derive(Debug)]
enum Stdin {
    Child(ChildStdout),
    File(File),
    Inherit,
}

#[derive(Debug)]
enum Stdout {
    File(File),
    CreatePipe,
    Inherit,
}

impl Stdin {
    /// # Panics
    /// Panics if `redirect` is not an input redirect
    fn new(redirect: &ast::Redirect) -> Result<Self> {
        match redirect.redirectee {
            ast::Redirectee::FileDescriptor(fd) => unsafe { Ok(File::from_raw_fd(fd).into()) },
            ast::Redirectee::Filename(ref filename) => match redirect.instruction {
                ast::RedirectInstruction::Output => {
                    panic!("Stdin::new called with stdout redirect");
                }
                ast::RedirectInstruction::Input => Ok(Stdin::File(
                    File::open(filename).with_context(|_| ErrorKind::Io)?
                )),
            },
        }
    }
}

impl Stdout {
    /// # Panics
    /// Panics if `redirect` is not an output redirect
    fn new(redirect: &ast::Redirect) -> Result<Self> {
        match redirect.redirectee {
            ast::Redirectee::FileDescriptor(fd) => unsafe { Ok(File::from_raw_fd(fd).into()) },
            ast::Redirectee::Filename(ref filename) => match redirect.instruction {
                ast::RedirectInstruction::Output => {
                    let file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(filename)
                        .context(ErrorKind::Io)?;
                    Ok(Stdout::File(file))
                }
                ast::RedirectInstruction::Input => {
                    panic!("Stdout::new called with stdin redirect");
                }
            },
        }
    }
}

impl From<File> for Stdin {
    fn from(file: File) -> Self {
        Stdin::File(file)
    }
}

impl From<File> for Stdout {
    fn from(file: File) -> Self {
        Stdout::File(file)
    }
}

impl From<Stdin> for Stdio {
    fn from(stdin: Stdin) -> Self {
        match stdin {
            Stdin::File(file) => file.into(),
            Stdin::Child(child) => child.into(),
            Stdin::Inherit => Self::inherit(),
        }
    }
}

impl From<Stdout> for Stdio {
    fn from(stdout: Stdout) -> Self {
        match stdout {
            Stdout::File(file) => file.into(),
            Stdout::CreatePipe => Self::piped(),
            Stdout::Inherit => Self::inherit(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Process {
    argv: Vec<String>,
    /// `id` is None when the process hasn't launched or the command is a Shell builtin
    id: Option<u32>,
    status: ProcessStatus,
    status_code: Option<ExitStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Completed,
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProcessStatus::Running => write!(f, "Running"),
            ProcessStatus::Stopped => write!(f, "Stopped"),
            ProcessStatus::Completed => write!(f, "Completed"),
        }
    }
}

impl Process {
    pub fn new_builtin(argv: &[String], status_code: ExitStatus) -> Process {
        Process {
            argv: argv.to_vec(),
            status: ProcessStatus::Completed,
            status_code: Some(status_code),
            ..Default::default()
        }
    }

    pub fn new_external(argv: &[String], id: u32) -> Process {
        Process {
            argv: argv.to_vec(),
            id: Some(id),
            ..Default::default()
        }
    }

    pub fn argv(&self) -> String {
        self.argv[..].join(" ")
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn status(&self) -> ProcessStatus {
        self.status
    }

    pub fn set_status(&mut self, status: ProcessStatus) {
        self.status = status
    }

    pub fn status_code(&self) -> Option<ExitStatus> {
        self.status_code
    }

    pub fn set_status_code(&mut self, status_code: ExitStatus) {
        self.status_code = Some(status_code);
    }

    /// # Panics
    /// Panics if job is in an invalid state
    pub fn wait(&mut self) -> Result<ExitStatus> {
        if let ProcessStatus::Completed = self.status {
            Ok(self.status_code.unwrap())
        } else if let Some(pid) = self.id {
            let status_code = wait_for_process(pid)?;
            self.status_code = Some(status_code);
            self.status = ProcessStatus::Completed;
            Ok(status_code)
        } else {
            panic!("process status is not 'Completed' and pid is not set");
        }
    }
}

impl Default for ProcessStatus {
    fn default() -> Self {
        ProcessStatus::Running
    }
}

/// Spawn processes for each `command`, returning processes, the process group, and a `bool`
/// representing whether the processes are running in the foreground.
pub fn spawn_processes(
    shell: &mut Shell,
    command: &ast::Command,
) -> Result<(Vec<Process>, Option<u32>, bool)> {
    let (processes, pgid, _) = _spawn_processes(shell, command, None, None, None)?;
    Ok((processes, pgid, true))
}

/// note: rustfmt formatting makes function less readable
#[cfg_attr(rustfmt, rustfmt_skip)]
fn _spawn_processes(
    shell: &mut Shell,
    command: &ast::Command,
    stdin: Option<Stdin>,
    stdout: Option<Stdout>,
    pgid: Option<u32>) -> Result<(Vec<Process>, Option<u32>, Option<Stdin>)>
{
    // restrict scope of borrowing `current` via `{current}` (new scope)
    // solves E0506 rustc error, "cannot assign to `current` because it is borrowed"
    match *{command} {
        ast::Command::Simple { ref words, ref redirects, .. } => {
            // simple commands prefer file redirects to piping, following bash's behavior
            let stdin_redirect = get_stdin_redirect(redirects);
            let stdout_redirect = get_stdout_redirect(redirects);

            // convert stdin and stdout to Stdin/Stdout and return if either fails
            // i.e. Option<&Redirect> -> Option<Result<Stdin>>
            //                        -> Result<Option<Stdin>>
            //                        -> Option<Stdin>

            let stdin = stdin_redirect
                .map(Stdin::new)
                .map_or(Ok(None), |v| v.map(Some))?
                .or(stdin)
                .unwrap_or(Stdin::Inherit);
            let stdout = stdout_redirect
                .map(Stdout::new)
                .map_or(Ok(None), |v| v.map(Some))?
                .or(stdout)
                .unwrap_or(Stdout::Inherit);

            let (result, pgid, output) = run_simple_command(shell, words, stdin, stdout, pgid)?;
            Ok((vec![result], pgid, output))
        }
        ast::Command::Connection { ref first, ref second, ref connector } => {
            run_connection_command(shell, first, second, connector, stdin, stdout, pgid)
        }
    }
}

fn run_simple_command(
    shell: &mut Shell,
    words: &[String],
    stdin: Stdin,
    stdout: Stdout,
    pgid: Option<u32>,
) -> Result<(Process, Option<u32>, Option<Stdin>)> {
    if builtins::is_builtin(words) {
        // TODO(rogardn): change Result usage in builtin to only be for rust
        // errors, e.g. builtin::execute shouldn't return a Result
        let (status_code, output) = match stdout {
            Stdout::File(mut file) => (builtins::run(shell, words, &mut file).0, None),
            Stdout::CreatePipe => {
                let (read_end_pipe, mut write_end_pipe) = create_pipe()?;
                (
                    builtins::run(shell, words, &mut write_end_pipe).0,
                    Some(read_end_pipe.into()),
                )
            }
            Stdout::Inherit => (builtins::run(shell, words, &mut io::stdout()).0, None),
        };

        let process = Process::new_builtin(words, status_code);
        Ok((process, pgid, output))
    } else {
        run_external_command(shell, &words[..], stdin, stdout, pgid)
    }
}

fn run_connection_command(
    shell: &mut Shell,
    first: &ast::Command,
    second: &ast::Command,
    connector: &ast::Connector,
    stdin: Option<Stdin>,
    stdout: Option<Stdout>,
    pgid: Option<u32>,
) -> Result<(Vec<Process>, Option<u32>, Option<Stdin>)> {
    match *connector {
        ast::Connector::Pipe => {
            let (mut first_result, pgid, pipe) =
                _spawn_processes(shell, first, stdin, Some(Stdout::CreatePipe), pgid)?;
            let (second_result, pgid, output) =
                _spawn_processes(shell, second, pipe, stdout, pgid)?;
            first_result.extend(second_result);
            Ok((first_result, pgid, output))
        }
        ast::Connector::Semicolon => {
            let (mut first_result, _, _) = _spawn_processes(shell, first, stdin, None, pgid)?;
            first_result.last_mut().unwrap().wait()?;
            let (second_result, pgid, output) =
                _spawn_processes(shell, second, None, stdout, None)?;
            first_result.extend(second_result);
            Ok((first_result, pgid, output))
        }
        ast::Connector::And => {
            let (mut first_result, _, _) = _spawn_processes(shell, first, stdin, None, pgid)?;
            let (pgid, output) = if first_result.last_mut().unwrap().wait()?.success() {
                let (second_result, pgid, output) =
                    _spawn_processes(shell, second, None, stdout, None)?;
                first_result.extend(second_result);
                (pgid, output)
            } else {
                (None, None)
            };
            Ok((first_result, pgid, output))
        }
        ast::Connector::Or => {
            let (mut first_result, _, _) = _spawn_processes(shell, first, stdin, None, pgid)?;
            let (pgid, output) = if !first_result.last_mut().unwrap().wait()?.success() {
                let (second_result, pgid, output) =
                    _spawn_processes(shell, second, None, stdout, None)?;
                first_result.extend(second_result);
                (pgid, output)
            } else {
                (None, None)
            };
            Ok((first_result, pgid, output))
        }
    }
}

fn run_external_command(
    shell: &Shell,
    words: &[String],
    stdin: Stdin,
    stdout: Stdout,
    pgid: Option<u32>,
) -> Result<(Process, Option<u32>, Option<Stdin>)> {
    let mut command = Command::new(&words[0]);
    command.args(words[1..].iter());
    command.stdin(stdin);
    command.stdout(stdout);

    let shell_is_interactive = shell.is_interactive();
    command.before_exec(move || {
        if shell_is_interactive {
            // Put process into process group
            let pid = unistd::getpid();
            let pgid = pgid.map(|pgid| Pid::from_raw(pgid as i32)).unwrap_or(pid);

            // setpgid(2) failing represents programmer error, e.g.
            // 1) invalid pid or pgid
            unistd::setpgid(pid, pgid).unwrap();

            // tcsetpgrp(3) failing represents programmer error, e.g.
            // 1) invalid fd or pgid
            // 2) not a tty
            // 3) incorrect permissions
            unistd::tcsetpgrp(util::get_terminal(), pgid).unwrap();

            // Reset job control signal handling back to default
            unsafe {
                // signal(3) failing represents programmer error, e.g.
                // 1) signal argument is not a valid signal number
                // 2) an attempt is made to supply a signal handler for a
                //    signal that cannot have a custom signal handler
                signal::signal(Signal::SIGINT, SigHandler::SigDfl).unwrap();
                signal::signal(Signal::SIGQUIT, SigHandler::SigDfl).unwrap();
                signal::signal(Signal::SIGTSTP, SigHandler::SigDfl).unwrap();
                signal::signal(Signal::SIGTTIN, SigHandler::SigDfl).unwrap();
                signal::signal(Signal::SIGTTOU, SigHandler::SigDfl).unwrap();
                signal::signal(Signal::SIGCHLD, SigHandler::SigDfl).unwrap();
            }
        }
        Ok(())
    });

    let child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            if shell_is_interactive {
                warn!("failed to spawn child, resetting terminal's pgrp");
                // see above comment for tcsetpgrp(2) failing being programmer
                // error
                unistd::tcsetpgrp(util::get_terminal(), unistd::getpgrp()).unwrap();
            }

            if e.kind() == io::ErrorKind::NotFound {
                return Err(Error::command_not_found(&words[0]));
            } else {
                return Err(e.context(ErrorKind::Io).into());
            }
        }
    };

    let pgid = pgid.unwrap_or_else(|| child.id());
    let temp_result = unistd::setpgid(
        Pid::from_raw(child.id() as libc::pid_t),
        Pid::from_raw(pgid as libc::pid_t),
    );
    log_if_err!(
        temp_result,
        "failed to set pgid ({}) for pid ({})",
        child.id(),
        pgid
    );

    Ok((
        Process::new_external(words, child.id()),
        Some(pgid),
        child.stdout.map(Stdin::Child),
    ))
}

/// Gets the last stdin redirect in `redirects`
fn get_stdin_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirect> {
    redirects
        .iter()
        .rev()
        .filter(|r| {
            if (r.instruction != ast::RedirectInstruction::Input) || (r.redirector.is_some()) {
                return false;
            }

            match r.redirectee {
                ast::Redirectee::Filename(_) => true,
                _ => false,
            }
        })
        .nth(0)
}

/// Gets the last stdout redirect in `redirects`
fn get_stdout_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirect> {
    redirects
        .iter()
        .rev()
        .filter(|r| {
            if r.instruction != ast::RedirectInstruction::Output {
                return false;
            }

            match r.redirectee {
                ast::Redirectee::Filename(_) => true,
                _ => false,
            }
        })
        .nth(0)
}

/// Wraps `unistd::pipe()` to return RAII structs instead of raw, owning file descriptors
/// Returns (`read_end_pipe`, `write_end_pipe`)
fn create_pipe() -> Result<(File, File)> {
    // IMPORTANT: immediately pass the RawFds returned by unistd::pipe()
    // into RAII structs (File). If the function returns before they are moved
    // into RAII structs, the fds could be leaked.
    // It is safe to call from_raw_fd here because read_end_pipe and
    // write_end_pipe are the owners of the file descriptors, meaning no one
    // else will close them out from under us.
    let (read_end_pipe, write_end_pipe) = unistd::pipe().context(ErrorKind::Nix)?;
    unsafe {
        Ok((
            File::from_raw_fd(read_end_pipe),
            File::from_raw_fd(write_end_pipe),
        ))
    }
}

fn wait_for_process(pid: u32) -> Result<ExitStatus> {
    use nix::sys::wait::{self, WaitStatus};
    use nix::unistd::Pid;

    let pid = Pid::from_raw(pid as i32);
    let wait_status = wait::waitpid(pid, None).context(ErrorKind::Nix)?;
    match wait_status {
        WaitStatus::Exited(_, status) => Ok(ExitStatus::from_status(status)),
        WaitStatus::Signaled(_, signal, _) => Ok(ExitStatus::from_status(128 + signal as i32)),
        _ => panic!("not sure what to do here"),
    }
}
