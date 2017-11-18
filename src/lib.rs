//! Bsh - Bob Shell

#![deny(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
        trivial_numeric_casts, unsafe_code, unused_import_braces, unused_qualifications)]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
// necessary for `error-chain`
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate lalrpop_util;
#[macro_use]
extern crate log;
extern crate odds;
extern crate rustyline;

#[cfg(test)]
extern crate rand;

pub use self::shell::{Shell, ShellConfig};

mod builtins;
mod editor;
#[allow(missing_docs)]
pub mod errors;
mod execute_command;
mod job_control;
mod parser;
pub mod shell;
