//! Bsh - Bob Shell
#![cfg_attr(feature="unstable", feature(path_relative_from))]

#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unused_import_braces, unused_qualifications)]

#![cfg_attr(feature="dev", allow(unstable_features))]
#![cfg_attr(feature="dev", feature(plugin))]
#![cfg_attr(feature="dev", plugin(clippy))]

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
mod history;
mod errors;
pub mod shell;
pub mod parse;
