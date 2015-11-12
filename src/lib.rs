//! Bsh - Bob Shell
#![cfg_attr(feature="unstable", feature(path_relative_from))]

#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unused_import_braces, unused_qualifications)]

#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate odds;
#[macro_use]
extern crate quick_error;
extern crate wait_timeout;

mod builtins;
pub mod error;
mod history;
pub mod shell;
pub mod parse;
