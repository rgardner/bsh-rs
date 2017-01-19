use errors::*;
use rustyline::{Config, CompletionType, Editor, history};
use rustyline::completion::FilenameCompleter;
use std::fmt;
use std::str;
use nom::IResult;

pub struct HistoryState {
    internal: Editor<(FilenameCompleter)>,
    /// The total number of history items ever saved
    count: usize,
    capacity: usize,
}

impl HistoryState {
    pub fn with_capacity(capacity: usize) -> HistoryState {
        let config = Config::builder()
            .max_history_size(capacity)
            .history_ignore_space(true)
            .completion_type(CompletionType::Circular)
            .build();

        let mut internal = Editor::with_config(config);
        internal.set_completer(Some(FilenameCompleter::new()));

        HistoryState {
            internal: internal,
            count: 0,
            capacity: capacity,
        }
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String> {
        let line = try!(self.internal.readline(prompt));
        Ok(line)
    }

    pub fn push(&mut self, job: &str) {
        if self.internal.add_history_entry(job) {
            self.count += 1;
        }
    }

    /// Get the history entry at an absolute position
    pub fn get(&self, abs_pos: usize) -> Option<&String> {
        // map abs_pos to [0, self.capacity]
        let begin = self.count.checked_sub(self.capacity).unwrap_or(0);
        if (abs_pos < begin) || (abs_pos > self.count) {
            return None;
        }

        self.internal.get_history_const().get(abs_pos - begin)
    }

    /// Set maximum number of remembered history entries.
    ///
    /// If `size` > current max size, retain last `size` entries.
    pub fn set_size(&mut self, size: usize) {
        self.internal.get_history().set_max_len(size);
        self.capacity = size;
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn clear(&mut self) {
        self.internal.clear_history();
        self.count = 0;
    }

    /// Perform history expansion.
    pub fn expand(&self, command: &mut String) -> Result<()> {
        named!(event<&str>, map_res!(preceded!(tag!("!"), is_not!(" ")), str::from_utf8));
        let input = command.clone();
        let arg = match event(input.as_bytes()) {
            IResult::Done(_, a) => a,
            _ => return Ok(()),
        };

        let entry = match arg.parse::<isize>() {
            Ok(0) => None,
            Ok(n) if n > 0 => self.get((n - 1) as usize),
            Ok(n) => self.count.checked_sub(n.wrapping_abs() as usize).and_then(|i| self.get(i)),
            Err(_) => {
                self.internal
                    .get_history_const()
                    .search(arg, self.count - 1, history::Direction::Reverse)
                    .and_then(|idx| self.internal.get_history_const().get(idx))
            }
        };

        match entry {
            Some(line) => {
                command.clear();
                command.push_str(&line);
            }
            None => {
                bail!(ErrorKind::BuiltinCommandError(format!("{}: event not found", command), 1));
            }
        }

        Ok(())
    }

    pub fn enumerate(&self) -> HistoryStateEnumerate {
        let start = self.count.checked_sub(self.capacity).unwrap_or(0);
        HistoryStateEnumerate {
            history: self,
            pos: start,
        }
    }
}

impl fmt::Display for HistoryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, e) in self.enumerate() {
            try!(write!(f, "\t{}\t{}\n", i + 1, e));
        }

        Ok(())
    }
}

impl fmt::Debug for HistoryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "count: {}\n", self.count));
        try!(write!(f, "capacity: {}\n", self.capacity));
        write!(f, "{}", self)
    }
}

pub struct HistoryStateEnumerate<'a> {
    history: &'a HistoryState,
    pos: usize,
}

impl<'a> Iterator for HistoryStateEnumerate<'a> {
    type Item = (usize, &'a String);

    fn next(&mut self) -> Option<(usize, &'a String)> {
        if self.pos < self.history.count {
            let v = self.history.get(self.pos).map(|e| (self.pos, e));
            self.pos += 1;
            v
        } else {
            None
        }
    }
}

impl<'a> fmt::Debug for HistoryStateEnumerate<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pos: {}\n", self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alloc_history_state(capacity: usize, full: usize) -> HistoryState {
        assert!(full <= capacity);
        let mut state = HistoryState::with_capacity(capacity);
        for i in 0..full {
            state.push(&format!("cmd{}", i));
        }
        state
    }

    #[test]
    fn init_with_capacity() {
        let capacity = 10;
        let state = HistoryState::with_capacity(capacity);
        assert!(state.internal.get_history_const().is_empty());
        assert_eq!(state.count, 0);
        assert_eq!(state.capacity, capacity);
    }

    #[test]
    fn clear() {
        let capacity = 10;
        let mut state = alloc_history_state(capacity, 5);
        state.clear();
        assert!(state.internal.get_history_const().is_empty());
        assert_eq!(state.count, 0);
        assert_eq!(state.capacity, capacity);
    }

    #[test]
    fn push_duplicate() {
        let mut state = HistoryState::with_capacity(2);

        let item = "dup";
        state.push(item);
        assert_eq!(state.internal.get_history_const().len(), 1);

        state.push(item);
        assert_eq!(state.internal.get_history_const().len(), 1);
    }

    #[test]
    fn push_rollover() {
        let mut state = alloc_history_state(10, 10);
        state.push("extra");
        assert_eq!(state.count, 11);
    }

    #[test]
    fn expand_empty_command() {
        let mut buf = String::new();
        let state = alloc_history_state(1, 1);
        assert!(state.expand(&mut buf).is_ok());
        assert!(buf.is_empty());
    }

    #[test]
    fn expand_empty_history() {
        let state = alloc_history_state(0, 0);

        let mut buf = String::new();
        assert!(state.expand(&mut buf).is_ok());
        assert!(buf.is_empty());

        let first_cmd = "!1";
        let mut buf = first_cmd.to_string();
        assert!(state.expand(&mut buf).is_err());
        assert_eq!(buf.as_str(), first_cmd);

        let last_cmd = "!-1";
        let mut buf = String::from(last_cmd);
        assert!(state.expand(&mut buf).is_err());
        assert_eq!(buf, last_cmd);
    }

    #[test]
    fn expand_positive_nth_command() {
        let (cap, full) = (10, 10);
        let state = alloc_history_state(cap, full);
        for i in 0..full {
            let mut buf = format!("!{}", i + 1);
            assert!(state.expand(&mut buf).is_ok());
            assert_eq!(buf, format!("cmd{}", i));
        }
    }

    #[test]
    fn expand_negative_nth_command() {
        let (cap, full) = (10, 10);
        let state = alloc_history_state(cap, full);
        for i in 0..full {
            let mut buf = format!("!-{}", i + 1);
            assert!(state.expand(&mut buf).is_ok());
            assert_eq!(buf, format!("cmd{}", full - i - 1));
        }
    }

    #[test]
    fn expand_string() {
        let state = alloc_history_state(10, 10);

        let mut buf = String::from("!c");
        assert!(state.expand(&mut buf).is_ok());
        assert_eq!(buf, "cmd9");

        buf = String::from("!cmd1");
        assert!(state.expand(&mut buf).is_ok());
        assert_eq!(buf, "cmd1");
    }
}
