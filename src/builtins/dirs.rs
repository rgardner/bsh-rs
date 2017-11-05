use errors::*;
use builtins;
use shell::Shell;
use std::env;
use std::path::Path;

pub struct Cd;

impl builtins::BuiltinCommand for Cd {
    const NAME: &'static str = builtins::CD_NAME;

    const HELP: &'static str = "\
cd: cd [dir]
    Change the current directory to DIR. The variable $HOME is the default dir.
    If DIR is '-', then the current directory will be the variable $OLDPWD,
    which is the last working directory.";

    fn run(_shell: &mut Shell, args: Vec<String>) -> Result<()> {
        let dir = match args.first().map(|x| &x[..]) {
            None => env::home_dir().ok_or(ErrorKind::BuiltinCommandError(
                "cd: HOME not set".to_string(),
                1,
            ))?,
            Some("-") => match env::var_os("OLDPWD") {
                Some(val) => {
                    println!("{}", val.to_str().unwrap());
                    Path::new(val.as_os_str()).to_path_buf()
                }
                None => {
                    bail!(ErrorKind::BuiltinCommandError(
                        "cd: OLDPWD not set".to_string(),
                        1
                    ));
                }
            },
            Some(val) => Path::new(val).to_path_buf(),
        };

        env::set_var("OLDPWD", env::current_dir()?);
        env::set_current_dir(dir)?;
        Ok(())
    }
}
