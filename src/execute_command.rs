use builtins;
use errors::*;
use nix::unistd;
use parser::ast;
use shell::Shell;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::FromRawFd;
use std::process::{ChildStdout, Command, Stdio};

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
    fn new(redirect: &ast::Redirect) -> Result<Self> {
        match redirect.redirectee {
            ast::Redirectee::FileDescriptor(fd) => unsafe { Ok(File::from_raw_fd(fd).into()) },
            ast::Redirectee::Filename(ref filename) => {
                match redirect.instruction {
                    ast::RedirectInstruction::Output => {
                        assert!(false, "Stdin::new called with stdout redirect");
                        unreachable!();
                    }
                    ast::RedirectInstruction::Input => Ok(Stdin::File(File::open(filename)?)),
                }
            }
        }
    }
}

impl Stdout {
    fn new(redirect: &ast::Redirect) -> Result<Self> {
        match redirect.redirectee {
            ast::Redirectee::FileDescriptor(fd) => unsafe { Ok(File::from_raw_fd(fd).into()) },
            ast::Redirectee::Filename(ref filename) => {
                match redirect.instruction {
                    ast::RedirectInstruction::Output => {
                        let file = OpenOptions::new().write(true).create(true).open(filename)?;
                        Ok(Stdout::File(file))
                    }
                    ast::RedirectInstruction::Input => {
                        assert!(false, "Stdout::new called with stdin redirect");
                        unreachable!();
                    }
                }
            }
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

#[derive(Default)]
pub struct Process {
    argv: Vec<String>,
    /// `id` is None when the process hasn't launched or the command is a Shell builtin
    id: Option<u32>,
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
    pub fn new_builtin(argv: &[String], status_code: i32) -> Process {
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

    pub fn id(&self) -> Option<u32> {
        self.id
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

impl Default for ProcessStatus {
    fn default() -> Self {
        ProcessStatus::Running
    }
}

pub fn spawn_processes(shell: &mut Shell, command: &ast::Command) -> Result<Vec<Process>> {
    Ok(_spawn_processes(shell, command, None, None)?.0)
}

/// note: rustfmt formatting makes function less readable
#[cfg_attr(rustfmt, rustfmt_skip)]
fn _spawn_processes(
    shell: &mut Shell,
    command: &ast::Command,
    stdin: Option<Stdin>,
    stdout: Option<Stdout>) -> Result<(Vec<Process>, Option<Stdin>)>
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

            let (result, output) = run_simple_command(shell, words, stdin, stdout)?;
            Ok((vec![result], output))
        }
        ast::Command::Connection { ref first, ref second, ref connector } => {
            run_connection_command(shell, first, second, connector, stdin, stdout)
        }
    }
}

fn run_simple_command(
    shell: &mut Shell,
    words: &[String],
    stdin: Stdin,
    stdout: Stdout,
) -> Result<(Process, Option<Stdin>)> {
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
        Ok((process, output))
    } else {
        run_external_command(&words[..], stdin, stdout)
    }
}

fn run_connection_command(
    shell: &mut Shell,
    first: &ast::Command,
    second: &ast::Command,
    connector: &ast::Connector,
    stdin: Option<Stdin>,
    stdout: Option<Stdout>,
) -> Result<(Vec<Process>, Option<Stdin>)> {
    match *connector {
        ast::Connector::Pipe => {
            let (mut first_result, pipe) =
                _spawn_processes(shell, first, stdin, Some(Stdout::CreatePipe))?;
            let (second_result, output) = _spawn_processes(shell, second, pipe, stdout)?;
            first_result.extend(second_result);
            Ok((first_result, output))

        }
        ast::Connector::Semicolon => {
            let (mut first_result, _) = _spawn_processes(shell, first, stdin, None)?;
            let (second_result, output) = _spawn_processes(shell, second, None, stdout)?;
            first_result.extend(second_result);
            Ok((first_result, output))
        }
    }
}

fn run_external_command(
    words: &[String],
    stdin: Stdin,
    stdout: Stdout,
) -> Result<(Process, Option<Stdin>)> {
    let child = Command::new(&words[0])
        .args(words[1..].iter())
        .stdin(stdin)
        .stdout(stdout)
        .spawn()?;
    Ok((
        Process::new_external(words, child.id()),
        child.stdout.map(Stdin::Child),
    ))
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

/// Wraps `unistd::pipe()` to return RAII structs instead of raw, owning file descriptors
/// Returns (`read_end_pipe`, `write_end_pipe`)
fn create_pipe() -> Result<(File, File)> {
    // IMPORTANT: immediately pass the RawFds returned by unistd::pipe()
    // into RAII structs (File). If the function returns before they are moved
    // into RAII structs, the fds could be leaked.
    // It is safe to call from_raw_fd here because read_end_pipe and
    // write_end_pipe are the owners of the file descriptors, meaning no one
    // else will close them out from under us.
    let (read_end_pipe, write_end_pipe) = unistd::pipe()?;
    unsafe {
        Ok((
            File::from_raw_fd(read_end_pipe),
            File::from_raw_fd(write_end_pipe),
        ))
    }
}
