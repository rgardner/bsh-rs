//! Bsh - Bob Shell

#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]
#![feature(tool_lints)]

extern crate atty;
#[macro_use]
extern crate cfg_if;
extern crate dirs;
extern crate docopt;
extern crate failure;
extern crate lalrpop_util;
extern crate libc;
#[macro_use]
extern crate log;
#[cfg(unix)]
extern crate nix;
extern crate rustyline;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub use crate::errors::{Error, ErrorKind, Result};
pub use crate::shell::{create_shell, create_simple_shell, Shell, ShellConfig};
pub use crate::util::BshExitStatusExt;

macro_rules! log_if_err {
    ($result:expr) => {{
        if let Err(e) = $result {
            error!("{}", e);
        }
    }};
    ($result:expr, $fmt:expr) => {{
        if let Err(e) = $result {
            error!(concat!($fmt, ": {}"), e);
        }
    }};
    ($result:expr, $fmt:expr, $($arg:tt)*) => {{
        if let Err(e) = $result {
            error!(concat!($fmt, ": {}"), $($arg)*, e);
        }
    }};
}

mod builtins;
mod core;
mod editor;
#[allow(missing_docs)]
pub mod errors;
#[allow(unsafe_code)]
mod execute_command;
// TODO: remove this once the dust has settled
#[allow(missing_docs)]
mod shell;
mod util;
