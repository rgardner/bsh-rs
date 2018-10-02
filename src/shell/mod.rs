use std::{path::Path, process::ExitStatus};

use core::job::{Job, JobId};
use editor::Editor;
use errors::Result;

#[cfg(unix)]
pub use self::unix::create_shell;
#[cfg(unix)]
pub mod unix;

pub trait Shell {
    fn execute_command_string(&mut self, input: &str) -> Result<()>;
    fn execute_commands_from_file(&mut self, path: &Path) -> Result<()>;
    fn execute_from_stdin(&mut self);
    fn exit(&mut self, n: Option<ExitStatus>) -> !;
    fn is_interactive(&self) -> bool;
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;

    // temporary job-control specific functions that will be moved off the
    // trait once some of the builtins are moved into the Shell's logic
    fn get_jobs(&self) -> Vec<Job>;
    fn has_background_jobs(&self) -> bool;
    fn put_job_in_foreground(&mut self, job_id: Option<JobId>) -> Result<Option<ExitStatus>>;
    fn put_job_in_background(&mut self, job_id: Option<JobId>) -> Result<()>;
    fn kill_background_job(&mut self, job_id: u32) -> Result<Option<Job>>;
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
    pub fn interactive(command_history_capacity: usize) -> Self {
        Self {
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
    pub fn noninteractive() -> Self {
        Default::default()
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            enable_command_history: false,
            command_history_capacity: 0,
            enable_job_control: false,
            display_messages: false,
        }
    }
}
