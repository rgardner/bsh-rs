//! Bsh - Shell Module
//!
//! The Shell itself is responsible for managing background jobs and for
//! maintaining an editor of previous commands.

use std::env;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{self, ExitStatus};

use dirs;
use failure::ResultExt;
use nix::unistd;

use core::{
    intermediate_representation as ir,
    job::{Job, JobId},
    parser::Command,
    variable_expansion,
};
use errors::{ErrorKind, Result};
use shell::{
    editor::Editor,
    execute_command::spawn_processes,
    job_control::{self, JobManager},
};
use util::{self, BshExitStatusExt};

const HISTORY_FILE_NAME: &str = ".bsh_history";
const SYNTAX_ERROR_EXIT_STATUS: i32 = 2;
const COMMAND_NOT_FOUND_EXIT_STATUS: i32 = 127;

/// Bsh Shell
pub struct Shell {
    /// Responsible for readline and history.
    pub editor: Editor,
    history_file: Option<PathBuf>,
    job_manager: JobManager,
    /// Exit status of last command executed.
    last_exit_status: ExitStatus,
    config: ShellConfig,
    /// Is `false` if the shell is running a script or if initializing job
    /// control fails.
    is_interactive: bool,
}

impl Shell {
    /// Constructs a new Shell to manage running jobs and command history.
    pub fn new(config: ShellConfig) -> Result<Shell> {
        let mut shell = Shell {
            editor: Editor::with_capacity(config.command_history_capacity),
            history_file: None,
            job_manager: Default::default(),
            last_exit_status: ExitStatus::from_success(),
            config,
            is_interactive: isatty(),
        };

        if shell.is_interactive {
            let result = job_control::initialize_job_control();
            if let Err(e) = result {
                error!(
                    "failed to initialize shell for job control despite isatty: {}",
                    e
                );
                shell.is_interactive = false;
            }
        }

        if config.enable_command_history {
            shell.load_history()?
        }

        info!("bsh started up");
        Ok(shell)
    }

    pub(crate) fn is_interactive(&self) -> bool {
        self.is_interactive
    }

    fn load_history(&mut self) -> Result<()> {
        self.history_file = dirs::home_dir().map(|p| p.join(HISTORY_FILE_NAME));
        if let Some(ref history_file) = self.history_file {
            self.editor.load_history(&history_file).or_else(|e| {
                if let ErrorKind::HistoryFileNotFound = *e.kind() {
                    return Ok(());
                }

                Err(e)
            })?;
        } else {
            warn!("unable to get home directory")
        }

        Ok(())
    }

    /// Custom prompt to output to the user.
    /// Returns `None` when end of file is reached.
    pub fn prompt(&mut self) -> Result<Option<String>> {
        let cwd = env::current_dir().unwrap();
        let home = dirs::home_dir().unwrap();
        let rel = match cwd.strip_prefix(&home) {
            Ok(rel) => Path::new("~").join(rel),
            Err(_) => cwd.clone(),
        };

        let prompt = format!(
            "{}|{}\n$ ",
            self.last_exit_status.code().unwrap(),
            rel.display()
        );
        let line = self.editor.readline(&prompt)?;
        Ok(line)
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

        let command = match Command::parse(input) {
            Ok(command) => Ok(command),
            Err(e) => {
                if let ErrorKind::Syntax(ref line) = *e.kind() {
                    eprintln!("bsh: syntax error near: {}", line);
                    self.last_exit_status = ExitStatus::from_status(SYNTAX_ERROR_EXIT_STATUS);
                    return Ok(());
                }

                Err(e)
            }
        }?;

        let inner_command = variable_expansion::expand_variables(&command.inner);
        let mut command_group = ir::Interpreter::parse(Command::new(&command.input, inner_command));
        self.execute_command(&mut command_group)?;

        Ok(())
    }

    /// Runs a bsh script from a file.
    pub fn execute_commands_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        use std::io::Read;
        let mut f = File::open(path).context(ErrorKind::Io)?;
        let mut buffer = String::new();
        f.read_to_string(&mut buffer)
            .with_context(|_| ErrorKind::Io)?;

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
                self.job_manager.do_job_notification();
            }

            let input = match self.prompt() {
                Ok(Some(line)) => line.trim().to_owned(),
                Ok(None) => break,
                e => {
                    log_if_err!(e, "prompt");
                    continue;
                }
            };

            let temp_result = self.execute_command_string(&input);
            log_if_err!(temp_result, "execute_command_string");
        }
    }

    /// Runs a job.
    fn execute_command(&mut self, command_group: &mut ir::CommandGroup) -> Result<()> {
        let process_group = match spawn_processes(self, &command_group) {
            Ok(process_group) => Ok(process_group),
            Err(e) => {
                if let ErrorKind::CommandNotFound(ref command) = *e.kind() {
                    eprintln!("bsh: {}: command not found", command);
                    self.last_exit_status = ExitStatus::from_status(COMMAND_NOT_FOUND_EXIT_STATUS);
                    return Ok(());
                }

                Err(e)
            }
        }?;

        let foreground = process_group.foreground;
        let job_id = self
            .job_manager
            .create_job(&command_group.input, process_group);
        if !self.is_interactive() {
            self.last_exit_status = self.job_manager.wait_for_job(job_id)?.unwrap();
        } else if foreground {
            self.last_exit_status = self
                .job_manager
                .put_job_in_foreground(Some(job_id), false /* cont */)?
                .unwrap();
        } else {
            self.job_manager
                .put_job_in_background(Some(job_id), false /* cont */)?;
        }
        Ok(())
    }

    /// Returns `true` if the shell has background jobs.
    pub fn has_background_jobs(&self) -> bool {
        self.job_manager.has_jobs()
    }

    /// Returns the shell's jobs (running and stopped).
    pub fn get_jobs(&self) -> Vec<Job> {
        self.job_manager.get_jobs()
    }

    /// Starts the specified job or the current one.
    pub fn put_job_in_foreground(&mut self, job_id: Option<JobId>) -> Result<Option<ExitStatus>> {
        self.job_manager
            .put_job_in_foreground(job_id, true /* cont */)
    }

    /// Puts the specified job in the background, or the current one.
    pub fn put_job_in_background(&mut self, job_id: Option<JobId>) -> Result<()> {
        self.job_manager
            .put_job_in_background(job_id, true /* cont */)
    }

    /// Kills a child with the corresponding job id.
    ///
    /// Returns `true` if a corresponding job exists; `false`, otherwise.
    pub fn kill_background_job(&mut self, job_id: u32) -> Result<Option<Job>> {
        self.job_manager.kill_job(JobId(job_id))
    }

    /// Exit the shell.
    ///
    /// Valid exit codes are between 0 and 255. Like bash and its descendents, it automatically
    /// converts exit codes to a u8 such that positive n becomes n & 256 and negative n becomes
    /// (256 + n) % 256.
    ///
    /// Exit the shell with a status of n. If n is None, then the exit status is that of the last
    /// command executed.
    pub fn exit(&mut self, n: Option<ExitStatus>) -> ! {
        if self.config.display_messages {
            println!("exit");
        }

        let code = match n {
            Some(n) => n.code().unwrap(),
            None => self.last_exit_status.code().unwrap(),
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

impl fmt::Debug for Shell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} jobs\n{:?}", self.job_manager, self.editor)
    }
}

/// Policy object to control a Shell's behavior
#[derive(Debug, Copy, Clone)]
pub struct ShellConfig {
    /// Determines if new command entries will be added to the shell's command history.
    ///
    /// Note: This is checked before the other command history config fields.
    enable_command_history: bool,

    /// Number of entries to store in the shell's command history
    command_history_capacity: usize,

    /// Determines if job control (fg and bg) is supported.
    enable_job_control: bool,

    /// Determines if some messages (e.g. "exit") should be displayed.
    display_messages: bool,
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

fn isatty() -> bool {
    let temp_result = unistd::isatty(util::get_terminal());
    log_if_err!(temp_result, "unistd::isatty");
    temp_result.unwrap_or(false)
}
