//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for
//! maintaining a history of previous commands.

use errors::*;
use builtins;
use parse::ParseJob;
use history::HistoryState;
use odds::vec::VecExt;
use std::env;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::process::{self, Child, Stdio};
use wait_timeout::ChildExt;

/// Bsh Shell
pub struct Shell {
    /// History of previously executed shell commands.
    pub history: HistoryState,
    jobs: Vec<BackgroundJob>,
    job_count: u32,
    /// Exit status of last command executed.
    last_exit_status: i32,
}

impl Shell {
    /// Constructs a new Shell to manage running jobs and command history.
    pub fn new(history_capacity: usize) -> Shell {
        Shell {
            history: HistoryState::with_capacity(history_capacity),
            jobs: Vec::new(),
            job_count: 0,
            last_exit_status: 0,
        }
    }

    /// Custom prompt to output to the user.
    pub fn prompt(&self, buf: &mut String) -> io::Result<usize> {
        let cwd = env::current_dir().unwrap();
        let home = env::home_dir().unwrap();
        let rel = match cwd.strip_prefix(&home) {
            Ok(rel) => ::std::path::Path::new("~").join(rel),
            Err(_) => cwd.clone(),
        };

        print!("{}|{} $ ", self.last_exit_status, rel.display());
        io::stdout().flush().unwrap();
        io::stdin().read_line(buf)
    }

    /// Perform history expansions.
    ///
    /// !n -> repeat command numbered n in the list of commands (starting at 1)
    /// !-n -> repeat last nth command (starting at -1)
    /// !string -> searches through history for first item that matches the string (via contains)
    pub fn expand_history(&self, job: &mut String) -> Result<()> {
        self.history.expand(job)
    }

    /// Add a job to the history.
    pub fn add_history(&mut self, job: &str) {
        self.history.push(&job);
    }

    /// Add a job to the background.
    ///
    /// Job ids start at 1 and increment upwards as long as all the job list is non-empty. When
    /// all jobs have finished executing, the next background job id will be 1.
    pub fn add_to_background(&mut self, child: Child) {
        self.job_count += 1;
        println!("[{}] {}", self.job_count, child.id());
        let job = BackgroundJob {
            command: String::new(),
            child: child,
            idx: self.job_count,
        };
        self.jobs.push(job);
    }

    /// Run a job.
    pub fn run(&mut self, job: &mut ParseJob) -> Result<()> {
        let process = job.commands.get_mut(0).unwrap();
        if builtins::is_builtin(&process.program) {
            let res = builtins::run(self, &process);
            self.last_exit_status = if let Err(ref e) = res {
                match *e {
                    Error(ErrorKind::BuiltinCommandError(_, code), _) => code,
                    Error(ErrorKind::Parse(_), _) => 2,
                    Error(ErrorKind::Io(_), _) => 1,
                    Error(ErrorKind::Msg(_), _) => 2,
                }
            } else {
                0
            };
            return res;
        }
        let mut command = process.to_command();

        if let Some(_) = job.infile {
            command.stdin(Stdio::piped());
        }

        if let Some(_) = job.outfile {
            command.stdout(Stdio::piped());
        }

        let mut child = try!(command.spawn());
        if let Some(ref mut stdin) = child.stdin {
            let infile = job.infile.take().unwrap();
            let mut f = try!(File::open(infile));
            let mut buf: Vec<u8> = vec![];
            try!(f.read_to_end(&mut buf));
            try!(stdin.write_all(&buf));
        }
        if let Some(ref mut stdout) = child.stdout {
            let outfile = job.outfile.take().unwrap();
            let mut file = try!(OpenOptions::new().write(true).create(true).open(outfile));
            let mut buf: Vec<u8> = vec![];
            try!(stdout.read_to_end(&mut buf));
            try!(file.write_all(&buf));
        } else if job.background {
            self.add_to_background(child);
        } else if !job.background {
            let output = child.wait_with_output().unwrap();
            self.last_exit_status = output.status.code().unwrap_or(0);
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }

        Ok(())
    }

    /// Returns `true` if the shell has background jobs.
    pub fn has_background_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Check on the status of background jobs, removing exited ones.
    pub fn check_jobs(&mut self) {
        self.jobs.retain_mut(|mut job| {
            match job.child.wait_timeout_ms(0).unwrap() {
                Some(status) => {
                    println!("[{}]+\t{}\t{}", job.idx, status, job.command);
                    false
                }
                None => true,
            }
        });
        if self.jobs.is_empty() {
            self.job_count = 0;
        }
    }

    /// Kills a child with the corresponding jobid.
    ///
    /// Returns `true` if a corresponding job exists; `false`, otherwise.
    pub fn kill_job(&mut self, jobid: u32) -> Result<Option<BackgroundJob>> {
        match self.jobs.iter().position(|j| j.idx == jobid) {
            Some(n) => {
                let mut job = self.jobs.remove(n);
                try!(job.child.kill());
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Exit the shell.
    ///
    /// Valid exit codes are between 0 and 255. Like bash and its descendents, it automatically
    /// converts exit codes to a u8 such that positive n becomes n & 256 and negative n becomes
    /// 256 + n % 256.
    ///
    /// Exit the shell with a status of n. If n is None, then the exit status is that of the last
    /// command executed.
    pub fn exit(&mut self, n: Option<i32>) {
        println!("exit");
        let code = match n {
            Some(n) => n,
            None => self.last_exit_status,
        };
        let code_like_u8 = if code < 0 {
            256 + code % 256
        } else {
            code % 256
        };
        process::exit(code_like_u8);
    }
}

impl fmt::Debug for Shell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} jobs\n{}", self.jobs.len(), self.history)
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
        write!(f,
               "command: {}\tpid: {}\tidx: {}",
               self.command,
               self.child.id(),
               self.idx)
    }
}
