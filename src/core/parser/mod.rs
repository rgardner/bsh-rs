//! BSH Parser

use self::grammar::CommandParser;
use crate::errors::{Error, Result};

pub mod ast;
#[allow(dead_code, unused_qualifications)]
#[allow(clippy::all)]
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
