use std::env;

use shell::builtins::{self, prelude::*};

pub struct Declare;

impl builtins::BuiltinCommand for Declare {
    const NAME: &'static str = builtins::DECLARE_NAME;

    const HELP: &'static str = "\
declare: declare [name[=value] ...]
    Declare a variable and assign it a value.";

    fn run(_shell: &mut Shell, args: Vec<String>, _stdout: &mut Write) -> Result<()> {
        let mut bad_args = Vec::new();
        for arg in args {
            let key_value: Vec<&str> = arg.splitn(2, '=').collect();
            match key_value.first() {
                Some(&"") | None => bad_args.push(arg.clone()),
                Some(s) => env::set_var(s, key_value.get(1).unwrap_or(&"")),
            }
        }

        if !bad_args.is_empty() {
            let msg = bad_args
                .iter()
                .map(|arg| format!("declare: {} is not a valid identifier", arg))
                .collect::<Vec<String>>()
                .join("\n");
            return Err(Error::builtin_command(msg, 1));
        }

        Ok(())
    }
}

pub struct Unset;

impl builtins::BuiltinCommand for Unset {
    const NAME: &'static str = builtins::UNSET_NAME;

    const HELP: &'static str = "\
unset: unset [name ...]
    For each name, remove the corresponding variable.";

    fn run(_shell: &mut Shell, args: Vec<String>, _stdout: &mut Write) -> Result<()> {
        let mut bad_args = Vec::new();
        for arg in args {
            if arg.is_empty() || arg.contains('=') {
                bad_args.push(arg.clone());
            } else {
                env::remove_var(arg);
            }
        }

        if !bad_args.is_empty() {
            let msg = bad_args
                .iter()
                .map(|arg| format!("unset: {} is not a valid identifier", arg))
                .collect::<Vec<String>>()
                .join("\n");
            return Err(Error::builtin_command(msg, 1));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::io;

    use shell::{
        builtins::BuiltinCommand,
        shell::{Shell, ShellConfig},
    };

    macro_rules! generate_unique_env_key {
        () => {
            format!("KEY_LINE{}_COLUMN{}", line!(), column!())
        };
    }

    #[test]
    fn declare_invalid_identifier() {
        let mut shell = Shell::new(ShellConfig::noninteractive()).unwrap();

        assert!(Declare::run(&mut shell, vec!["".into()], &mut io::sink()).is_err());
        assert!(Declare::run(&mut shell, vec!["=FOO".into()], &mut io::sink()).is_err());

        let key = generate_unique_env_key!();
        let value = "bar";
        assert!(
            Declare::run(
                &mut shell,
                vec!["=baz".into(), format!("{}={}", key, value), "=baz".into()],
                &mut io::sink(),
            ).is_err()
        );
        assert_eq!(env::var(key).unwrap(), value);
    }

    #[test]
    fn declare_assignment() {
        let mut shell = Shell::new(ShellConfig::noninteractive()).unwrap();

        let key = generate_unique_env_key!();
        assert!(Declare::run(&mut shell, vec![key.clone()], &mut io::sink()).is_ok());
        assert_eq!(&env::var(&key).unwrap(), "");

        let value1 = "bar";
        assert!(
            Declare::run(
                &mut shell,
                vec![format!("{}={}", key, value1)],
                &mut io::sink(),
            ).is_ok()
        );
        assert_eq!(env::var(&key).unwrap(), value1);

        let value2 = "baz";
        assert!(
            Declare::run(
                &mut shell,
                vec![format!("{}={}", key, value2)],
                &mut io::sink(),
            ).is_ok()
        );
        assert_eq!(env::var(&key).unwrap(), value2);
    }

    #[test]
    fn declare_multiple_assignments() {
        let mut shell = Shell::new(ShellConfig::noninteractive()).unwrap();

        let key1 = generate_unique_env_key!();
        let key2 = generate_unique_env_key!();
        let value = "baz";
        assert!(
            Declare::run(
                &mut shell,
                vec![format!("{}={}", key1, value), format!("{}={}", key2, value)],
                &mut io::sink(),
            ).is_ok()
        );
        assert_eq!(env::var(&key1).unwrap(), value);
        assert_eq!(env::var(&key2).unwrap(), value);
    }

    #[test]
    fn unset_invalid_identifier() {
        let mut shell = Shell::new(ShellConfig::noninteractive()).unwrap();
        let key = generate_unique_env_key!();
        assert!(Declare::run(&mut shell, vec![key.clone()], &mut io::sink()).is_ok());
        assert!(
            Unset::run(
                &mut shell,
                vec!["".into(), key.clone(), "=FOO".into()],
                &mut io::sink(),
            ).is_err()
        );
        assert!(env::var(key).is_err());
    }

    #[test]
    fn unset_multiple_assignments() {
        let mut shell = Shell::new(ShellConfig::noninteractive()).unwrap();
        let key1 = generate_unique_env_key!();
        let key2 = generate_unique_env_key!();
        assert!(
            Declare::run(
                &mut shell,
                vec![key1.clone(), key2.clone()],
                &mut io::sink(),
            ).is_ok()
        );

        assert!(
            Unset::run(
                &mut shell,
                vec![key1.clone(), key2.clone()],
                &mut io::sink(),
            ).is_ok()
        );
        assert!(env::var(key1).is_err());
        assert!(env::var(key2).is_err());
    }
}