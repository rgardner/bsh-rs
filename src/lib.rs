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
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate dirs;
extern crate docopt;
extern crate failure;
extern crate lalrpop_util;
#[macro_use]
extern crate log;
extern crate nix;
extern crate rustyline;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub use errors::{Error, ErrorKind, Result};
pub use shell::shell::{Shell, ShellConfig};
pub use util::BshExitStatusExt;

macro_rules! log_if_err {
    ($result:expr) => {{
        if let Err(e) = $result {
            error!("{}", e);
        }
    }};
    ($result:expr, $fmt:expr, $($arg:tt)*) => {{
        if let Err(e) = $result {
            error!(concat!($fmt, ": {}"), $($arg)*, e);
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

mod core;
#[allow(missing_docs)]
pub mod errors;
mod shell;
mod util;
