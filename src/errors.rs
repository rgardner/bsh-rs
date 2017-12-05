//! Error module. See the [error-chain](https://crates.io/crates/error-chain) crate for details.

use parser;

error_chain! {
    links {
        Parser(parser::Error, parser::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        Nix(::nix::Error);
        ReadlineError(::rustyline::error::ReadlineError);
    }

    errors {
        BuiltinCommandError(message: String, code: i32) {
            description(message)
        }
    }
}
