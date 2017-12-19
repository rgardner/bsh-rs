//! Error module. See the [error-chain](https://crates.io/crates/error-chain) crate for details.

error_chain! {
    foreign_links {
        Docopt(::docopt::Error);
        Io(::std::io::Error);
        Nix(::nix::Error);
        ReadlineError(::rustyline::error::ReadlineError);
    }

    errors {
        /// Generic syntax error containing offending line
        SyntaxError(line: String) {
            description("syntax error")
            display("syntax error: '{}'", line)
        }
        BuiltinCommandError(message: String, code: i32) {
            description(message)
        }
        CommandNotFoundError(command: String) {
            display("{}: command not found", command)
        }
        NoSuchJobError(job: String) {
            display("{}: no such job", job)
        }
    }
}
