//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for
//! maintaining a history of previous commands.

use builtins;
use error::BshError;
use parse::ParseJob;
use history::HistoryState;
use odds::vec::VecExt;
use std::env;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::process::{Child, Stdio};
use wait_timeout::ChildExt;

/// Bsh Shell
pub struct Shell {
    jobs: Vec<BackgroundJob>,
    job_count: u32,
    history: HistoryState,
}

impl Shell {
    /// Constructs a new Shell to manage running jobs and command history.
    pub fn new(history_capacity: usize) -> Shell {
        Shell {
            jobs: Vec::new(),
            job_count: 0,
            history: HistoryState::with_capacity(history_capacity),
        }
    }

    /// Custom prompt to output to the user.
    pub fn prompt(buf: &mut String) -> io::Result<usize> {
        prompt(buf)
    }

    /// Add a job to the history.
    pub fn add_history(&mut self, job: &ParseJob) {
        self.history.push(&job);
    }

    /// Add a job to the background.
    pub fn add_to_background(&mut self, child: Child) {
        println!("[{}]: {}", self.jobs.len(), child.id());
        self.job_count += 1;
        let job = BackgroundJob {
            command: "".to_string(),
            child: child,
            idx: self.job_count,
        };
        self.jobs.push(job);
    }

    /// Run a job.
    pub fn run(&mut self, job: &mut ParseJob) -> Result<(), BshError> {
        let process = job.commands.get_mut(0).unwrap();
        if builtins::is_builtin(&process.program) {
            return builtins::run(&process);
        }
        let mut command = process.to_command();
        // if it's a builtin, call the builtin

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
            print!("{}",
                   String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout));
        }

        Ok(())
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
}

impl fmt::Debug for Shell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} jobs\n{}", self.jobs.len(), self.history)
    }
}

struct BackgroundJob {
    command: String,
    child: Child,
    idx: u32,
}

#[cfg(feature="unstable")]
/// Custom prompt to output to the user.
fn prompt(buf: &mut String) -> io::Result<usize> {
    use std::path::Path;
    let cwd = env::current_dir().unwrap();
    let home = env::home_dir().unwrap();
    let rel = match cwd.relative_from(&home) {
        Some(rel) => Path::new("~/").join(rel),
        None => cwd.clone(),
    };

    print!("{} $ ", rel.display());
    io::stdout().flush().unwrap();
    io::stdin().read_line(buf)
}

#[cfg(not(feature="unstable"))]
/// Prompt the user for input.
fn prompt(buf: &mut String) -> io::Result<usize> {
    let cwd = env::current_dir().unwrap();
    print!("{} $ ", cwd.display());
    io::stdout().flush().unwrap();
    io::stdin().read_line(buf)
}
