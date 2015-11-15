use error::{self, Result};
use builtins::{BuiltinCommand, Error};
use shell::Shell;
use std::env;
use std::path::Path;

pub struct Cd;

impl BuiltinCommand for Cd {
    fn name() -> String {
        String::from("cd")
    }

    fn help() -> String {
        String::from("\
cd: cd [dir]
    Change the current directory to DIR. The variable $HOME is the default dir.
    If DIR is '-', then the current directory will be the variable $OLDPWD,
    which is the last working directory.")
    }

    fn run(_shell: &mut Shell, args: Vec<String>) -> Result<()> {
        let dir = match args.get(0).map(|x| &x[..]) {
            Some("~") | None =>
                try!(env::home_dir().ok_or(Error::InvalidArgs(String::from("cd: HOME not set"), 1))),
            Some("-") => match env::var_os("OLDPWD") {
                Some(val) => {
                    println!("{}", val.to_str().unwrap());
                    Path::new(val.as_os_str()).to_path_buf()
                }
                None => {
                    return Err(error::Error::BuiltinError(Error::InvalidArgs(String::from("cd: OLDPWD not set"), 1)));
                }
            },
            Some(val) => Path::new(val).to_path_buf(),
        };
        env::set_var("OLDPWD", try!(env::current_dir()));
        try!(env::set_current_dir(dir));
        Ok(())
    }
}
