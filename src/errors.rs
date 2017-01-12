//! Error module. See the [error-chain](https://crates.io/crates/error-chain) crate for details.

use parse;

error_chain! {
    links {
        Parse(parse::Error, parse::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        ReadlineError(::rustyline::error::ReadlineError);
    }

    errors {
        BuiltinCommandError(message: String, code: i32) {
            description(message)
        }
    }
}
