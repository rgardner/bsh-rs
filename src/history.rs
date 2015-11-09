use parse::ParseInfo;
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

    pub fn push(&mut self, job: &ParseInfo) {
        let idx = self.count % self.entries.capacity();
        let entry = HistoryEntry {
            line: job.command.clone(),
            timestamp: self.count,
        };
        match self.entries.get(idx) {
            Some(_) => self.entries[idx] = entry,
            None => self.entries.insert(idx, entry),
        }
        self.count += 1;
    }
}

impl fmt::Display for HistoryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let entries = self.entries
                          .iter()
                          .map(|e| format!("\t{}\t{}", e.timestamp.clone(), e.line.clone()))
                          .collect::<Vec<String>>()
                          .join("\n");
        write!(f, "{}", entries)
    }
}
