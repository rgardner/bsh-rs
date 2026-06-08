use std::process::ExitStatus;

#[cfg(unix)]
pub mod unix;

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
    /// # Examples
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

    /// # Examples
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

    /// # Examples
    /// ```rust
    /// # extern crate bsh;
    /// # fn main() {
    /// use bsh::BshExitStatusExt;
    /// use std::process::ExitStatus;
    /// assert!(ExitStatus::from_status(0).success());
    /// assert!(!ExitStatus::from_status(1).success());
    /// # }
    /// ```
    #[cfg(unix)]
    fn from_status(code: i32) -> Self {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    /// # Examples
    /// ```rust
    /// # extern crate bsh;
    /// # fn main() {
    /// use bsh::BshExitStatusExt;
    /// use std::process::ExitStatus;
    /// assert!(ExitStatus::from_status(0).success());
    /// assert!(!ExitStatus::from_status(1).success());
    /// # }
    /// ```
    #[cfg(windows)]
    fn from_status(code: i32) -> Self {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw((code as u32) << 8)
    }
}
