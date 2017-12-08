use errors::*;
use execute_command::{Process, ProcessStatus};
use nix::libc;
use nix::sys::wait::{self, WaitStatus};
use nix::unistd::Pid;
use odds::vec::VecExt;
use std::fmt;
use std::process::{Child, ExitStatus};
use util::BshExitStatusExt;

pub struct Job {
    input: String,
    processes: Vec<Process>,
    last_status_code: Option<ExitStatus>,
    notified_stopped_job: bool,
}

impl Job {
    pub fn new(input: &str, processes: Vec<Process>) -> Job {
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
            input: input.to_string(),
            processes,
            last_status_code,
            notified_stopped_job: false,
        }
    }

    pub fn last_status_code(&self) -> Option<ExitStatus> {
        self.last_status_code
    }

    pub fn wait(&mut self) -> Result<()> {
        while !self.is_stopped() && !self.is_completed() {
            let wait_any_child = Pid::from_raw(-1);
            let wait_status = wait::waitpid(wait_any_child, Some(wait::WUNTRACED))?;
            match wait_status {
                WaitStatus::Exited(pid, status_code) => {
                    debug!("{} exited with {}.", pid, status_code);
                    let process = &mut find_process(&mut self.processes, pid).unwrap();
                    process.set_status(ProcessStatus::Completed);
                    let status_code = i32::from(status_code);
                    let status_code = ExitStatus::from_status(status_code);
                    process.set_status_code(status_code);
                    self.last_status_code = Some(status_code);
                }
                WaitStatus::Stopped(pid, signal) => {
                    debug!("{} was signaled to stop {:?}.", pid, signal);
                    let process = &mut find_process(&mut self.processes, pid).unwrap();
                    process.set_status(ProcessStatus::Stopped);
                }
                WaitStatus::Signaled(pid, signal, ..) => {
                    eprintln!("{} terminated by signal {:?}.", pid, signal);
                    let process = &mut find_process(&mut self.processes, pid).unwrap();
                    process.set_status(ProcessStatus::Stopped);
                    // TODO: decide if ExitStatus should preserve signal and status
                    // separately or if should combine together
                    let status_code = 128 + (signal as i32);
                    let status_code = ExitStatus::from_status(status_code);
                    process.set_status_code(status_code);
                    self.last_status_code = Some(status_code);
                }
                _ => continue,
            }
        }

        Ok(())
    }

    fn is_stopped(&self) -> bool {
        for process in &self.processes {
            if process.status() != ProcessStatus::Stopped {
                return false;
            }
        }

        true
    }

    fn is_completed(&self) -> bool {
        for process in &self.processes {
            if process.status() != ProcessStatus::Completed {
                return false;
            }
        }

        true
    }
}

fn find_process(processes: &mut Vec<Process>, pid: Pid) -> Option<&mut Process> {
    for process in processes.iter_mut() {
        if let Some(id) = process.id() {
            if Pid::from_raw(id as libc::pid_t) == pid {
                return Some(process);
            }
        }
    }

    None
}

#[derive(Default)]
pub struct BackgroundJobManager {
    jobs: Vec<BackgroundJob>,
    job_count: u32,
}

impl BackgroundJobManager {
    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Add a job to the background.
    ///
    /// Job ids start at 1 and increment upwards as long as all the job list is non-empty. When
    /// all jobs have finished executing, the next background job id will be 1.
    pub fn add_job(&mut self, child: Child) {
        self.job_count += 1;
        println!("[{}] {}", self.job_count, child.id());
        let job = BackgroundJob {
            command: String::new(),
            child: child,
            idx: self.job_count,
        };
        self.jobs.push(job);
    }

    pub fn kill_job(&mut self, job_id: u32) -> Result<Option<BackgroundJob>> {
        match self.jobs.iter().position(|j| j.idx == job_id) {
            Some(n) => {
                let mut job = self.jobs.remove(n);
                job.child.kill()?;
                if self.jobs.is_empty() {
                    self.job_count = 0;
                }
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Check on the status of background jobs, removing exited ones.
    pub fn check_jobs(&mut self) {
        self.jobs.retain_mut(
            |job| match job.child.try_wait().expect(
                "error in try_wait",
            ) {
                Some(status) => {
                    println!("[{}]+\t{}\t{}", job.idx, status, job.command);
                    false
                }
                None => true,
            },
        );
        if self.jobs.is_empty() {
            self.job_count = 0;
        }
    }
}

impl fmt::Debug for BackgroundJobManager {
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

/// A job running in the background that the shell is responsible for.
pub struct BackgroundJob {
    /// The original command string entered.
    pub command: String,
    child: Child,
    idx: u32,
}

impl fmt::Debug for BackgroundJob {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "command: {}\tpid: {}\tidx: {}",
            self.command,
            self.child.id(),
            self.idx
        )
    }
}
