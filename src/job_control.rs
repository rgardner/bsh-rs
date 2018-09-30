use std::fmt;
use std::process::ExitStatus;

use failure::{Fail, ResultExt};
use nix;
use nix::errno::Errno;
use nix::libc;
use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::termios::{self, Termios};
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{self, Pid};

use core::job::{Job, JobId, ProcessGroup};
use errors::{Error, ErrorKind, Result};
use util::{self, VecExt};

pub fn initialize_job_control() -> Result<()> {
    let shell_terminal = util::get_terminal();

    // Loop until the shell is in the foreground
    loop {
        let shell_pgid = unistd::getpgrp();
        if unistd::tcgetpgrp(shell_terminal).context(ErrorKind::Nix)? == shell_pgid {
            break;
        } else {
            signal::kill(
                Pid::from_raw(-libc::pid_t::from(shell_pgid)),
                Signal::SIGTTIN,
            ).unwrap();
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
    let shell_terminal = util::get_terminal();
    let temp_result = unistd::tcsetpgrp(shell_terminal, shell_pgid);
    log_if_err!(temp_result, "failed to grab control of terminal");

    Ok(())
}

#[derive(Default)]
pub struct JobManager {
    jobs: Vec<Job>,
    job_count: u32,
    current_job: Option<JobId>,
}

impl JobManager {
    pub fn create_job(&mut self, input: &str, process_group: ProcessGroup) -> JobId {
        let job_id = self.get_next_job_id();
        self.jobs.push(Job::new(
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

    pub fn get_jobs(&self) -> Vec<Job> {
        self.jobs.clone()
    }

    /// Waits for job to stop or complete.
    ///
    /// This function also updates the statuses of other jobs if we receive
    /// a signal for one of their processes.
    pub fn wait_for_job(&mut self, job_id: JobId) -> Result<Option<ExitStatus>> {
        loop {
            let wait_status =
                wait::waitpid(None, Some(WaitPidFlag::WUNTRACED)).context(ErrorKind::Nix)?;
            self.mark_process_status(&wait_status);

            if self.job_is_stopped(job_id) || self.job_is_completed(job_id) {
                break;
            }
        }

        let job_index = self.find_job(job_id).expect("job not found");
        let last_status_code = self.jobs[job_index].last_status_code();
        Ok(last_status_code)
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
            self.jobs
                .update(job_index, |j| j.set_last_running_in_foreground(true));
            let job_pgid = self.jobs[job_index].pgid();
            let job_tmodes = self.jobs[job_index].tmodes().clone();
            let _terminal_state = job_pgid.map(|pgid| TerminalState::new(Pid::from_raw(pgid)));

            // Send the job a continue signal if necessary
            if cont {
                if let Some(ref tmodes) = job_tmodes {
                    let temp_result = termios::tcsetattr(
                        util::get_terminal(),
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
            self.jobs
                .update(job_index, |j| j.set_last_running_in_foreground(false));
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

    pub fn kill_job(&mut self, job_id: JobId) -> Result<Option<Job>> {
        if let Some(job_index) = self.find_job(job_id) {
            if let Some(pgid) = self.jobs[job_index].pgid() {
                signal::kill(Pid::from_raw(-pgid), Signal::SIGKILL).context(ErrorKind::Nix)?;
            }

            Ok(Some(self.jobs.remove(job_index)))
        } else {
            return Ok(None);
        }
    }

    /// Checks for processes that have status information available, without
    /// blocking.
    pub fn update_job_statues(&mut self) -> Result<()> {
        loop {
            let wait_status =
                wait::waitpid(None, Some(WaitPidFlag::WUNTRACED | WaitPidFlag::WNOHANG));
            match wait_status {
                Ok(WaitStatus::StillAlive) | Err(nix::Error::Sys(Errno::ECHILD)) => break,
                Ok(status) => self.mark_process_status(&status),
                Err(e) => return Err(e.context(ErrorKind::Nix).into()),
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
                if !job.last_running_in_foreground() {
                    println!("{}", job);
                }
            } else if job.is_stopped() && !job.notified_stopped_job() {
                println!("{}", job);
                *job = job.clone().set_notified_stopped_job(true)
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
                let job_index = self.find_job_with_process(pid).expect("unable to find job");
                self.jobs
                    .update(job_index, |job| job.mark_exited(pid.into(), status_code));
            }
            WaitStatus::Signaled(pid, signal, ..) => {
                debug!("{} terminated by signal {:?}.", pid, signal);
                let job_index = self.find_job_with_process(pid).expect("unable to find job");
                self.jobs
                    .update(job_index, |job| job.mark_signaled(pid.into(), signal));
            }
            WaitStatus::Stopped(pid, signal) => {
                debug!("{} was signaled to stop {:?}.", pid, signal);
                let job_index = self.find_job_with_process(pid).expect("unable to find job");
                self.jobs
                    .update(job_index, |job| job.mark_stopped(pid.into(), signal));
                self.current_job = Some(self.jobs[job_index].id());
            }
            WaitStatus::StillAlive => panic!("mark_process_status called with StillAlive"),
            _ => (),
        }
    }

    /// # Panics
    /// Panics if job is not found
    fn job_is_stopped(&self, job_id: JobId) -> bool {
        let job_index = self.find_job(job_id).expect("job not found");
        self.jobs[job_index].is_stopped()
    }

    /// # Panics
    /// Panics if job is not found
    fn job_is_completed(&self, job_id: JobId) -> bool {
        let job_index = self.find_job(job_id).expect("job not found");
        self.jobs[job_index].is_completed()
    }

    fn find_job(&self, job_id: JobId) -> Option<usize> {
        self.jobs.iter().position(|job| job.id() == job_id)
    }

    fn find_job_with_process(&self, pid: Pid) -> Option<usize> {
        self.jobs.iter().position(|job| job.has_process(pid.into()))
    }
}

impl fmt::Debug for JobManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} jobs\tjob_count: {}", self.jobs.len(), self.job_count)?;
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
        let shell_terminal = util::get_terminal();
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
