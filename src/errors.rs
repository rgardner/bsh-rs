//! Error module. See the [error-chain](https://crates.io/crates/error-chain) crate for details.

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        BuiltinError(message: String, code: i32) {
            description(message)
        }
    }
}
