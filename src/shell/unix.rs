//! The JobControlShell can run command groups in the foreground and background,
//! in addition to the normal shell abilities such as managing the command
//! history.

use std::env;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{self, ExitStatus};

use atty::{self, Stream};
use dirs;
use failure::ResultExt;
use libc;
use log::{debug, error, info, warn};
use nix::{
    sys::{
        signal::{self, SigHandler, Signal},
        termios::{self, Termios},
    },
    unistd::{self, Pid},
};

use super::{
    Job, JobId, Shell, ShellConfig, COMMAND_NOT_FOUND_EXIT_STATUS, HISTORY_FILE_NAME,
    SYNTAX_ERROR_EXIT_STATUS,
};
use crate::{
    core::{intermediate_representation as ir, parser::Command, variable_expansion},
    editor::Editor,
    errors::{Error, ErrorKind, Result},
    execute_command::{spawn_processes, Process, ProcessGroup, ProcessStatus},
    util::{self, BshExitStatusExt},
};

pub struct JobControlShell {
    /// Responsible for readline and history.
    editor: Editor,
    history_file: Option<PathBuf>,
    job_manager: JobManager,
    /// Exit status of last command executed.
    last_exit_status: ExitStatus,
    config: ShellConfig,
    /// Is `false` if the shell is running a script or if initializing job
    /// control fails.
    is_interactive: bool,
}

impl JobControlShell {
    /// Constructs a new JobControlShell to manage running jobs and command history.
    pub fn new(config: ShellConfig) -> Result<Self> {
        let mut shell = Self {
            editor: Editor::with_capacity(config.command_history_capacity),
            history_file: None,
            job_manager: Default::default(),
            last_exit_status: ExitStatus::from_success(),
            config,
            is_interactive: atty::is(Stream::Stdin),
        };

        if shell.is_interactive {
            let result = initialize_job_control();
            if let Err(e) = result {
                error!(
                    "failed to initialize shell for job control despite isatty: {}",
                    e
                );
                shell.is_interactive = false;
            }
        }

        if config.enable_command_history {
            shell.load_history()?
        }

        info!("bsh started up");
        Ok(shell)
    }

    fn load_history(&mut self) -> Result<()> {
        self.history_file = dirs::home_dir().map(|p| p.join(HISTORY_FILE_NAME));
        if let Some(ref history_file) = self.history_file {
            self.editor.load_history(&history_file).or_else(|e| {
                if let ErrorKind::HistoryFileNotFound = *e.kind() {
                    return Ok(());
                }

                Err(e)
            })?;
        } else {
            warn!("unable to get home directory")
        }

        Ok(())
    }

    /// Custom prompt to output to the user.
    /// Returns `None` when end of file is reached.
    fn prompt(&mut self) -> Result<Option<String>> {
        let cwd = env::current_dir().unwrap();
        let home = dirs::home_dir().unwrap();
        let rel = match cwd.strip_prefix(&home) {
            Ok(rel) => Path::new("~").join(rel),
            Err(_) => cwd.clone(),
        };

        let prompt = format!(
            "{}|{}\n$ ",
            self.last_exit_status.code().unwrap(),
            rel.display()
        );
        let line = self.editor.readline(&prompt)?;
        Ok(line)
    }

    /// Runs a job.
    fn execute_command(&mut self, command_group: &mut ir::CommandGroup) -> Result<()> {
        let process_group = match spawn_processes(self, command_group) {
            Ok(process_group) => Ok(process_group),
            Err(e) => {
                if let ErrorKind::CommandNotFound(ref command) = *e.kind() {
                    eprintln!("bsh: {}: command not found", command);
                    self.last_exit_status = ExitStatus::from_status(COMMAND_NOT_FOUND_EXIT_STATUS);
                    return Ok(());
                }

                Err(e)
            }
        }?;

        let foreground = process_group.foreground;
        let job_id = self
            .job_manager
            .create_job(&command_group.input, process_group);
        if !self.is_interactive() {
            self.last_exit_status = self.job_manager.wait_for_job(job_id)?.unwrap();
        } else if foreground {
            self.last_exit_status = self
                .job_manager
                .put_job_in_foreground(Some(job_id), false /* cont */)?
                .unwrap();
        } else {
            self.job_manager
                .put_job_in_background(Some(job_id), false /* cont */)?;
        }
        Ok(())
    }
}

impl Shell for JobControlShell {
    fn execute_command_string(&mut self, input: &str) -> Result<()> {
        // skip if empty
        if input.is_empty() {
            return Ok(());
        }

        let mut command = input.to_owned();
        if self.config.enable_command_history {
            self.editor.expand_history(&mut command)?;
            self.editor.add_history_entry(input);
        }

        let command = match Command::parse(input) {
            Ok(command) => Ok(command),
            Err(e) => {
                if let ErrorKind::Syntax(ref line) = *e.kind() {
                    eprintln!("bsh: syntax error near: {}", line);
                    self.last_exit_status = ExitStatus::from_status(SYNTAX_ERROR_EXIT_STATUS);
                    return Ok(());
                }

                Err(e)
            }
        }?;

        let inner_command =
            variable_expansion::expand_variables(&command.inner, dirs::home_dir(), env::vars());
        let mut command_group = ir::Interpreter::parse(Command::new(&command.input, inner_command));
        self.execute_command(&mut command_group)?;

        Ok(())
    }

    fn execute_commands_from_file(&mut self, path: &Path) -> Result<()> {
        use std::io::Read;
        let mut f = File::open(path).context(ErrorKind::Io)?;
        let mut buffer = String::new();
        f.read_to_string(&mut buffer)
            .with_context(|_| ErrorKind::Io)?;

        for line in buffer.split('\n') {
            self.execute_command_string(line)?
        }

        Ok(())
    }

    fn execute_from_stdin(&mut self) {
        loop {
            if self.config.enable_job_control {
                // Check the status of background jobs, removing exited ones.
                self.job_manager.do_job_notification();
            }

            let input = match self.prompt() {
                Ok(Some(line)) => line.trim().to_owned(),
                Ok(None) => break,
                e => {
                    log_if_err!(e, "prompt");
                    continue;
                }
            };

            let temp_result = self.execute_command_string(&input);
            log_if_err!(temp_result, "execute_command_string");
        }
    }

    fn exit(&mut self, n: Option<ExitStatus>) -> ! {
        if self.config.display_messages {
            println!("exit");
        }

        let code = match n {
            Some(n) => n.code().unwrap(),
            None => self.last_exit_status.code().unwrap(),
        };
        let code_like_u8 = if code < 0 {
            (256 + code) % 256
        } else {
            code % 256
        };

        if self.config.enable_command_history {
            if let Some(ref history_file) = self.history_file {
                if let Err(e) = self.editor.save_history(&history_file) {
                    error!(
                        "error: failed to save history to file during shutdown: {}",
                        e
                    );
                }
            }
        }

        info!("bsh has shut down");
        process::exit(code_like_u8);
    }

    fn is_interactive(&self) -> bool {
        self.is_interactive
    }

    fn is_job_control_enabled(&self) -> bool {
        self.is_interactive
    }

    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn get_jobs(&self) -> Vec<&dyn Job> {
        self.job_manager.get_jobs()
    }

    fn has_background_jobs(&self) -> bool {
        self.job_manager.has_jobs()
    }

    fn put_job_in_foreground(&mut self, job_id: Option<JobId>) -> Result<Option<ExitStatus>> {
        self.job_manager
            .put_job_in_foreground(job_id, true /* cont */)
    }

    fn put_job_in_background(&mut self, job_id: Option<JobId>) -> Result<()> {
        self.job_manager
            .put_job_in_background(job_id, true /* cont */)
    }

    fn kill_background_job(&mut self, job_id: u32) -> Result<Option<&dyn Job>> {
        self.job_manager.kill_job(JobId(job_id))
    }
}

impl fmt::Debug for JobControlShell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} jobs\n{:?}", self.job_manager, self.editor)
    }
}

pub fn create_shell(config: ShellConfig) -> Result<Box<dyn Shell>> {
    let shell = JobControlShell::new(config)?;
    Ok(Box::new(shell))
}

fn initialize_job_control() -> Result<()> {
    let shell_terminal = util::unix::get_terminal();

    // Loop until the shell is in the foreground
    loop {
        let shell_pgid = unistd::getpgrp();
        if unistd::tcgetpgrp(shell_terminal).context(ErrorKind::Nix)? == shell_pgid {
            break;
        } else {
            signal::kill(
                Pid::from_raw(-libc::pid_t::from(shell_pgid)),
                Signal::SIGTTIN,
            )
            .unwrap();
        }
    }

    // Ignore interactive and job-control signals
    unsafe {
        signal::signal(Signal::SIGINT, SigHandler::SigIgn).unwrap();
        signal::signal(Signal::SIGQUIT, SigHandler::SigIgn).unwrap();
        signal::signal(Signal::SIGTSTP, SigHandler::SigIgn).unwrap();
        signal::signal(Signal::SIGTTIN, SigHandler::SigIgn).unwrap();
        signal::signal(Signal::SIGTTOU, SigHandler::SigIgn).unwrap();
    }

    // Put outselves in our own process group
    let shell_pgid = Pid::this();
    unistd::setpgid(shell_pgid, shell_pgid).context(ErrorKind::Nix)?;

    // Grab control of the terminal and save default terminal attributes
    let shell_terminal = util::unix::get_terminal();
    let temp_result = unistd::tcsetpgrp(shell_terminal, shell_pgid);
    log_if_err!(temp_result, "failed to grab control of terminal");

    Ok(())
}

trait AsJob {
    fn as_job(&self) -> &dyn Job;
}

impl<T: Job> AsJob for T {
    fn as_job(&self) -> &dyn Job {
        self
    }
}

#[derive(Copy, Clone, Debug)]
pub enum JobStatus {
    Running,
    Stopped,
    Completed,
}

trait JobExt: Job {
    fn tmodes(&self) -> &Option<Termios>;
    fn status(&self) -> JobStatus;
}

#[derive(Default)]
pub struct JobManager {
    jobs: Vec<JobImpl>,
    job_count: u32,
    current_job: Option<JobId>,
}

impl JobManager {
    pub fn create_job(&mut self, input: &str, process_group: ProcessGroup) -> JobId {
        let job_id = self.get_next_job_id();
        self.jobs.push(JobImpl::new(
            job_id,
            input,
            process_group.id.map(|pgid| pgid as libc::pid_t),
            process_group.processes,
        ));
        job_id
    }

    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    pub fn get_jobs(&self) -> Vec<&dyn Job> {
        self.jobs.iter().map(|j| j.as_job()).collect()
    }

    /// Waits for job to stop or complete.
    ///
    /// This function also updates the statuses of other jobs if we receive
    /// a signal for one of their processes.
    pub fn wait_for_job(&mut self, job_id: JobId) -> Result<Option<ExitStatus>> {
        while self.job_is_running(job_id) {
            for job in &mut self.jobs {
                job.try_wait()?;
            }
        }

        let job_index = self.find_job(job_id).expect("job not found");
        Ok(self.jobs[job_index].last_status_code())
    }

    pub fn put_job_in_foreground(
        &mut self,
        job_id: Option<JobId>,
        cont: bool,
    ) -> Result<Option<ExitStatus>> {
        let job_id = job_id
            .or(self.current_job)
            .ok_or_else(|| Error::no_such_job("current"))?;
        debug!("putting job [{}] in foreground", job_id);

        let _terminal_state = {
            let job_index = self
                .find_job(job_id)
                .ok_or_else(|| Error::no_such_job(format!("{}", job_id)))?;
            self.jobs[job_index].set_last_running_in_foreground(true);
            let job_pgid = self.jobs[job_index].pgid();
            let job_tmodes = self.jobs[job_index].tmodes().clone();
            let _terminal_state = job_pgid.map(|pgid| TerminalState::new(Pid::from_raw(pgid)));

            // Send the job a continue signal if necessary
            if cont {
                if let Some(ref tmodes) = job_tmodes {
                    let temp_result = termios::tcsetattr(
                        util::unix::get_terminal(),
                        termios::SetArg::TCSADRAIN,
                        tmodes,
                    );
                    log_if_err!(
                        temp_result,
                        "error setting terminal configuration for job ({})",
                        job_id
                    );
                }
                if let Some(ref pgid) = job_pgid {
                    signal::kill(Pid::from_raw(-pgid), Signal::SIGCONT).context(ErrorKind::Nix)?;
                }
            }
            _terminal_state
        };
        self.wait_for_job(job_id)
    }

    pub fn put_job_in_background(&mut self, job_id: Option<JobId>, cont: bool) -> Result<()> {
        let job_id = job_id
            .or(self.current_job)
            .ok_or_else(|| Error::no_such_job("current"))?;
        debug!("putting job [{}] in background", job_id);

        let job_pgid = {
            let job_index = self
                .find_job(job_id)
                .ok_or_else(|| Error::no_such_job(format!("{}", job_id)))?;
            self.jobs[job_index].set_last_running_in_foreground(false);
            self.jobs[job_index].pgid()
        };

        if cont {
            if let Some(ref pgid) = job_pgid {
                signal::kill(Pid::from_raw(-pgid), Signal::SIGCONT).context(ErrorKind::Nix)?;
            }
        }

        self.current_job = Some(job_id);
        Ok(())
    }

    pub fn kill_job(&mut self, job_id: JobId) -> Result<Option<&dyn Job>> {
        if let Some(job_index) = self.find_job(job_id) {
            self.jobs[job_index].kill()?;
            Ok(Some(&self.jobs[job_index]))
        } else {
            Ok(None)
        }
    }

    /// Checks for processes that have status information available, without
    /// blocking.
    pub fn update_job_statues(&mut self) -> Result<()> {
        for job in &mut self.jobs {
            job.try_wait()?;
        }

        Ok(())
    }

    /// Notify the user about stopped or terminated jobs and remove terminated
    /// jobs from the active job list.
    pub fn do_job_notification(&mut self) {
        let temp_result = self.update_job_statues();
        log_if_err!(temp_result, "do_job_notification");

        for job in &mut self.jobs.iter_mut() {
            if job.is_completed() && !job.last_running_in_foreground() {
                // Unnecessary to notify if the job was last running in the
                // foreground, because the user will have noticed it completed.
                println!("{}", *job);
            } else if job.is_stopped() && !job.notified_stopped_job() {
                println!("{}", *job);
                job.set_notified_stopped_job(true);
            }
        }

        // Remove completed jobs
        self.jobs.retain(|j| !j.is_completed());
    }

    fn get_next_job_id(&mut self) -> JobId {
        self.job_count += 1;
        JobId(self.job_count)
    }

    /// # Panics
    /// Panics if job is not found
    fn job_is_running(&self, job_id: JobId) -> bool {
        let job_index = self.find_job(job_id).expect("job not found");
        !self.jobs[job_index].is_stopped() && !self.jobs[job_index].is_completed()
    }

    fn find_job(&self, job_id: JobId) -> Option<usize> {
        self.jobs.iter().position(|job| job.id() == job_id)
    }
}

impl fmt::Debug for JobManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} jobs\tjob_count: {}", self.jobs.len(), self.job_count)?;
        for job in &self.jobs {
            write!(f, "{:?}", job)?;
        }

        Ok(())
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Stopped => write!(f, "Stopped"),
            JobStatus::Completed => write!(f, "Completed"),
        }
    }
}

pub struct JobImpl {
    id: JobId,
    input: String,
    pgid: Option<libc::pid_t>,
    processes: Vec<Box<dyn Process>>,
    last_status_code: Option<ExitStatus>,
    last_running_in_foreground: bool,
    notified_stopped_job: bool,
    tmodes: Option<Termios>,
}

impl JobImpl {
    pub fn new(
        id: JobId,
        input: &str,
        pgid: Option<libc::pid_t>,
        processes: Vec<Box<dyn Process>>,
    ) -> Self {
        // Initialize last_status_code if possible; this prevents a completed
        // job from having a None last_status_code if all processes have
        // already completed (e.g. 'false && echo foo')
        let last_status_code = processes.iter().rev().find_map(|p| p.status_code());

        Self {
            id,
            input: input.to_string(),
            pgid,
            processes,
            last_status_code,
            last_running_in_foreground: true,
            notified_stopped_job: false,
            tmodes: termios::tcgetattr(util::unix::get_terminal()).ok(),
        }
    }

    fn pgid(&self) -> Option<libc::pid_t> {
        self.pgid
    }

    fn last_status_code(&self) -> Option<ExitStatus> {
        self.last_status_code
    }

    fn last_running_in_foreground(&self) -> bool {
        self.last_running_in_foreground
    }

    fn set_last_running_in_foreground(&mut self, last_running_in_foreground: bool) {
        self.last_running_in_foreground = last_running_in_foreground;
    }

    fn kill(&mut self) -> Result<()> {
        for process in &mut self.processes {
            process.kill()?;
        }

        Ok(())
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        for process in &mut self.processes {
            if let Some(exit_status) = process.try_wait()? {
                // BUG: this is not actually the most recently exited process,
                // but instead the latest process in the job that has exited
                self.last_status_code = Some(exit_status);
            }
        }

        Ok(self.last_status_code)
    }

    fn notified_stopped_job(&self) -> bool {
        self.notified_stopped_job
    }

    fn set_notified_stopped_job(&mut self, notified_stopped_job: bool) {
        self.notified_stopped_job = notified_stopped_job;
    }

    fn is_stopped(&self) -> bool {
        self.processes
            .iter()
            .all(|p| p.status() == ProcessStatus::Stopped)
    }

    fn is_completed(&self) -> bool {
        self.processes
            .iter()
            .all(|p| p.status() == ProcessStatus::Completed)
    }
}

impl Job for JobImpl {
    fn id(&self) -> JobId {
        self.id
    }

    fn input(&self) -> String {
        self.input.clone()
    }

    fn display(&self) -> String {
        format!("[{}] {}\t{}", self.id, self.status(), self.input)
    }

    fn processes(&self) -> &Vec<Box<dyn Process>> {
        &self.processes
    }
}

impl JobExt for JobImpl {
    fn tmodes(&self) -> &Option<Termios> {
        &self.tmodes
    }

    fn status(&self) -> JobStatus {
        if self.is_stopped() {
            JobStatus::Stopped
        } else if self.is_completed() {
            JobStatus::Completed
        } else {
            JobStatus::Running
        }
    }
}

impl fmt::Display for JobImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}\t{}", self.id, self.status(), self.input)
    }
}

impl fmt::Debug for JobImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "id: {}\tinput: {}", self.id, self.input)
    }
}

/// RAII struct to encapsulate manipulating terminal state.
struct TerminalState {
    prev_pgid: Pid,
    prev_tmodes: Option<Termios>,
}

impl TerminalState {
    fn new(new_pgid: Pid) -> TerminalState {
        debug!("setting terminal process group to job's process group");
        let shell_terminal = util::unix::get_terminal();
        unistd::tcsetpgrp(shell_terminal, new_pgid).unwrap();
        TerminalState {
            prev_pgid: unistd::getpgrp(),
            prev_tmodes: termios::tcgetattr(shell_terminal).ok(),
        }
    }
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        debug!("putting shell back into foreground and restoring shell's terminal modes");
        let shell_terminal = util::unix::get_terminal();
        unistd::tcsetpgrp(shell_terminal, self.prev_pgid).unwrap();
        if let Some(ref prev_tmodes) = self.prev_tmodes {
            let temp_result =
                termios::tcsetattr(shell_terminal, termios::SetArg::TCSADRAIN, prev_tmodes);
            log_if_err!(
                temp_result,
                "error restoring terminal configuration for shell"
            );
        }
    }
}
