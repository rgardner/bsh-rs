use errors::*;
use builtins;
use shell::Shell;

pub struct History;

impl builtins::BuiltinCommand for History {
    fn name() -> &'static str {
        builtins::HISTORY_NAME
    }

    fn help() -> &'static str {
        "\
history: history [-c] [-s size] [n]
    Display the history list with line numbers. Argument of N
    says to list only the last N lines. The `-c' option causes
    the history list to be cleared by deleting all of the entries.
    The `-s' option sets the size of the history list."
    }

    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if let None = args.first() {
            println!("{}", shell.history);
            return Ok(());
        }
        let arg = args.first().unwrap();
        match &**arg {
            "-c" => shell.history.clear(),
            "-s" => {
                if let Some(s) = args.get(2) {
                    if let Ok(n) = s.parse::<usize>() {
                        shell.history.set_size(n);
                    }
                }
            }
            s => {
                match s.parse::<usize>() {
                    Ok(n) => println!("{}", shell.history.display(n)),
                    Err(_) => {
                        let msg = format!("history: {}: nonnegative numeric argument required", s);
                        bail!(ErrorKind::BuiltinCommandError(msg, 1));
                    }
                }
            }
        }
        Ok(())
    }
}
