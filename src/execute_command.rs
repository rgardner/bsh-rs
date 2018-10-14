use std::ffi::OsStr;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::iter;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};

use failure::{Fail, ResultExt};

use crate::{
    builtins,
    core::{intermediate_representation as ir, parser::ast},
    errors::{Error, ErrorKind, Result},
    shell::Shell,
};

#[derive(Debug)]
pub enum Stdin {
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
    #[cfg(unix)]
    fn new(redirect: &ir::Stdio, pipe: Option<Stdin>) -> Result<Self> {
        use std::os::unix::io::FromRawFd;

        match (redirect, pipe) {
            (ir::Stdio::FileDescriptor(0), _) => Ok(Stdin::Inherit),
            (ir::Stdio::FileDescriptor(fd), _) => unsafe { Ok(File::from_raw_fd(*fd).into()) },
            (ir::Stdio::Filename(filename), _) => Ok(Stdin::File(
                File::open(filename).with_context(|_| ErrorKind::Io)?,
            )),
            (_, Some(stdin)) => Ok(stdin),
            _ => Ok(Stdin::Inherit),
        }
    }

    #[cfg(windows)]
    fn new(redirect: &ir::Stdio, pipe: Option<Stdin>) -> Result<Self> {
        match (redirect, pipe) {
            (ir::Stdio::FileDescriptor(0), _) => Ok(Stdin::Inherit),
            (ir::Stdio::FileDescriptor(_fd), _) => unimplemented!(),
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
    #[cfg(unix)]
    fn new(redirect: &ir::Stdio, pipe: Option<Output>) -> Result<Self> {
        use std::os::unix::io::FromRawFd;

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

    #[cfg(windows)]
    fn new(redirect: &ir::Stdio, pipe: Option<Output>) -> Result<Self> {
        match (redirect, pipe) {
            (ir::Stdio::FileDescriptor(_fd), _) => unimplemented!(),
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

#[cfg(unix)]
impl AsRawFd for Stdin {
    fn as_raw_fd(&self) -> RawFd {
        use libc;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessId(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Completed,
}

pub trait Process {
    fn id(&self) -> Option<ProcessId>;
    fn argv(&self) -> String;
    fn status(&self) -> ProcessStatus;
    fn status_code(&self) -> Option<ExitStatus>;
    fn stdout(&mut self) -> Option<Stdin>;
    fn kill(&mut self) -> Result<()>;
    fn wait(&mut self) -> Result<ExitStatus>;
    fn try_wait(&mut self) -> Result<Option<ExitStatus>>;
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Process {{ id: {} }}",
            self.id()
                .map(|id| id.0.to_string())
                .unwrap_or_else(|| "(builtin)".to_string())
        )
    }
}

#[derive(Debug)]
pub struct ProcessGroup {
    pub id: Option<u32>,
    pub processes: Vec<Box<Process>>,
    pub foreground: bool,
}

struct BuiltinProcess {
    argv: Vec<String>,
    status_code: ExitStatus,
    stdout: Option<Stdin>,
}

impl BuiltinProcess {
    pub fn new<S1, S2>(
        program: S1,
        args: &[S2],
        status_code: ExitStatus,
        stdout: Option<Stdin>,
    ) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Self {
            argv: iter::once(program)
                .map(|p| p.as_ref().to_string())
                .chain(args.iter().map(|arg| arg.as_ref().to_string()))
                .collect(),
            status_code,
            stdout,
        }
    }
}

impl Process for BuiltinProcess {
    fn id(&self) -> Option<ProcessId> {
        None
    }

    fn argv(&self) -> String {
        self.argv[..].join(" ")
    }

    fn status(&self) -> ProcessStatus {
        ProcessStatus::Completed
    }

    fn status_code(&self) -> Option<ExitStatus> {
        Some(self.status_code)
    }

    fn stdout(&mut self) -> Option<Stdin> {
        self.stdout.take()
    }

    fn kill(&mut self) -> Result<()> {
        Ok(())
    }

    fn wait(&mut self) -> Result<ExitStatus> {
        Ok(self.status_code)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        Ok(Some(self.status_code))
    }
}

struct ExternalProcess {
    argv: Vec<String>,
    child: Child,
    status: ProcessStatus,
    status_code: Option<ExitStatus>,
}

impl ExternalProcess {
    pub fn new<S1, S2>(program: S1, args: &[S2], child: Child) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Self {
            argv: iter::once(&program)
                .map(|p| p.as_ref().to_string())
                .chain(args.iter().map(|arg| arg.as_ref().to_string()))
                .collect(),
            child,
            status: ProcessStatus::Running,
            status_code: None,
        }
    }
}

impl Process for ExternalProcess {
    fn id(&self) -> Option<ProcessId> {
        Some(self.child.id().into())
    }

    fn argv(&self) -> String {
        self.argv[..].join(" ")
    }

    fn status(&self) -> ProcessStatus {
        self.status
    }

    fn status_code(&self) -> Option<ExitStatus> {
        self.status_code
    }

    fn stdout(&mut self) -> Option<Stdin> {
        self.child.stdout.take().map(Stdin::Child)
    }

    fn kill(&mut self) -> Result<()> {
        self.child.kill().context(ErrorKind::Io)?;
        Ok(())
    }

    fn wait(&mut self) -> Result<ExitStatus> {
        let exit_status = self.child.wait().context(ErrorKind::Io)?;
        self.status = ProcessStatus::Completed;
        self.status_code = Some(exit_status);
        Ok(exit_status)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        if let Some(exit_status) = self.child.try_wait().context(ErrorKind::Io)? {
            self.status = ProcessStatus::Completed;
            self.status_code = Some(exit_status);
            Ok(Some(exit_status))
        } else {
            Ok(None)
        }
    }
}

impl From<u32> for ProcessId {
    fn from(value: u32) -> Self {
        ProcessId(value)
    }
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

/// Spawn processes for each `command`, returning processes, the process group, and a `bool`
/// representing whether the processes are running in the foreground.
pub fn spawn_processes(
    shell: &mut Shell,
    command_group: &ir::CommandGroup,
) -> Result<ProcessGroup> {
    let (processes, pgid) = _spawn_processes(shell, &command_group.command, None, None, None)?;
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
) -> Result<(Vec<Box<Process>>, Option<u32>)> {
    match command {
        ir::Command::Simple(simple_command) => {
            let stdin = Stdin::new(&simple_command.stdin, stdin)?;
            let stdout = Output::new(&simple_command.stdout, stdout)?;
            let stderr = Output::new(&simple_command.stderr, None /*pipe*/)?;
            let (result, pgid) = run_simple_command(
                shell,
                &simple_command.program,
                &simple_command.args,
                stdin,
                stdout,
                stderr,
                pgid,
            )?;
            Ok((vec![result], pgid))
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
) -> Result<(Box<Process>, Option<u32>)>
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
) -> Result<(Vec<Box<Process>>, Option<u32>)> {
    match connector {
        ast::Connector::Pipe => {
            let (mut first_result, pgid) =
                _spawn_processes(shell, first, stdin, Some(Output::CreatePipe), pgid)?;
            let (second_result, pgid) = _spawn_processes(
                shell,
                second,
                first_result.last_mut().unwrap().stdout(),
                stdout,
                pgid,
            )?;
            first_result.extend(second_result);
            Ok((first_result, pgid))
        }
        ast::Connector::Semicolon => {
            let (mut first_result, _) = _spawn_processes(shell, first, stdin, None, pgid)?;
            first_result.last_mut().unwrap().wait()?;
            let (second_result, pgid) = _spawn_processes(shell, second, None, stdout, None)?;
            first_result.extend(second_result);
            Ok((first_result, pgid))
        }
        ast::Connector::And => {
            let (mut first_result, _) = _spawn_processes(shell, first, stdin, None, pgid)?;
            first_result.last_mut().unwrap().wait()?;
            let pgid = if first_result
                .last()
                .unwrap()
                .status_code()
                .unwrap()
                .success()
            {
                let (second_result, pgid) = _spawn_processes(shell, second, None, stdout, None)?;
                first_result.extend(second_result);
                pgid
            } else {
                None
            };
            Ok((first_result, pgid))
        }
        ast::Connector::Or => {
            let (mut first_result, _) = _spawn_processes(shell, first, stdin, None, pgid)?;
            first_result.last_mut().unwrap().wait()?;
            let pgid = if !first_result
                .last()
                .unwrap()
                .status_code()
                .unwrap()
                .success()
            {
                let (second_result, pgid) = _spawn_processes(shell, second, None, stdout, None)?;
                first_result.extend(second_result);
                pgid
            } else {
                None
            };
            Ok((first_result, pgid))
        }
    }
}

fn run_builtin_command<S1, S2>(
    shell: &mut Shell,
    program: S1,
    args: &[S2],
    stdout: Output,
    pgid: Option<u32>,
) -> Result<(Box<Process>, Option<u32>)>
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

    Ok((
        Box::new(BuiltinProcess::new(&program, &args, status_code, output)),
        pgid,
    ))
}

#[cfg(unix)]
fn run_external_command<S1, S2>(
    shell: &Shell,
    program: S1,
    args: &[S2],
    stdin: Stdin,
    stdout: Output,
    stderr: Output,
    pgid: Option<u32>,
) -> Result<(Box<Process>, Option<u32>)>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    use std::os::unix::process::CommandExt;

    use libc;
    use nix::{
        sys::signal::{self, SigHandler, Signal},
        unistd::{self, Pid},
    };

    use crate::util;

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

    let job_control_is_enabled = shell.is_job_control_enabled();
    let shell_terminal = util::unix::get_terminal();
    command.before_exec(move || {
        if job_control_is_enabled {
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
            if job_control_is_enabled {
                warn!("failed to spawn child, resetting terminal's pgrp");
                // see above comment for tcsetpgrp(2) failing being programmer
                // error
                unistd::tcsetpgrp(util::unix::get_terminal(), unistd::getpgrp()).unwrap();
            }

            if e.kind() == io::ErrorKind::NotFound {
                return Err(Error::command_not_found(program));
            } else {
                return Err(e.context(ErrorKind::Io).into());
            }
        }
    };

    let pgid = pgid.unwrap_or_else(|| child.id());
    if job_control_is_enabled {
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
    }

    Ok((
        Box::new(ExternalProcess::new(program, args, child)),
        Some(pgid),
    ))
}

#[cfg(windows)]
fn run_external_command<S1, S2>(
    _shell: &Shell,
    program: S1,
    args: &[S2],
    stdin: Stdin,
    stdout: Output,
    stderr: Output,
    pgid: Option<u32>,
) -> Result<(Box<Process>, Option<u32>)>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    let mut command = Command::new(OsStr::new(program.as_ref()));
    command.args(args.iter().map(AsRef::as_ref).map(OsStr::new));
    command.stdin(stdin);
    command.stdout(stdout);
    command.stderr(stderr);

    let child = command.spawn().map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            Error::command_not_found(&program)
        } else {
            e.context(ErrorKind::Io).into()
        }
    })?;

    let pgid = pgid.unwrap_or_else(|| child.id());
    Ok((
        Box::new(ExternalProcess::new(program, args, child)),
        Some(pgid),
    ))
}

/// Wraps `unistd::pipe()` to return RAII structs instead of raw, owning file descriptors
/// Returns (`read_end_pipe`, `write_end_pipe`)
#[cfg(unix)]
fn create_pipe() -> Result<(File, File)> {
    use std::os::unix::io::FromRawFd;

    use nix::unistd;

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

#[cfg(windows)]
fn create_pipe() -> Result<(File, File)> {
    // TODO (#22): Support Windows
    // See CreatePipe, HANDLE, and "impl FromRawHandle for File"
    unimplemented!()
}
