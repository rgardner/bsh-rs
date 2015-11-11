use builtins;
use std::io;

quick_error! {
    #[derive(Debug)]
    pub enum BshError {
        /// IO Error
        Io(err: io::Error) {
            from()
        }
        BuiltinError(err: builtins::Error) {
            from()
        }
    }
}
