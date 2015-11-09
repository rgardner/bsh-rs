//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for maintaining a history of
//! previous commands.

use parse::ParseInfo;
use std::env;
use std::io::{self, Write};
use std::path::Path;

/// Bsh Shell
#[derive(Debug)]
pub struct Shell {
    jobs: Vec<ParseInfo>,
    history: Vec<ParseInfo>,
}

impl Shell {
    /// Constructs a new Shell to manage running jobs and command history.
    pub fn new() -> Shell {
        Shell {
            jobs: Vec::new(),
            history: Vec::new(),
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
}
