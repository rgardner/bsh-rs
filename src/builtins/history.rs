use crate::{
    builtins::{self, prelude::*},
    editor::Editor,
};

pub struct History;

impl builtins::BuiltinCommand for History {
    const NAME: &'static str = builtins::HISTORY_NAME;

    const HELP: &'static str = "\
history: history [-c] [-s size] [n]
    Display the history list with line numbers. Argument of N
    says to list only the last N lines. The `-c' option causes
    the history list to be cleared by deleting all of the entries.
    The `-s' option sets the size of the history list.";

    fn run<T: AsRef<str>>(shell: &mut dyn Shell, args: &[T], stdout: &mut dyn Write) -> Result<()> {
        if args.is_empty() {
            write!(stdout, "{}", shell.editor()).context(ErrorKind::Io)?;
            return Ok(());
        }

        match args.first().unwrap().as_ref() {
            "-c" => shell.editor_mut().clear_history(),
            "-s" => {
                if let Some(s) = args.get(2) {
                    if let Ok(n) = s.as_ref().parse::<usize>() {
                        shell.editor_mut().set_history_max_size(n);
                    }
                }
            }
            s => match s.parse::<usize>() {
                Ok(n) => writeln!(stdout, "{}", history_display(shell.editor(), n))
                    .context(ErrorKind::Io)?,
                Err(_) => {
                    let msg = format!("history: {}: nonnegative numeric argument required", s);
                    return Err(Error::builtin_command(msg, 1));
                }
            },
        }
        Ok(())
    }
}

pub fn history_display(state: &Editor, n_last_entries: usize) -> String {
    let num_to_skip = state.get_history_count().saturating_sub(n_last_entries);
    state
        .enumerate_history_entries()
        .skip(num_to_skip)
        .map(|(i, e)| format!("\t{}\t{}", i + 1, e))
        .collect::<Vec<String>>()
        .join("\n")
}
