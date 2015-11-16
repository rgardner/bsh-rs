use parse::ParseJob;
use std::cmp;
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
