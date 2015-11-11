use builtins;
use std::io;

quick_error! {
    #[derive(Debug)]
    pub enum BshError {
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
