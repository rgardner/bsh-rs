//! BSH Parser

use self::grammar::CommandParser;
use errors::*;

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
    pub fn parse(input: &str) -> Result<Command> {
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
