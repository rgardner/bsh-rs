use error;
use builtins;
use std::cmp::{self, Ordering};
use std::fmt;
use std::str::{self, FromStr};
use nom::{digit, IResult};

#[derive(Debug)]
struct HistoryEntry {
    line: String,
    timestamp: usize,
}

#[derive(Debug)]
pub struct HistoryState {
    entries: Vec<HistoryEntry>,
    /// The total number of history items ever saved.
    count: usize,
}

impl HistoryState {
    pub fn with_capacity(capacity: usize) -> HistoryState {
        HistoryState {
            entries: Vec::with_capacity(capacity),
            count: 0,
        }
    }

    pub fn push(&mut self, job: &str) {
        let idx = self.count % self.entries.capacity();
        let entry = HistoryEntry {
            line: job.to_owned(),
            timestamp: self.count + 1,
        };
        match self.entries.get(idx) {
            Some(_) => self.entries[idx] = entry,
            None => self.entries.insert(idx, entry),
        }
        self.count += 1;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.count = 0;
    }

    pub fn set_size(&mut self, size: usize) {
        if size == 0 {
            return;
        }
        let capacity = self.entries.capacity();
        match size.cmp(&capacity) {
            Ordering::Equal => return,
            Ordering::Less => {
                self.entries.clear();
                self.entries.shrink_to_fit();
                self.entries.reserve_exact(size);
            }
            Ordering::Greater => {
                // Empty vectors: reserve_exact(size) = || capacity = size;
                // Nonempty vectors: reserve_exact(size) = || capacity += size;
                let reserve = if self.count > 0 {
                    size - self.entries.capacity()
                } else {
                    size
                };
                self.entries.reserve_exact(reserve);
            }
        }
    }

    pub fn display(&self, last: usize) -> String {
        let len = self.entries.len();
        let skip = len - cmp::min(last, len);
        let idx = self.count % self.entries.capacity();
        let (end, start) = self.entries.split_at(idx);
        start.iter()
             .chain(end.iter())
             .skip(skip)
             .map(|e| format!("\t{}\t{}", e.timestamp.clone(), e.line.clone()))
             .collect::<Vec<String>>()
             .join("\n")
    }


    /// Perform history expansion.
    pub fn expand(&self, command: &mut String) -> error::Result<()> {
        named!(unum<isize>, map_res!(map_res!(digit, str::from_utf8), FromStr::from_str));
        named!(inum<isize>, alt!(unum | chain!(tag!("-") ~ n: unum, || { -n })));
        named!(event<isize>, preceded!(tag!("!"), inum));
        let raw_n = match event(command.as_bytes()) {
            IResult::Done(_, n) => n,
            _ => return Ok(()),
        };

        let n: usize = match raw_n {
            0 => {
                let msg = format!("{}: event not found", command);
                return Err(error::Error::BuiltinError(builtins::Error::InvalidArgs(msg, 1)));
            }
            n if n < 0 => (n + (self.entries.len() as isize)) as usize,
            n => (n - 1) as usize,
        };
        match self.entries.get(n) {
            Some(entry) => {
                command.clear();
                command.push_str(&entry.line);
            }
            None => {
                let msg = format!("{}: event not found", command);
                return Err(error::Error::BuiltinError(builtins::Error::InvalidArgs(msg, 1)));
            }
        };
        Ok(())
    }
}

impl fmt::Display for HistoryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display(self.count))
    }
}

impl fmt::Display for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t{}\t{}", self.timestamp, self.line)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parse::ParseJob;

    fn alloc_history_state(capacity: usize, full: usize) -> HistoryState {
        assert!(full <= capacity);
        let mut state = HistoryState::with_capacity(capacity);
        let job = ParseJob::parse("cmd").unwrap().unwrap();
        for _ in 0..full {
            state.push(&job.clone());
        }
        state
    }

    #[test]
    fn init_with_capacity() {
        let capacity = 10;
        let state = HistoryState::with_capacity(capacity);
        assert_eq!(capacity, state.entries.capacity());
        assert!(state.entries.is_empty());
        assert_eq!(0, state.count);
    }

    #[test]
    fn clear() {
        let capacity = 10;
        let mut state = alloc_history_state(capacity, 5);
        state.clear();
        assert!(state.entries.is_empty());
        assert_eq!(capacity, state.entries.capacity());
        assert_eq!(0, state.count);
    }

    #[test]
    fn set_size_equal() {
        let init_capacity = 10;

        // empty history state
        let mut state = HistoryState::with_capacity(init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());
        state.set_size(init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());

        // full history state
        let mut state = alloc_history_state(init_capacity, init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());
        state.set_size(init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());
    }

    #[test]
    fn set_size_greater() {
        let init_capacity = 10;
        let new_capacity = 15;

        // empty history state
        let mut state = HistoryState::with_capacity(init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());
        state.set_size(new_capacity);
        assert_eq!(new_capacity, state.entries.capacity());

        // full history state
        let mut state = alloc_history_state(init_capacity, init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());
        state.set_size(new_capacity);
        assert_eq!(new_capacity, state.entries.capacity());
    }

    #[test]
    fn set_size_less() {
        let init_capacity = 10;
        let new_capacity = 5;

        // empty history state
        let mut state = HistoryState::with_capacity(init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());
        state.set_size(new_capacity);
        assert_eq!(new_capacity, state.entries.capacity());

        // full history state
        let mut state = alloc_history_state(init_capacity, init_capacity);
        assert_eq!(init_capacity, state.entries.capacity());
        state.set_size(new_capacity);
        assert_eq!(new_capacity, state.entries.capacity());
    }

    #[test]
    fn expand_empty_command() {
        let cmd = "";
        let state = alloc_history_state(1, 1);
        assert!(expand(cmd).is_none());
    }

    #[test]
    fn expand_empty_history() {
        let state = alloc_history_state(0, 0);
    }

    #[test]
    fn expand_positive_nth_command() {
        let state = alloc_history_state(10, 10);
    }

    #[test]
    fn expand_negative_nth_command() {
        let state = alloc_history_state(10, 10);
    }

    #[test]
    fn expand_search_string() {
        let state = alloc_history_state(10, 10);
    }
}
