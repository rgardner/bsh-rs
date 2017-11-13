//! BSH Parser

pub mod ast;
#[allow(dead_code, unused_qualifications)]
#[cfg_attr(feature = "clippy", allow(clippy))]
#[cfg_attr(feature = "cargo-clippy", allow(clippy))]
#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

error_chain! {
    errors {
        /// Generic syntax error containing offending line
        SyntaxError(l: String) {
            description("syntax error")
            display("syntax error: '{}'", l)
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub input: String,
    pub inner: ast::Command,
}

impl Command {
    pub fn parse(input: &str) -> Result<Command> {
        let result = grammar::parse_Command(input)
            .map_err(|_| ErrorKind::SyntaxError(input.into()).into())
            .map(|inner| {
                Command {
                    input: input.into(),
                    inner,
                }
            });
        debug!("parsed Command: {:?}", result);
        result
    }
}
