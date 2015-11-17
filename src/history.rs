use parse::ParseJob;
use std::cmp::{self, Ordering};
use std::fmt;

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

    pub fn push(&mut self, job: &ParseJob) {
        let idx = self.count % self.entries.capacity();
        let entry = HistoryEntry {
            line: job.command.clone(),
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
                self.clear();
                self.entries.truncate(size);
                self.entries.shrink_to_fit();
            }
            Ordering::Greater => self.entries.reserve_exact(size - capacity),
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
}

impl fmt::Display for HistoryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display(self.count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parse::ParseJob;

    #[test]
    fn init_with_capacity() {
        let init_size = 10;
        let state = HistoryState::with_capacity(init_size);
        assert_eq!(init_size, state.entries.capacity());
        assert!(state.entries.is_empty());
        assert_eq!(0, state.count);
    }

    #[test]
    fn clear() {
        let init_size = 10;
        let mut state = HistoryState::with_capacity(init_size);
        let jobs = "cmd1 cmd2 cmd3".split_whitespace().map(|c| ParseJob::parse(c).unwrap().unwrap());
        for j in jobs {
            state.push(&j);
        }
        state.clear();
        assert!(state.entries.is_empty());
        assert_eq!(0, state.count);
    }

    #[test]
    fn set_size_equal() {
        let init_size = 10;
        let mut state = HistoryState::with_capacity(init_size);
        assert_eq!(init_size, state.entries.capacity());
        state.set_size(init_size);
        assert_eq!(init_size, state.entries.capacity());
    }

    #[test]
    #[ignore]
    fn set_size_greater() {
        let init_size = 10;
        let new_size = 15;
        let mut state = HistoryState::with_capacity(init_size);
        assert_eq!(init_size, state.entries.capacity());
        state.set_size(new_size);
        assert_eq!(new_size, state.entries.capacity());
    }

    #[test]
    #[ignore]
    fn set_size_less() {
        let init_size = 10;
        let new_size = 5;
        let mut state = HistoryState::with_capacity(init_size);
        assert_eq!(init_size, state.entries.capacity());
        state.set_size(new_size);
        assert_eq!(new_size, state.entries.capacity());
    }
}
