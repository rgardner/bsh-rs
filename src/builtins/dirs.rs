use std::env;
use std::path::Path;

use dirs;

use crate::builtins::{self, prelude::*};

pub struct Cd;

impl builtins::BuiltinCommand for Cd {
    const NAME: &'static str = builtins::CD_NAME;

    const HELP: &'static str = "\
cd: cd [dir]
    Change the current directory to DIR. The variable $HOME is the default dir.
    If DIR is '-', then the current directory will be the variable $OLDPWD,
    which is the last working directory.";

    fn run<T: AsRef<str>>(
        _shell: &mut dyn Shell,
        args: &[T],
        stdout: &mut dyn Write,
    ) -> Result<()> {
        let dir = match args.first().map(|arg| arg.as_ref()) {
            None => {
                dirs::home_dir().ok_or_else(|| Error::builtin_command("cd: HOME not set", 1))?
            }
            Some("-") => match env::var_os("OLDPWD") {
                Some(path) => {
                    let unicode_path = path
                        .to_str()
                        .ok_or_else(|| Error::builtin_command("invalid Unicode", 1))?;
                    stdout
                        .write_all(unicode_path.as_bytes())
                        .context(ErrorKind::Io)?;
                    Path::new(path.as_os_str()).to_path_buf()
                }
                None => {
                    return Err(Error::builtin_command("cd: OLDPWD not set", 1));
                }
            },
            Some(val) => Path::new(val).to_path_buf(),
        };

        env::set_var("OLDPWD", env::current_dir().context(ErrorKind::Io)?);
        env::set_current_dir(dir).context(ErrorKind::Io)?;
        Ok(())
    }
}
