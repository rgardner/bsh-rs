//! Error module. See the [failure](https://crates.io/crates/failure) crate for details.

use std::fmt;
use std::result;

use failure::{Backtrace, Context, Fail};

/// Bsh result alias.
pub type Result<T> = result::Result<T, Error>;

/// Bsh error type.
#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    /// Returns the corresponding [`ErrorKind`] for this error.
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub(crate) fn syntax<T: AsRef<str>>(line: T) -> Self {
        Error::from(ErrorKind::Syntax(line.as_ref().to_string()))
    }

    pub(crate) fn builtin_command<T: AsRef<str>>(message: T, code: i32) -> Self {
        Error::from(ErrorKind::BuiltinCommand {
            message: message.as_ref().to_string(),
            code,
        })
    }

    pub(crate) fn command_not_found<T: AsRef<str>>(command: T) -> Self {
        Error::from(ErrorKind::CommandNotFound(command.as_ref().to_string()))
    }

    pub(crate) fn no_such_job<T: AsRef<str>>(job: T) -> Self {
        Error::from(ErrorKind::NoSuchJob(job.as_ref().to_string()))
    }

    pub(crate) fn no_job_control() -> Self {
        Error::from(ErrorKind::NoJobControl)
    }

    #[cfg(windows)]
    pub(crate) fn not_supported<T: AsRef<str>>(message: T) -> Self {
        Error::from(ErrorKind::NotSupported(message.as_ref().to_string()))
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.ctx.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.ctx.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ctx.fmt(f)
    }
}

/// Bsh error kinds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// Syntax error.
    Syntax(String),
    /// Builtin command error.
    BuiltinCommand {
        /// Error message.
        message: String,
        /// Error code.
        code: i32,
    },
    /// Command not found error.
    CommandNotFound(String),
    /// No such job error.
    NoSuchJob(String),
    /// Job control not available error.
    NoJobControl,
    /// Operation not supported error.
    NotSupported(String),
    /// Underlying error from the Docopt crate.
    Docopt,
    /// I/O error.
    Io,
    /// Underlying error from the Nix crate.
    Nix,
    /// Underlying error from the Readline crate.
    Readline,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ErrorKind::Syntax(ref line) => write!(f, "syntax error: '{}'", line),
            ErrorKind::BuiltinCommand { ref message, .. } => write!(f, "{}", message),
            ErrorKind::CommandNotFound(ref line) => write!(f, "{}: command not found", line),
            ErrorKind::NoSuchJob(ref job) => write!(f, "{}: no such job", job),
            ErrorKind::NoJobControl => write!(f, "no job control"),
            ErrorKind::NotSupported(ref message) => write!(f, "{}", message),
            ErrorKind::Docopt => write!(f, "Docopt error occurred"),
            ErrorKind::Io => write!(f, "I/O error occurred"),
            ErrorKind::Nix => write!(f, " Nix error occurred"),
            ErrorKind::Readline => write!(f, "Readline error occurred"),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::from(Context::new(kind))
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(ctx: Context<ErrorKind>) -> Error {
        Error { ctx }
    }
}
