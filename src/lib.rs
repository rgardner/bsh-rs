//! Bsh - Bob Shell

#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unused_import_braces, unused_qualifications)]

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

// necessary for `error-chain`
#![recursion_limit = "1024"]

#[macro_use]
extern crate nom;
extern crate odds;
#[macro_use]
extern crate error_chain;
extern crate wait_timeout;

pub use self::shell::Shell;
pub use self::parse::{ParseCommand, ParseJob};

mod builtins;
mod errors;
mod history;
pub mod parse;
pub mod shell;
