use errors::*;
use builtins;
use history::HistoryState;
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
            print!("{}", shell.history);
            return Ok(());
        }

        match &**args.first().unwrap() {
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
                    Ok(n) => println!("{}", history_display(&shell.history, n)),
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

pub fn history_display(state: &HistoryState, n_last_entries: usize) -> String {
    let num_to_skip = state.count().checked_sub(n_last_entries).unwrap_or(0);
    state.enumerate()
        .skip(num_to_skip)
        .map(|(i, e)| format!("\t{}\t{}", i + 1, e))
        .collect::<Vec<String>>()
        .join("\n")
}
