use errors::*;
use execute_command::{Process, ProcessStatus};
use nix;
use nix::errno::Errno;
use nix::libc;
use nix::sys::signal::{self, Signal};
use nix::sys::termios::{self, Termios};
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{self, Pid};
use std::fmt;
use std::process::ExitStatus;
use util::{self, BshExitStatusExt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JobId(pub u32);

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Job {
    id: JobId,
    input: String,
    pgid: Option<libc::pid_t>,
    processes: Vec<Process>,
    last_status_code: Option<ExitStatus>,
    last_running_in_foreground: bool,
    notified_stopped_job: bool,
    tmodes: Option<Termios>,
}

impl Job {
    fn new(id: JobId, input: &str, pgid: Option<libc::pid_t>, processes: Vec<Process>) -> Job {
        // Initialize last_status_code if possible; this prevents a completed
        // job from having a None last_status_code if all processes have
        // already completed (e.g. 'false && echo foo')
        let last_status_code = processes
            .iter()
            .rev()
            .filter(|p| p.status_code().is_some())
            .nth(0)
            .map(|p| p.status_code().unwrap());

        Job {
            id,
            input: input.to_string(),
            pgid,
            processes,
            last_status_code,
            last_running_in_foreground: true,
            notified_stopped_job: false,
            tmodes: termios::tcgetattr(util::get_terminal()).ok(),
        }
    }

    pub fn input(&self) -> String {
        self.input.clone()
    }

    fn status(&self) -> ProcessStatus {
        if self.is_stopped() {
            ProcessStatus::Stopped
        } else if self.is_completed() {
            ProcessStatus::Completed
        } else {
            ProcessStatus::Running
        }
    }

    fn last_status_code(&self) -> Option<ExitStatus> {
        self.last_status_code
    }

    fn is_stopped(&self) -> bool {
        self.processes.iter().all(|p| {
            p.status() == ProcessStatus::Stopped
        })
    }

    fn is_completed(&self) -> bool {
        self.processes.iter().all(|p| {
            p.status() == ProcessStatus::Completed
        })
    }

    fn mark_exited(&mut self, pid: Pid, status_code: i32) {
        let status_code = {
            let process = self.find_process_mut(pid);
            process.set_status(ProcessStatus::Completed);
            let status_code = ExitStatus::from_status(status_code);
            process.set_status_code(status_code);
            status_code
        };
        self.last_status_code = Some(status_code);
    }

    fn mark_stopped(&mut self, pid: Pid, signal: &Signal) {
        let status_code = {
            let process = self.find_process_mut(pid);
            process.set_status(ProcessStatus::Stopped);
            Self::get_status_code_for_signal(signal)
        };
        self.last_status_code = Some(status_code);
    }

    fn mark_signaled(&mut self, pid: Pid, signal: &Signal) {
        let status_code = {
            let process = self.find_process_mut(pid);
            process.set_status(ProcessStatus::Completed);
            let status_code = Self::get_status_code_for_signal(signal);
            process.set_status_code(status_code);
            status_code
        };
        self.last_status_code = Some(status_code);
    }

    fn has_process(&self, pid: Pid) -> bool {
        self.processes.iter().any(|p| {
            p.id()
                .map(|other| Pid::from_raw(other as i32) == pid)
                .unwrap_or(false)
        })
    }

    /// # Panics
    /// Panics if process not found.
    fn find_process_mut(&mut self, pid: Pid) -> &mut Process {
        for process in &mut self.processes {
            if let Some(other_pid) = process.id() {
                if Pid::from_raw(other_pid as i32) == pid {
                    return process;
                }
            }
        }

        panic!("Process not found");
    }

    fn get_status_code_for_signal(signal: &Signal) -> ExitStatus {
        // TODO: decide if ExitStatus should preserve signal and status
        // separately or if should combine together
        let status_code = 128 + (*signal as i32);
        ExitStatus::from_status(status_code)
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id: {}\tinput: {}", self.id, self.input)
    }
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}\t{}", self.id, self.status(), self.input)
    }
}

#[derive(Default)]
pub struct JobManager {
    jobs: Vec<Job>,
    job_count: u32,
}

impl JobManager {
    pub fn create_job(&mut self, input: &str, pgid: Option<u32>, processes: Vec<Process>) -> JobId {
        let job_id = self.get_next_job_id();
        self.jobs.push(Job::new(
            job_id,
            input,
            pgid.map(|pgid| pgid as libc::pid_t),
            processes,
        ));
        job_id
    }

    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Waits for job to stop or complete.
    ///
    /// This function also updates the statuses of other jobs if we receive
    /// a signal for one of their processes.
    pub fn wait_for_job(&mut self, job_id: JobId) -> Result<Option<ExitStatus>> {
        while !self.job_is_stopped(job_id) && !self.job_is_completed(job_id) {
            let wait_status = wait::waitpid(None, Some(WaitPidFlag::WUNTRACED))?;
            self.mark_process_status(&wait_status);
        }

        let job = self.find_job(job_id);
        let last_status_code = job.last_status_code();
        Ok(last_status_code)
    }

    pub fn put_job_in_foreground(
        &mut self,
        job_id: JobId,
        cont: bool,
    ) -> Result<Option<ExitStatus>> {
        debug!("putting job [{}] in foreground", job_id);

        let _terminal_state = {
            let job = self.find_job_mut(job_id);
            job.last_running_in_foreground = true;
            let _terminal_state = job.pgid.map(|pgid| TerminalState::new(Pid::from_raw(pgid)));

            // Send the job a continue signal if necessary
            if cont {
                if let Some(ref tmodes) = job.tmodes {
                    let temp_result = termios::tcsetattr(
                        util::get_terminal(),
                        termios::SetArg::TCSADRAIN,
                        tmodes,
                    );
                    log_if_err!(
                        temp_result,
                        "error setting terminal configuration for job ({})",
                        job.id
                    );
                }
                if let Some(ref pgid) = job.pgid {
                    signal::kill(Pid::from_raw(-pgid), Signal::SIGCONT)?;
                }
            }
            _terminal_state
        };
        self.wait_for_job(job_id)
    }

    pub fn put_job_in_background(&mut self, job_id: JobId, cont: bool) -> Result<()> {
        let job = self.find_job_mut(job_id);
        job.last_running_in_foreground = false;
        if cont {
            if let Some(ref pgid) = job.pgid {
                signal::kill(Pid::from_raw(-pgid), Signal::SIGCONT)?;
            }
        }
        Ok(())
    }

    pub fn kill_job(&mut self, job_id: JobId) -> Result<Option<Job>> {
        let index = self.jobs.iter().position(|j| j.id == job_id);
        if index.is_none() {
            return Ok(None);
        }

        {
            let job = &self.jobs[index.unwrap()];
            if let Some(pgid) = job.pgid {
                signal::kill(Pid::from_raw(-pgid), Signal::SIGKILL)?;
            }
        }

        Ok(Some(self.jobs.remove(index.unwrap())))
    }

    /// Checks for processes that have status information available, without
    /// blocking.
    pub fn update_job_statues(&mut self) -> Result<()> {
        loop {
            let wait_status =
                wait::waitpid(None, Some(WaitPidFlag::WUNTRACED | WaitPidFlag::WNOHANG));
            match wait_status {
                Ok(status) => self.mark_process_status(&status),
                Err(nix::Error::Sys(Errno::ECHILD)) => break,
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    /// Notify the user about stopped or terminated jobs and remove terminated
    /// jobs from the active job list.
    pub fn do_job_notification(&mut self) {
        let temp_result = self.update_job_statues();
        log_if_err!(temp_result, "do_job_notification");

        for job in &mut self.jobs {
            if job.is_completed() {
                // Unnecessary to notify if the job was last running in the
                // foreground, because the user will have noticed it completed.
                if !job.last_running_in_foreground {
                    println!("{}", job);
                }
            } else if job.is_stopped() && !job.notified_stopped_job {
                println!("{}", job);
                job.notified_stopped_job = true;
            }
        }

        // Remove completed jobs
        self.jobs.retain(|j| !j.is_completed());
    }

    fn get_next_job_id(&mut self) -> JobId {
        self.job_count += 1;
        JobId(self.job_count)
    }

    fn mark_process_status(&mut self, wait_status: &WaitStatus) {
        match *wait_status {
            WaitStatus::Exited(pid, status_code) => {
                debug!("{} exited with {}.", pid, status_code);
                let job = self.find_job_with_process_mut(pid);
                job.mark_exited(pid, status_code);
            }
            WaitStatus::Signaled(pid, signal, ..) => {
                debug!("{} terminated by signal {:?}.", pid, signal);
                let job = self.find_job_with_process_mut(pid);
                job.mark_signaled(pid, &signal);
            }
            WaitStatus::Stopped(pid, signal) => {
                debug!("{} was signaled to stop {:?}.", pid, signal);
                let job = self.find_job_with_process_mut(pid);
                job.mark_stopped(pid, &signal);
                job.last_running_in_foreground = false;
            }
            _ => (),
        }
    }

    /// # Panics
    /// Panics if job is not found
    fn job_is_stopped(&self, job_id: JobId) -> bool {
        self.find_job(job_id).is_stopped()

    }

    /// # Panics
    /// Panics if job is not found
    fn job_is_completed(&self, job_id: JobId) -> bool {
        self.find_job(job_id).is_completed()
    }

    /// # Panics
    /// Panics if job is not found
    fn find_job(&self, job_id: JobId) -> &Job {
        self.jobs.iter().find(|job| job.id == job_id).unwrap()
    }

    /// # Panics
    /// Panics if job is not found
    fn find_job_mut(&mut self, job_id: JobId) -> &mut Job {
        self.jobs.iter_mut().find(|job| job.id == job_id).unwrap()
    }

    fn find_job_with_process_mut(&mut self, pid: Pid) -> &mut Job {
        self.jobs
            .iter_mut()
            .find(|job| job.has_process(pid))
            .unwrap()
    }
}

impl fmt::Debug for JobManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} jobs\tjob_count: {}\n",
            self.jobs.len(),
            self.job_count
        )?;
        for job in &self.jobs {
            write!(f, "{:?}", job)?;
        }

        Ok(())
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
        let shell_terminal = util::get_terminal();
        let temp_result = unistd::tcsetpgrp(shell_terminal, new_pgid);
        log_if_err!(temp_result, "failed to set terminal process group");
        TerminalState {
            prev_pgid: unistd::getpgrp(),
            prev_tmodes: termios::tcgetattr(shell_terminal).ok(),
        }
    }
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        debug!("putting shell back into foreground and restoring shell's terminal modes");
        let shell_terminal = util::get_terminal();
        let temp_result = unistd::tcsetpgrp(shell_terminal, self.prev_pgid);
        log_if_err!(temp_result, "error putting shell in foreground");
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
