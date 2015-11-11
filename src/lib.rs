//! Bsh - Bob Shell
#![feature(path_relative_from)]

#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unused_import_braces, unused_qualifications)]

extern crate odds;
#[macro_use]
extern crate quick_error;
extern crate wait_timeout;

mod builtins;
mod error;
mod history;
pub mod shell;
pub mod parse;
