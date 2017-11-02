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
extern crate odds;
extern crate rustyline;
extern crate wait_timeout;

#[cfg(test)]
extern crate rand;

pub use self::shell::{Shell, ShellConfig};

macro_rules! eprintln {
    ($($tt:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(&mut ::std::io::stderr(), $($tt)*);
    }}
}

mod builtins;
mod editor;
#[allow(missing_docs)]
pub mod errors;
mod parser;
pub mod shell;
