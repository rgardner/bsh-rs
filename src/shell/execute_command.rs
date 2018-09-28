use std::ffi::OsStr;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::iter;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{ChildStdout, Command, ExitStatus, Stdio};

use failure::{Fail, ResultExt};
use nix::libc;
use nix::sys::signal::{self, SigHandler, Signal};
use nix::unistd::{self, Pid};

use core::{intermediate_representation as ir, parser::ast};
use errors::{Error, ErrorKind, Result};
use shell::{builtins, shell::Shell};
use util::{self, BshExitStatusExt};

#[derive(Debug)]
enum Stdin {
    Inherit,
    File(File),
    Child(ChildStdout),
}

#[derive(Debug)]
enum Output {
    Inherit,
    File(File),
    CreatePipe,
}

impl Stdin {
    /// simple commands prefer file redirects to piping, following bash's behavior
    fn new(redirect: &ir::Stdio, pipe: Option<Stdin>) -> Result<Self> {
        match (redirect, pipe) {
            (ir::Stdio::FileDescriptor(fd), _) => unsafe { Ok(File::from_raw_fd(*fd).into()) },
            (ir::Stdio::Filename(filename), _) => Ok(Stdin::File(
                File::open(filename).with_context(|_| ErrorKind::Io)?,
            )),
            (_, Some(stdin)) => Ok(stdin),
            _ => Ok(Stdin::Inherit),
        }
    }
}

impl Output {
    /// simple commands prefer file redirects to piping, following bash's behavior
    fn new(redirect: &ir::Stdio, pipe: Option<Output>) -> Result<Self> {
        match (redirect, pipe) {
            (ir::Stdio::FileDescriptor(fd), _) => unsafe { Ok(File::from_raw_fd(*fd).into()) },
            (ir::Stdio::Filename(filename), _) => Ok(Output::File(
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(filename)
                    .context(ErrorKind::Io)?,
            )),
            (_, Some(output)) => Ok(output),
            _ => Ok(Output::Inherit),
        }
    }
}

impl From<File> for Stdin {
    fn from(file: File) -> Self {
        Stdin::File(file)
    }
}

impl AsRawFd for Stdin {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Stdin::Inherit => libc::STDIN_FILENO,
            Stdin::File(f) => f.as_raw_fd(),
            Stdin::Child(child) => child.as_raw_fd(),
        }
    }
}

impl From<File> for Output {
    fn from(file: File) -> Self {
        Output::File(file)
    }
}

impl From<Stdin> for Stdio {
    fn from(stdin: Stdin) -> Self {
        match stdin {
            Stdin::Inherit => Self::inherit(),
            Stdin::File(file) => file.into(),
            Stdin::Child(child) => child.into(),
        }
    }
}

impl From<Output> for Stdio {
    fn from(stdout: Output) -> Self {
        match stdout {
            Output::Inherit => Self::inherit(),
            Output::File(file) => file.into(),
            Output::CreatePipe => Self::piped(),
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
    pub fn new_builtin<S1, S2>(program: S1, args: &[S2], status_code: ExitStatus) -> Process
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Process {
            argv: iter::once(program)
                .map(|p| p.as_ref().to_string())
                .chain(args.iter().map(|arg| arg.as_ref().to_string()))
                .collect(),
            status: ProcessStatus::Completed,
            status_code: Some(status_code),
            ..Default::default()
        }
    }

    pub fn new_external<S1, S2>(program: S1, args: &[S2], id: u32) -> Process
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Process {
            argv: iter::once(&program)
                .map(|p| p.as_ref().to_string())
                .chain(args.iter().map(|arg| arg.as_ref().to_string()))
                .collect(),
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

pub struct ProcessGroup {
    pub id: Option<u32>,
    pub processes: Vec<Process>,
    pub foreground: bool,
}

/// Spawn processes for each `command`, returning processes, the process group, and a `bool`
/// representing whether the processes are running in the foreground.
pub fn spawn_processes(
    shell: &mut Shell,
    command_group: &ir::CommandGroup,
) -> Result<ProcessGroup> {
    let (processes, pgid, _) = _spawn_processes(shell, &command_group.command, None, None, None)?;
    Ok(ProcessGroup {
        id: pgid,
        processes,
        foreground: !command_group.background,
    })
}

fn _spawn_processes(
    shell: &mut Shell,
    command: &ir::Command,
    stdin: Option<Stdin>,
    stdout: Option<Output>,
    pgid: Option<u32>,
) -> Result<(Vec<Process>, Option<u32>, Option<Stdin>)> {
    match command {
        ir::Command::Simple(simple_command) => {
            let stdin = Stdin::new(&simple_command.stdin, stdin)?;
            let stdout = Output::new(&simple_command.stdout, stdout)?;
            let stderr = Output::new(&simple_command.stderr, None /*pipe*/)?;
            let (result, pgid, output) = run_simple_command(
                shell,
                &simple_command.program,
                &simple_command.args,
                stdin,
                stdout,
                stderr,
                pgid,
            )?;
            Ok((vec![result], pgid, output))
        }
        ir::Command::Connection {
            ref first,
            ref second,
            connector,
        } => run_connection_command(shell, first, second, *connector, stdin, stdout, pgid),
    }
}

fn run_simple_command<S1, S2>(
    shell: &mut Shell,
    program: S1,
    args: &[S2],
    stdin: Stdin,
    stdout: Output,
    stderr: Output,
    pgid: Option<u32>,
) -> Result<(Process, Option<u32>, Option<Stdin>)>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    if builtins::is_builtin(&program) {
        run_builtin_command(shell, program, &args, stdout, pgid)
    } else {
        run_external_command(shell, program, &args, stdin, stdout, stderr, pgid)
    }
}

fn run_connection_command(
    shell: &mut Shell,
    first: &ir::Command,
    second: &ir::Command,
    connector: ast::Connector,
    stdin: Option<Stdin>,
    stdout: Option<Output>,
    pgid: Option<u32>,
) -> Result<(Vec<Process>, Option<u32>, Option<Stdin>)> {
    match connector {
        ast::Connector::Pipe => {
            let (mut first_result, pgid, pipe) =
                _spawn_processes(shell, first, stdin, Some(Output::CreatePipe), pgid)?;
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

fn run_builtin_command<S1, S2>(
    shell: &mut Shell,
    program: S1,
    args: &[S2],
    stdout: Output,
    pgid: Option<u32>,
) -> Result<(Process, Option<u32>, Option<Stdin>)>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    // TODO(rogardn): change Result usage in builtin to only be for rust
    // errors, e.g. builtin::execute shouldn't return a Result
    let (status_code, output) = match stdout {
        Output::File(mut file) => (builtins::run(shell, &program, args, &mut file).0, None),
        Output::CreatePipe => {
            let (read_end_pipe, mut write_end_pipe) = create_pipe()?;
            (
                builtins::run(shell, &program, args, &mut write_end_pipe).0,
                Some(read_end_pipe.into()),
            )
        }
        Output::Inherit => (
            builtins::run(shell, &program, args, &mut io::stdout()).0,
            None,
        ),
    };

    let process = Process::new_builtin(&program, &args, status_code);
    Ok((process, pgid, output))
}

fn run_external_command<S1, S2>(
    shell: &Shell,
    program: S1,
    args: &[S2],
    stdin: Stdin,
    stdout: Output,
    stderr: Output,
    pgid: Option<u32>,
) -> Result<(Process, Option<u32>, Option<Stdin>)>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    let mut command = Command::new(OsStr::new(program.as_ref()));
    command.args(args.iter().map(AsRef::as_ref).map(OsStr::new));

    // Configure stdout and stderr (e.g. pipe, redirect). Do not configure
    // stdin, as we need to do that manually in before_exec *after* we have
    // set the terminal control device to the job's process group. If we were
    // to configure stdin here, then stdin would be changed before our code
    // executes in before_exec, so if the child is not the first process in the
    // pipeline, its stdin would not be a tty and tcsetpgrp would tell us so.
    command.stdout(stdout);
    command.stderr(stderr);

    let shell_is_interactive = shell.is_interactive();
    let shell_terminal = util::get_terminal();
    command.before_exec(move || {
        if shell_is_interactive {
            // Put process into process group
            let pid = unistd::getpid();
            let pgid = pgid.map(|pgid| Pid::from_raw(pgid as i32)).unwrap_or(pid);

            // setpgid(2) failing represents programmer error, e.g.
            // 1) invalid pid or pgid
            unistd::setpgid(pid, pgid).expect("setpgid failed");

            // Set the terminal control device in both parent process (see job
            // manager) and child process to avoid race conditions
            // tcsetpgrp(3) failing represents programmer error, e.g.
            // 1) invalid fd or pgid
            // 2) not a tty
            //   - Are you configuring stdin using Command::stdin? If so, then
            //     stdin will not be a TTY if this process isn't first in the
            //     pipeline, as Command::stdin configures stdin *before*
            //     before_exec runs.
            // 3) incorrect permissions
            unistd::tcsetpgrp(shell_terminal, pgid).expect("tcsetpgrp failed");

            // Reset job control signal handling back to default
            unsafe {
                // signal(3) failing represents programmer error, e.g.
                // 1) signal argument is not a valid signal number
                // 2) an attempt is made to supply a signal handler for a
                //    signal that cannot have a custom signal handler
                signal::signal(Signal::SIGINT, SigHandler::SigDfl)
                    .expect("failed to set SIGINT signal handler");
                signal::signal(Signal::SIGQUIT, SigHandler::SigDfl)
                    .expect("failed to set SIGQUIT signal handler");
                signal::signal(Signal::SIGTSTP, SigHandler::SigDfl)
                    .expect("failed to set SIGTSTP signal handler");
                signal::signal(Signal::SIGTTIN, SigHandler::SigDfl)
                    .expect("failed to set SIGTTIN signal handler");
                signal::signal(Signal::SIGTTOU, SigHandler::SigDfl)
                    .expect("failed to set SIGTTOU signal handler");
                signal::signal(Signal::SIGCHLD, SigHandler::SigDfl)
                    .expect("failed to set SIGCHLD signal handler");
            }
        }

        // See comment at the top of this function on why we are configuring
        // this manually (hint: it's because tcsetpgrp needs the original stdin
        // and Command::stdin will change stdin *before* before_exec runs).
        let stdin = stdin.as_raw_fd();
        if stdin != libc::STDIN_FILENO {
            unistd::dup2(stdin, libc::STDIN_FILENO).expect("failed to dup stdin");
            unistd::close(stdin).expect("failed to close stdin");
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
                return Err(Error::command_not_found(program));
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
        Process::new_external(program, args, child.id()),
        Some(pgid),
        child.stdout.map(Stdin::Child),
    ))
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
