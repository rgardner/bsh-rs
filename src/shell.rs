//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for
//! maintaining a history of previous commands.

use parse::ParseInfo;
use history::HistoryState;
use odds::vec::VecExt;
use std::env;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, Stdio};
use wait_timeout::ChildExt;

/// Bsh Shell
pub struct Shell {
    jobs: Vec<Child>,
    history: HistoryState,
}

impl Shell {
    /// Constructs a new Shell to manage running jobs and command history.
    pub fn new(history_capacity: usize) -> Shell {
        Shell {
            jobs: Vec::new(),
            history: HistoryState::with_capacity(history_capacity),
        }
    }

    /// Custom prompt to output to the user.
    pub fn prompt(buf: &mut String) -> io::Result<usize> {
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

    /// Add a job to the history.
    pub fn add_history(&mut self, job: &ParseInfo) {
        self.history.push(&job);
    }

    /// Add a job to the background.
    pub fn add_to_background(&mut self, child: Child) {
        println!("[{}]: {}", self.jobs.len(), child.id());
        self.jobs.push(child);
    }

    /// Run a job.
    pub fn run(&mut self, job: &mut ParseInfo) -> Result<(), io::Error> {
        let mut command = job.commands.get_mut(0).unwrap();
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
        self.jobs
            .retain_mut(|mut child| {
                match child.wait_timeout_ms(0).unwrap() {
                    Some(status) => {
                        println!("[{}]+", status.code());
                        false
                    }
                    None => true
                }
            });
    }
}

impl fmt::Debug for Shell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} jobs\n{}", self.jobs.len(), self.history)
    }
}
