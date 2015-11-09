//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for maintaining a history of
//! previous commands.

use parse::ParseInfo;
use history::HistoryState;
use std::env;
use std::io::{self, Write};
use std::path::Path;

/// Bsh Shell
#[derive(Debug)]
pub struct Shell {
    jobs: Vec<ParseInfo>,
    history: HistoryState,
}

impl Shell {
    /// Constructs a new Shell to manage running jobs and command history.
    pub fn new(history_capacity: usize) -> Shell {
        Shell {
            jobs: Vec::new(),
            history: HistoryState::new(history_capacity),
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
        self.history.push(job);
    }
}
