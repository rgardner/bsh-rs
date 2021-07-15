//! BSH Parser

use lalrpop_util::lalrpop_mod;
use log::debug;

use self::grammar::CommandParser;
use crate::errors::{Error, Result};

pub mod ast;
#[rustfmt::skip]
lalrpop_mod!(#[allow(clippy::all, unused_qualifications)] grammar, "/core/parser/grammar.rs");

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
