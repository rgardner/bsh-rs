//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for
//! maintaining an editor of previous commands.

use editor::Editor;
use errors::*;
use execute_command::spawn_processes;
use job_control::{BackgroundJob, BackgroundJobManager};
use parser::{Command, ast};
use rustyline::error::ReadlineError;
use std::env;
use std::fmt;
use std::fs::File;
use std::io;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::process;

const HISTORY_FILE_NAME: &str = ".bsh_history";

/// Bsh Shell
pub struct Shell {
    /// Responsible for readline and history.
    pub editor: Editor,
    history_file: Option<PathBuf>,
    background_jobs: BackgroundJobManager,
    /// Exit status of last command executed.
    last_exit_status: i32,
    config: ShellConfig,
}

impl Shell {
    /// Constructs a new Shell to manage running jobs and command history.
    pub fn new(config: ShellConfig) -> Result<Shell> {
        let mut shell = Shell {
            editor: Editor::with_capacity(config.command_history_capacity),
            history_file: None,
            background_jobs: Default::default(),
            last_exit_status: 0,
            config,
        };

        if config.enable_command_history {
            shell.load_history()?
        }

        info!("bsh started up");
        Ok(shell)
    }

    fn load_history(&mut self) -> Result<()> {
        self.history_file = env::home_dir().map(|p| p.join(HISTORY_FILE_NAME));
        if let Some(ref history_file) = self.history_file {
            self.editor.load_history(&history_file).or_else(|e| {
                if let ErrorKind::ReadlineError(ReadlineError::Io(ref inner)) = *e.kind() {
                    if inner.kind() == io::ErrorKind::NotFound {
                        return Ok(());
                    }
                }

                Err(e)
            })?;
        }

        Ok(())
    }

    /// Custom prompt to output to the user.
    pub fn prompt(&mut self) -> Result<String> {
        let cwd = env::current_dir().unwrap();
        let home = env::home_dir().unwrap();
        let rel = match cwd.strip_prefix(&home) {
            Ok(rel) => Path::new("~").join(rel),
            Err(_) => cwd.clone(),
        };

        let prompt = format!("{}|{}\n$ ", self.last_exit_status, rel.display());
        let line = self.editor.readline(&prompt)?;
        Ok(line)
    }

    /// Expands shell and environment variables in command parts.
    /// note: rustfmt formatting makes function less readable
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn expand_variables(&mut self, command: &mut Command) {
        let mut current = &mut command.inner;
        loop {
            // restrict scope of borrowing `current` via `{current}` (new scope)
            // solves E0506 rustc error, "cannot assign to `current` because it is borrowed"
            current = match *{current} {
                ast::Command::Simple { ref mut words, ref mut redirects, .. } => {
                    expand_variables_simple_command(words, redirects.as_mut_slice());
                    break;
                },
                ast::Command::Connection { ref mut first, ref mut second, .. } => {
                    match *first.deref_mut() {
                        ast::Command::Simple { ref mut words, ref mut redirects, .. } => {
                            expand_variables_simple_command(words, redirects.as_mut_slice());
                        },
                        _ => unreachable!(),
                    };
                    &mut *second
                }
            };
        }
    }

    /// Runs a job from a command string.
    pub fn execute_command_string(&mut self, input: &str) -> Result<()> {
        // skip if empty
        if input.is_empty() {
            return Ok(());
        }

        let mut command = input.to_owned();
        if self.config.enable_command_history {
            self.editor.expand_history(&mut command)?;
            self.editor.add_history_entry(input);
        }

        let mut command = Command::parse(input)?;
        self.expand_variables(&mut command);
        self.execute_command(&mut command)?;

        Ok(())
    }

    /// Runs a bsh script from a file.
    pub fn execute_commands_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        use std::io::Read;
        let mut f = File::open(path)?;
        let mut buffer = String::new();
        f.read_to_string(&mut buffer)?;

        for line in buffer.split('\n') {
            self.execute_command_string(line)?
        }

        Ok(())
    }

    /// Runs jobs from stdin until EOF is received.
    pub fn execute_from_stdin(&mut self) {
        loop {
            if self.config.enable_job_control {
                // Check the status of background jobs, removing exited ones.
                self.background_jobs.check_jobs();
            }

            let input = match self.prompt() {
                Ok(line) => line.trim().to_owned(),
                Err(Error(ErrorKind::ReadlineError(ReadlineError::Eof), _)) => break,
                _ => continue,
            };

            if let Err(e) = self.execute_command_string(&input) {
                eprintln!("bsh: {}", e);
            }
        }
    }

    /// Runs a job.
    fn execute_command(&mut self, command: &mut Command) -> Result<()> {
        let processes = spawn_processes(self, &command.inner)?;
        self.last_exit_status = processes.last().unwrap().status_code().unwrap();
        Ok(())
    }

    /// Returns `true` if the shell has background jobs.
    pub fn has_background_jobs(&self) -> bool {
        self.background_jobs.has_jobs()
    }

    /// Kills a child with the corresponding job id.
    ///
    /// Returns `true` if a corresponding job exists; `false`, otherwise.
    pub fn kill_background_job(&mut self, job_id: u32) -> Result<Option<BackgroundJob>> {
        self.background_jobs.kill_job(job_id)
    }

    /// Exit the shell.
    ///
    /// Valid exit codes are between 0 and 255. Like bash and its descendents, it automatically
    /// converts exit codes to a u8 such that positive n becomes n & 256 and negative n becomes
    /// (256 + n) % 256.
    ///
    /// Exit the shell with a status of n. If n is None, then the exit status is that of the last
    /// command executed.
    pub fn exit(&mut self, n: Option<i32>) -> ! {
        if self.config.display_messages {
            println!("exit");
        }

        let code = match n {
            Some(n) => n,
            None => self.last_exit_status,
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
}

fn expand_variables_simple_command(words: &mut Vec<String>, redirects: &mut [ast::Redirect]) {
    for word in words.iter_mut() {
        *word = expand_variables_helper(word);
    }
    for redirect in redirects.iter_mut() {
        if let Some(ast::Redirectee::Filename(ref mut filename)) = redirect.redirector {
            *filename = expand_variables_helper(filename);
        }
        if let ast::Redirectee::Filename(ref mut filename) = redirect.redirectee {
            *filename = expand_variables_helper(filename);
        }
    }
}

fn expand_variables_helper(s: &str) -> String {
    let expansion = match s {
        "~" => env::home_dir().map(|p| p.to_string_lossy().into_owned()),
        s if s.starts_with('$') => env::var(s[1..].to_string()).ok(),
        _ => Some(s.to_string()),
    };

    expansion.unwrap_or_else(|| "".to_string())
}

impl fmt::Debug for Shell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} jobs\n{:?}", self.background_jobs, self.editor)
    }
}

/// Policy object to control a Shell's behavior
#[derive(Debug, Copy, Clone)]
pub struct ShellConfig {
    /// Determines if new command entries will be added to the shell's command history.
    ///
    /// Note: This is checked before the other command history config fields.
    pub enable_command_history: bool,

    /// Number of entries to store in the shell's command history
    pub command_history_capacity: usize,

    /// Determines if job control (fg and bg) is supported.
    pub enable_job_control: bool,

    /// Determines if some messages (e.g. "exit") should be displayed.
    pub display_messages: bool,
}

impl ShellConfig {
    /// Creates an interactive shell, e.g. command history, job control
    ///
    /// # Complete List
    /// - Command History is enabled
    /// - Job Control is enabled
    /// - Some additional messages are displayed
    pub fn interactive(command_history_capacity: usize) -> ShellConfig {
        ShellConfig {
            enable_command_history: true,
            command_history_capacity,
            enable_job_control: true,
            display_messages: true,
        }
    }

    /// Creates a noninteractive shell, e.g. no command history, no job control
    ///
    /// # Complete List
    /// - Command History is disabled. Commands are not saved and history expansions are not
    ///   performed. The history builtin command is not affected by this option.
    /// - Job Control is disabled.
    /// - Fewer messages are displayed
    pub fn noninteractive() -> ShellConfig {
        Default::default()
    }
}

impl Default for ShellConfig {
    fn default() -> ShellConfig {
        ShellConfig {
            enable_command_history: false,
            command_history_capacity: 0,
            enable_job_control: false,
            display_messages: false,
        }
    }
}
