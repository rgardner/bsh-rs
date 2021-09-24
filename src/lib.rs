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

pub use crate::errors::{Error, ErrorKind, Result};
pub use crate::shell::{create_shell, create_simple_shell, Shell, ShellConfig};
pub use crate::util::BshExitStatusExt;

macro_rules! log_if_err {
    ($result:expr) => {{
        if let Err(e) = $result {
            log::error!("{}", e);
        }
    }};
    ($result:expr, $fmt:expr) => {{
        if let Err(e) = $result {
            log::error!(concat!($fmt, ": {}"), e);
        }
    }};
    ($result:expr, $fmt:expr, $($arg:tt)*) => {{
        if let Err(e) = $result {
            log::error!(concat!($fmt, ": {}"), $($arg)*, e);
        }
    }};
}

mod builtins;
mod core;
mod editor;
pub mod errors;
#[allow(unsafe_code)]
mod execute_command;
mod shell;
mod util;
