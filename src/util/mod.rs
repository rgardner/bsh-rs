use std::io;
use std::os::unix::prelude::*;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

/// BSH Utility Extensions for `ExitStatus`
pub trait BshExitStatusExt {
    /// Create an ExitStatus to indicate *successful* program execution.
    fn from_success() -> Self;

    /// Create an ExitStatus to indicate *unsuccessful* program execution.
    fn from_failure() -> Self;

    /// Create an ExitStatus from a status code
    fn from_status(code: i32) -> Self;
}

impl BshExitStatusExt for ExitStatus {
    /// # Example
    /// ```rust
    /// # extern crate bsh;
    /// # fn main() {
    /// use bsh::BshExitStatusExt;
    /// use std::process::ExitStatus;
    /// assert!(ExitStatus::from_success().success());
    /// # }
    /// ```
    fn from_success() -> Self {
        ExitStatus::from_status(0)
    }

    /// # Example
    /// ```rust
    /// # extern crate bsh;
    /// # fn main() {
    /// use bsh::BshExitStatusExt;
    /// use std::process::ExitStatus;
    /// assert!(!ExitStatus::from_failure().success());
    /// # }
    /// ```
    fn from_failure() -> Self {
        ExitStatus::from_status(1)
    }

    /// # Example
    /// ```rust
    /// # extern crate bsh;
    /// # fn main() {
    /// use bsh::BshExitStatusExt;
    /// use std::process::ExitStatus;
    /// assert!(ExitStatus::from_status(0).success());
    /// assert!(!ExitStatus::from_status(1).success());
    /// # }
    /// ```
    fn from_status(code: i32) -> Self {
        ExitStatus::from_raw(code << 8)
    }
}

pub fn get_terminal() -> RawFd {
    io::stdin().as_raw_fd()
}
