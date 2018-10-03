use std::{
    env,
    fs::File,
    path::{Path, PathBuf},
    process::{self, ExitStatus},
};

use dirs;
use failure::ResultExt;

use core::{
    intermediate_representation as ir,
    job::{Job, JobId},
    parser::Command,
    variable_expansion,
};
use editor::Editor;
use errors::{Error, ErrorKind, Result};
use util::{self, BshExitStatusExt};

const HISTORY_FILE_NAME: &str = ".bsh_history";
const SYNTAX_ERROR_EXIT_STATUS: i32 = 2;
const COMMAND_NOT_FOUND_EXIT_STATUS: i32 = 127;

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

pub struct SimpleShell {
    editor: Editor,
    history_file: Option<PathBuf>,
    last_exit_status: ExitStatus,
    config: ShellConfig,
    is_interactive: bool,
}

impl SimpleShell {
    fn new(config: ShellConfig) -> Result<Self> {
        let shell = SimpleShell {
            editor: Editor::with_capacity(config.command_history_capacity),
            history_file: None,
            last_exit_status: ExitStatus::from_success(),
            config,
            is_interactive: util::isatty(),
        };

        Ok(shell)
    }

    /// Custom prompt to output to the user.
    /// Returns `None` when end of file is reached.
    fn prompt(&mut self) -> Result<Option<String>> {
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

    fn execute_command(&mut self, _command_group: &mut ir::CommandGroup) -> Result<()> {
        unimplemented!()
    }
}

impl Shell for SimpleShell {
    fn execute_command_string(&mut self, input: &str) -> Result<()> {
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

        let inner_command =
            variable_expansion::expand_variables(&command.inner, dirs::home_dir(), env::vars());
        let mut command_group = ir::Interpreter::parse(Command::new(&command.input, inner_command));
        self.execute_command(&mut command_group)?;

        Ok(())
    }

    fn execute_commands_from_file(&mut self, path: &Path) -> Result<()> {
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

    fn execute_from_stdin(&mut self) {
        loop {
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

    fn exit(&mut self, n: Option<ExitStatus>) -> ! {
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

    fn is_interactive(&self) -> bool {
        self.is_interactive
    }

    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn get_jobs(&self) -> Vec<Job> {
        Vec::new()
    }

    fn has_background_jobs(&self) -> bool {
        false
    }

    fn put_job_in_foreground(&mut self, _job_id: Option<JobId>) -> Result<Option<ExitStatus>> {
        Err(Error::no_job_control())
    }

    fn put_job_in_background(&mut self, _job_id: Option<JobId>) -> Result<()> {
        Err(Error::no_job_control())
    }

    fn kill_background_job(&mut self, job_id: u32) -> Result<Option<Job>> {
        // For compatibility with bash, return "no such job" instead of "no job
        // control"
        Err(Error::no_such_job(job_id.to_string()))
    }
}

pub fn create_simple_shell(config: ShellConfig) -> Result<Box<dyn Shell>> {
    let shell = SimpleShell::new(config)?;
    Ok(Box::new(shell))
}
