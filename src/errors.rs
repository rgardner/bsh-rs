//! Error module. See the [failure](https://crates.io/crates/failure) crate for details.

use std::fmt;
use std::result;

use failure::{Backtrace, Context, Fail};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub(crate) fn syntax<T: AsRef<str>>(line: T) -> Error {
        Error::from(ErrorKind::Syntax(line.as_ref().to_string()))
    }

    pub(crate) fn builtin_command<T: AsRef<str>>(message: T, code: i32) -> Error {
        Error::from(ErrorKind::BuiltinCommand {
            message: message.as_ref().to_string(),
            code,
        })
    }

    pub(crate) fn command_not_found<T: AsRef<str>>(command: T) -> Error {
        Error::from(ErrorKind::CommandNotFound(command.as_ref().to_string()))
    }

    pub(crate) fn no_such_job<T: AsRef<str>>(job: T) -> Error {
        Error::from(ErrorKind::NoSuchJob(job.as_ref().to_string()))
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.ctx.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.ctx.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.ctx.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Syntax(String),
    BuiltinCommand { message: String, code: i32 },
    CommandNotFound(String),
    HistoryFileNotFound,
    NoSuchJob(String),
    Docopt,
    Io,
    Nix,
    Readline,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorKind::Syntax(ref line) => write!(f, "syntax error: '{}'", line),
            ErrorKind::BuiltinCommand { ref message, .. } => write!(f, "{}", message),
            ErrorKind::CommandNotFound(ref line) => write!(f, "{}: command not found", line),
            ErrorKind::HistoryFileNotFound => write!(f, "history file not found"),
            ErrorKind::NoSuchJob(ref job) => write!(f, "{}: no such job", job),
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
