use std::{io, os::unix::prelude::*};

pub fn get_terminal() -> RawFd {
    io::stdin().as_raw_fd()
}
