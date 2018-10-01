use std::io;
use std::os::unix::prelude::*;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

pub use self::unix::isatty;

pub mod unix;

pub trait VecExt<T> {
    /// Replace element at `index` with the result of the closure.
    fn update<F>(&mut self, index: usize, f: F)
    where
        F: Fn(T) -> T;
}

impl<T> VecExt<T> for Vec<T> {
    fn update<F>(&mut self, index: usize, f: F)
    where
        F: Fn(T) -> T,
    {
        let entry = self.swap_remove(index);
        self.push(f(entry));
        let last_index = self.len() - 1;
        self.swap(index, last_index);
    }
}

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
    fn from_status(code: i32) -> Self {
        ExitStatus::from_raw(code << 8)
    }
}

pub fn get_terminal() -> RawFd {
    io::stdin().as_raw_fd()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_update() {
        let mut primes = vec![1, 2, 3];
        primes.update(0, |p| p * 2);
        assert_eq!(primes, vec![2, 2, 3]);
    }
}
