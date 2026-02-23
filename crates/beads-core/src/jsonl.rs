//! JSONL (JSON Lines) read/write support.
//!
//! Each line in a JSONL file is a complete JSON object representing one Issue.

use std::io::{self, BufRead, Write};

use crate::issue::Issue;

/// Error type for JSONL operations.
#[derive(Debug, thiserror::Error)]
pub enum JsonlError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error at line {line}: {source}")]
    Json {
        line: usize,
        source: serde_json::Error,
    },
}

/// Result alias for JSONL operations.
pub type Result<T> = std::result::Result<T, JsonlError>;

/// Writes issues as JSONL to the given writer.
///
/// Each issue is written as a single JSON line followed by a newline.
pub fn write_jsonl<W: Write>(writer: &mut W, issues: &[Issue]) -> Result<()> {
    for issue in issues {
        serde_json::to_writer(&mut *writer, issue)
            .map_err(|e| JsonlError::Json { line: 0, source: e })?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

/// Returns an iterator that reads issues from a JSONL reader.
///
/// Each line is parsed as a JSON object. Empty lines are skipped.
pub fn read_jsonl<R: BufRead>(reader: R) -> JsonlIter<R> {
    JsonlIter {
        reader,
        line_number: 0,
    }
}

/// Iterator over JSONL-encoded issues.
pub struct JsonlIter<R> {
    reader: R,
    line_number: usize,
}

impl<R: BufRead> Iterator for JsonlIter<R> {
    type Item = Result<Issue>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        loop {
            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    self.line_number += 1;
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue; // skip empty lines
                    }
                    match serde_json::from_str::<Issue>(trimmed) {
                        Ok(issue) => return Some(Ok(issue)),
                        Err(e) => {
                            return Some(Err(JsonlError::Json {
                                line: self.line_number,
                                source: e,
                            }));
                        }
                    }
                }
                Err(e) => return Some(Err(JsonlError::Io(e))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::IssueBuilder;
    use std::io::BufReader;

    #[test]
    fn write_and_read_roundtrip() {
        let issues = vec![
            IssueBuilder::new("Issue 1")
                .id("bd-001")
                .priority(1)
                .build(),
            IssueBuilder::new("Issue 2")
                .id("bd-002")
                .priority(2)
                .build(),
        ];

        let mut buf = Vec::new();
        write_jsonl(&mut buf, &issues).unwrap();

        let reader = BufReader::new(buf.as_slice());
        let read_back: Vec<Issue> = read_jsonl(reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].title, "Issue 1");
        assert_eq!(read_back[1].title, "Issue 2");
    }

    #[test]
    fn read_skips_empty_lines() {
        let data = b"{\"title\":\"A\"}\n\n{\"title\":\"B\"}\n";
        let reader = BufReader::new(data.as_slice());
        let results: Vec<Issue> = read_jsonl(reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn read_reports_line_number_on_error() {
        let data = b"{\"title\":\"A\"}\nnot-json\n";
        let reader = BufReader::new(data.as_slice());
        let results: Vec<_> = read_jsonl(reader).collect();
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        match &results[1] {
            Err(JsonlError::Json { line, .. }) => assert_eq!(*line, 2),
            other => panic!("expected JSON error, got {:?}", other),
        }
    }
}
