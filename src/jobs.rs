use builtins;
use errors::*;
use odds::vec::VecExt;
use shell::Shell;
use std::ffi::OsStr;
use std::fmt;
use std::process::{Child, Command, ExitStatus};

#[derive(Default)]
pub struct BackgroundJobManager {
    jobs: Vec<BackgroundJob>,
    job_count: u32,
}

impl BackgroundJobManager {
    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    // pub fn launch_job(&mut self, shell: &mut Shell, job: &Job) -> Result<()> {
    // }

    pub fn execute_simple_command<S>(shell: &mut Shell, words: &[S]) -> (i32, Result<()>)
    where
        S: AsRef<str> + AsRef<OsStr>,
    {
        if builtins::is_builtin(words) {
            builtins::run(shell, words)
        } else {
            execute_external_command(words)
        }
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

fn execute_external_command<S: AsRef<OsStr>>(words: &[S]) -> (i32, Result<()>) {
    let result = execute_external_command_internal(words);
    match result {
        Ok(exit_code) => (exit_code, Ok(())),
        Err(e) => (1, Err(e)),
    }
}

fn execute_external_command_internal<S: AsRef<OsStr>>(words: &[S]) -> Result<(i32)> {
    let child = Command::new(&words[0]).args(words[1..].iter()).spawn()?;
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
