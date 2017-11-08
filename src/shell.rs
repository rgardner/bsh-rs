//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for
//! maintaining a editor of previous commands.

use builtins;
use editor::Editor;
use errors::*;
use jobs::{BackgroundJob, BackgroundJobManager};
use parser::{Command, Job};
use rustyline::error::ReadlineError;
use std::env;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Stdio};

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
            shell.history_file = env::home_dir().map(|p| p.join(HISTORY_FILE_NAME));
            if let Some(ref history_file) = shell.history_file {
                shell.editor.load_history(&history_file).or_else(|e| {
                    if let ErrorKind::ReadlineError(ReadlineError::Io(ref inner)) = *e.kind() {
                        if inner.kind() == io::ErrorKind::NotFound {
                            return Ok(());
                        }
                    }

                    Err(e)
                })?;
            } else {
                eprintln!("failed to get bsh history file path");
            }
        }

        info!("bsh started up");
        Ok(shell)
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
    pub fn expand_variables(&mut self, job: &Job) -> Job {
        Job {
            input: job.input.clone(),
            commands: job.commands
                .iter()
                .map(|cmd| {
                    Command {
                        argv: cmd.argv
                            .iter()
                            .map(|s| expand_variables_helper(s))
                            .collect(),
                        infile: cmd.infile.clone().map(|s| expand_variables_helper(&s)),
                        outfile: cmd.outfile.clone().map(|s| expand_variables_helper(&s)),
                        ..*cmd
                    }
                })
                .collect(),
            background: job.background,
        }
    }

    /// Runs a job from a command string.
    pub fn execute_command_string(&mut self, input: &str) -> Result<()> {
        let mut command = input.to_owned();
        if self.config.enable_command_history {
            self.editor.expand_history(&mut command)?;
            self.editor.add_history_entry(input);
        }

        let jobs = Job::parse(input)?;
        for mut job in jobs {
            job = self.expand_variables(&job);
            self.execute_job(&mut job)?;
        }

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
    fn execute_job(&mut self, job: &mut Job) -> Result<()> {
        for command in &mut job.commands {
            if builtins::is_builtin(&command.program()) {
                let result = builtins::run(self, command);
                if let Err(e) = result {
                    eprintln!("{}", e);
                }
            } else {
                self.execute_external_command(command, job.background)?;
            }

            self.last_exit_status = command.status;
        }

        Ok(())
    }

    fn execute_external_command(
        &mut self,
        command: &mut Command,
        run_in_background: bool,
    ) -> Result<()> {
        let mut external_command = command.to_command();

        if command.infile.is_some() {
            external_command.stdin(Stdio::piped());
        }

        if command.outfile.is_some() {
            external_command.stdout(Stdio::piped());
        }

        let mut child = external_command.spawn()?;
        if let Some(ref mut stdin) = child.stdin {
            if let Some(ref infile) = command.infile {
                let mut f = File::open(infile)?;
                let mut buf: Vec<u8> = vec![];
                f.read_to_end(&mut buf)?;
                stdin.write_all(&buf)?;
            }
        }

        if let Some(ref mut stdout) = child.stdout {
            if let Some(ref outfile) = command.outfile {
                let mut file = OpenOptions::new().write(true).create(true).open(outfile)?;
                let mut buf: Vec<u8> = vec![];
                stdout.read_to_end(&mut buf)?;
                file.write_all(&buf)?;
            }
        }

        if run_in_background {
            self.background_jobs.add_job(child);
        } else {
            let output = child.wait_with_output().unwrap();
            command.status = output.status.code().unwrap_or(0);
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }

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
    /// 256 + (n % 256).
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
            256 + (code % 256)
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
