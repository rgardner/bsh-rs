use nix::unistd;

pub fn isatty() -> bool {
    let temp_result = unistd::isatty(super::get_terminal());
    log_if_err!(temp_result, "unistd::isatty");
    temp_result.unwrap_or(false)
}
