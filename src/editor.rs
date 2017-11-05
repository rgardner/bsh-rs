use errors::*;
use rustyline::{self, history, CompletionType, Config};
use rustyline::completion::FilenameCompleter;
use std::fmt;
use std::path::Path;
use std::str;

pub struct Editor {
    internal: rustyline::Editor<(FilenameCompleter)>,
    /// The total number of history items ever saved
    history_count: usize,
    history_capacity: usize,
}

impl Editor {
    pub fn with_capacity(capacity: usize) -> Editor {
        let config = Config::builder()
            .max_history_size(capacity)
            .history_ignore_space(true)
            .completion_type(CompletionType::Circular)
            .build();

        let mut internal = rustyline::Editor::with_config(config);
        internal.set_completer(Some(FilenameCompleter::new()));

        Editor {
            internal: internal,
            history_count: 0,
            history_capacity: capacity,
        }
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String> {
        let line = self.internal.readline(prompt)?;
        Ok(line)
    }

    pub fn load_history<P: AsRef<Path> + ?Sized>(&mut self, path: &P) -> Result<()> {
        self.internal.load_history(path)?;
        Ok(())
    }

    pub fn save_history<P: AsRef<Path> + ?Sized>(&mut self, path: &P) -> Result<()> {
        self.internal.save_history(path)?;
        Ok(())
    }

    pub fn add_history_entry(&mut self, job: &str) {
        if self.internal.add_history_entry(job) {
            self.history_count += 1;
        }
    }

    /// Get the history entry at an absolute position
    pub fn get_history_entry(&self, abs_pos: usize) -> Option<&String> {
        // map abs_pos to [0, self.history_capacity]
        let begin = self.history_count
            .checked_sub(self.history_capacity)
            .unwrap_or(0);
        if (abs_pos < begin) || (abs_pos > self.history_count) {
            return None;
        }

        self.internal.get_history_const().get(abs_pos - begin)
    }

    /// Set maximum number of remembered history entries.
    ///
    /// If `size` > current max size, retain last `size` entries.
    pub fn set_history_max_size(&mut self, size: usize) {
        self.internal.get_history().set_max_len(size);
        self.history_capacity = size;
    }

    pub fn get_history_count(&self) -> usize {
        self.history_count
    }

    pub fn clear_history(&mut self) {
        self.internal.clear_history();
        self.history_count = 0;
    }

    /// Perform history expansion.
    pub fn expand_history(&self, command: &mut String) -> Result<()> {
        if !command.starts_with('!') {
            return Ok(());
        }

        let arg = command[1..].to_string();
        let entry = match arg.parse::<isize>() {
            Ok(0) => None,
            Ok(n) if n > 0 => self.get_history_entry((n - 1) as usize),
            Ok(n) => self.history_count
                .checked_sub(n.wrapping_abs() as usize)
                .and_then(|i| self.get_history_entry(i)),
            Err(_) => self.internal
                .get_history_const()
                .search(&arg, self.history_count - 1, history::Direction::Reverse)
                .and_then(|idx| self.internal.get_history_const().get(idx)),
        };

        match entry {
            Some(line) => {
                command.clear();
                command.push_str(line);
            }
            None => {
                bail!(ErrorKind::BuiltinCommandError(
                    format!("{}: event not found", command),
                    1
                ));
            }
        }

        Ok(())
    }

    pub fn enumerate_history_entries(&self) -> EditorEnumerate {
        let start = self.history_count
            .checked_sub(self.history_capacity)
            .unwrap_or(0);
        EditorEnumerate {
            editor: self,
            pos: start,
        }
    }
}

impl fmt::Display for Editor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, e) in self.enumerate_history_entries() {
            write!(f, "\t{}\t{}\n", i + 1, e)?;
        }

        Ok(())
    }
}

impl fmt::Debug for Editor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "count: {}\n", self.history_count)?;
        write!(f, "capacity: {}\n", self.history_capacity)?;
        write!(f, "{}", self)
    }
}

pub struct EditorEnumerate<'a> {
    editor: &'a Editor,
    pos: usize,
}

impl<'a> Iterator for EditorEnumerate<'a> {
    type Item = (usize, &'a String);

    fn next(&mut self) -> Option<(usize, &'a String)> {
        let v = self.editor
            .get_history_entry(self.pos)
            .map(|e| (self.pos, e));
        if v.is_some() {
            self.pos += 1;
        }

        v
    }
}

impl<'a> fmt::Debug for EditorEnumerate<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pos: {}\n", self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alloc_history_state(capacity: usize, full: usize) -> Editor {
        assert!(full <= capacity);
        let mut state = Editor::with_capacity(capacity);
        for i in 0..full {
            state.add_history_entry(&format!("cmd{}", i));
        }
        state
    }

    #[test]
    fn init_with_capacity() {
        let capacity = 10;
        let state = Editor::with_capacity(capacity);
        assert!(state.internal.get_history_const().is_empty());
        assert_eq!(state.history_count, 0);
        assert_eq!(state.history_capacity, capacity);
    }

    #[test]
    fn clear() {
        let capacity = 10;
        let mut state = alloc_history_state(capacity, 5);
        state.clear_history();
        assert!(state.internal.get_history_const().is_empty());
        assert_eq!(state.history_count, 0);
        assert_eq!(state.history_capacity, capacity);
    }

    #[test]
    fn add_history_entry_duplicate() {
        let mut state = Editor::with_capacity(2);

        let item = "dup";
        state.add_history_entry(item);
        assert_eq!(state.internal.get_history_const().len(), 1);

        state.add_history_entry(item);
        assert_eq!(state.internal.get_history_const().len(), 1);
    }

    #[test]
    fn add_history_entry_rollover() {
        let mut state = alloc_history_state(10, 10);
        state.add_history_entry("extra");
        assert_eq!(state.history_count, 11);
    }

    #[test]
    fn expand_empty_command() {
        let mut buf = String::new();
        let state = alloc_history_state(1, 1);
        assert!(state.expand_history(&mut buf).is_ok());
        assert!(buf.is_empty());
    }

    #[test]
    fn expand_empty_history() {
        let state = alloc_history_state(0, 0);

        let mut buf = String::new();
        assert!(state.expand_history(&mut buf).is_ok());
        assert!(buf.is_empty());

        let first_cmd = "!1";
        let mut buf = first_cmd.to_string();
        assert!(state.expand_history(&mut buf).is_err());
        assert_eq!(buf.as_str(), first_cmd);

        let last_cmd = "!-1";
        let mut buf = String::from(last_cmd);
        assert!(state.expand_history(&mut buf).is_err());
        assert_eq!(buf, last_cmd);
    }

    #[test]
    fn expand_positive_nth_command() {
        let (cap, full) = (10, 10);
        let state = alloc_history_state(cap, full);
        for i in 0..full {
            let mut buf = format!("!{}", i + 1);
            assert!(state.expand_history(&mut buf).is_ok());
            assert_eq!(buf, format!("cmd{}", i));
        }
    }

    #[test]
    fn expand_negative_nth_command() {
        let (cap, full) = (10, 10);
        let state = alloc_history_state(cap, full);
        for i in 0..full {
            let mut buf = format!("!-{}", i + 1);
            assert!(state.expand_history(&mut buf).is_ok());
            assert_eq!(buf, format!("cmd{}", full - i - 1));
        }
    }

    #[test]
    fn expand_string() {
        let state = alloc_history_state(10, 10);

        let mut buf = String::from("!c");
        assert!(state.expand_history(&mut buf).is_ok());
        assert_eq!(buf, "cmd9");

        buf = String::from("!cmd1");
        assert!(state.expand_history(&mut buf).is_ok());
        assert_eq!(buf, "cmd1");
    }
}
