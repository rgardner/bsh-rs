//! BSH Parser

use self::grammar::CommandParser;
use errors::{Error, Result};

pub mod ast;
#[allow(dead_code, unused_qualifications)]
#[cfg_attr(feature = "clippy", allow(clippy))]
#[cfg_attr(feature = "cargo-clippy", allow(clippy))]
#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

#[derive(Debug)]
pub struct Command {
    pub input: String,
    pub inner: ast::Command,
}

impl Command {
    pub fn new(input: &str, inner: ast::Command) -> Self {
        Self {
            input: input.to_string(),
            inner,
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        let result = CommandParser::new()
            .parse(input)
            .map_err(|_| Error::syntax(input))
            .map(|inner| Command {
                input: input.into(),
                inner,
            });
        debug!("parsed Command: {:?}", result);
        result
    }
}
