//! Following a growing Client Log by byte offset.
//!
//! Polling-based on purpose: the game keeps the file open and appends to it,
//! and polling a size every few hundred ms is cheaper and more reliable than
//! filesystem notifications for this.

use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::log::{LogRecord, parse_line};

#[derive(Debug)]
pub struct LogTail {
    path: PathBuf,
    /// Byte offset just past the last complete line consumed.
    offset: u64,
    /// Bytes of an incomplete trailing line, kept until its newline arrives.
    partial: Vec<u8>,
}

impl LogTail {
    /// Follow `path` starting from `offset` (use the file's length to skip
    /// history, or a saved offset to catch up on what was missed).
    pub fn new(path: impl Into<PathBuf>, offset: u64) -> Self {
        Self {
            path: path.into(),
            offset,
            partial: Vec::new(),
        }
    }

    /// Follow `path` from its current end.
    pub fn from_end(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let len = std::fs::metadata(&path)?.len();
        Ok(Self::new(path, len))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Offset to persist so a later session can resume exactly here.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Read everything appended since the last poll.
    pub fn poll(&mut self) -> io::Result<Vec<LogRecord>> {
        let mut file = File::open(&self.path)?;
        let len = file.metadata()?.len();
        if len < self.offset {
            // Truncated or replaced (e.g. the player deleted the log).
            self.offset = 0;
            self.partial.clear();
        }
        if len == self.offset {
            return Ok(Vec::new());
        }
        file.seek(SeekFrom::Start(self.offset + self.partial.len() as u64))?;
        let mut buf = std::mem::take(&mut self.partial);
        file.take(len - self.offset - buf.len() as u64)
            .read_to_end(&mut buf)?;

        let complete = buf.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        self.offset += complete as u64;
        self.partial = buf.split_off(complete);

        Ok(buf
            .split(|&b| b == b'\n')
            .filter_map(|line| parse_line(std::str::from_utf8(line).ok()?))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::log::LogEvent;

    const ENTER: &str = "2026/11/18 01:00:00 1 x [INFO Client 1] : You have entered The Coast.\n";

    fn temp_log(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("poe-core-tail-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, "").unwrap();
        path
    }

    fn append(path: &Path, text: &str) {
        let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    #[test]
    fn reads_only_new_complete_lines() {
        let path = temp_log("new_lines.txt");
        append(&path, ENTER);
        let mut tail = LogTail::from_end(&path).unwrap();
        assert!(tail.poll().unwrap().is_empty());

        let (head, rest) = ENTER.split_at(30);
        append(&path, head);
        assert!(tail.poll().unwrap().is_empty(), "partial line held back");
        append(&path, rest);
        let records = tail.poll().unwrap();
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].event, LogEvent::AreaEntered { .. }));
        assert_eq!(tail.offset(), (ENTER.len() * 2) as u64);
    }

    #[test]
    fn resumes_from_saved_offset() {
        let path = temp_log("resume.txt");
        append(&path, ENTER);
        append(&path, ENTER);
        let mut tail = LogTail::new(&path, ENTER.len() as u64);
        assert_eq!(tail.poll().unwrap().len(), 1);
    }

    #[test]
    fn restarts_after_truncation() {
        let path = temp_log("truncate.txt");
        append(&path, ENTER);
        append(&path, ENTER);
        let mut tail = LogTail::from_end(&path).unwrap();
        std::fs::write(&path, ENTER).unwrap();
        assert_eq!(tail.poll().unwrap().len(), 1);
    }
}
