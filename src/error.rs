//! Defines Error and Result types for use with the bsh library.
use builtins;
use std::io;
use std::result;

quick_error! {
    /// Specialized Error type for all bsh operations.
    #[derive(Debug)]
    pub enum Error {
        /// Wrapper around io::Error
        Io(err: io::Error) {
            display("{}", err)
            description(err.description())
            cause(err)
            from()
        }
        /// Wrapper around builtins::Error
        BuiltinError(err: builtins::Error) {
            display("{}", err)
            description(err.description())
            cause(err)
            from()
        }
    }
}


/// A specialized Result type for bsh operations.
///
/// This type is used because executing shell commands can cause errors.
///
/// Like std::io::Result, users of this alias should generally use bsh_rs::Result instead of
/// importing this directly.
pub type Result<T> = result::Result<T, Error>;
