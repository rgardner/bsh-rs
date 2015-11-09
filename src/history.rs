use parse::ParseInfo;

#[derive(Debug)]
struct HistoryEntry {
    line: String,
    timestamp: usize,
}

#[derive(Debug)]
pub struct HistoryState {
    entries: Vec<HistoryEntry>,
    /// The entries vector will hold exactly `capacity` elements and will wrap around.
    capacity: usize,
    /// The total number of history items ever saved.
    count: usize,
}

impl HistoryState {
    pub fn new(capacity: usize) -> HistoryState {
        HistoryState {
            entries: Vec::with_capacity(capacity),
            capacity: capacity,
            count: 0
        }
    }

    pub fn push(&mut self, job: &ParseInfo) {
        let idx = self.count % self.capacity;
        let entry = HistoryEntry {
            line: job.command.clone(),
            timestamp: self.count,
        };
        self.entries[idx] = entry;
        self.count += 1;
    }
}
