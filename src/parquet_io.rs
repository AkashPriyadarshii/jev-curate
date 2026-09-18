use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Stream reader yielding row texts and original lines/JSON strings.
pub struct DatasetReader;

impl DatasetReader {
    /// Reads records from a JSONL file line-by-line without loading into RAM.
    pub fn read_jsonl<P: AsRef<Path>>(path: P) -> Result<Vec<(String, String)>> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open file {}", path.as_ref().display()))?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();
        for line in reader.lines() {
            let line_str = line?;
            let trimmed = line_str.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Extract primary text field if JSON object, else use raw line
            let text = if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                json.get("text")
                    .or_else(|| json.get("content"))
                    .or_else(|| json.get("instruction"))
                    .or_else(|| json.get("prompt"))
                    .or_else(|| json.get("output"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(trimmed)
                    .to_string()
            } else {
                trimmed.to_string()
            };

            records.push((text, line_str));
        }

        Ok(records)
    }

    /// Writes passed records to clean output file.
    pub fn write_clean_jsonl<P: AsRef<Path>>(path: P, records: &[String]) -> Result<()> {
        let mut file = File::create(path)?;
        for rec in records {
            writeln!(file, "{}", rec)?;
        }
        Ok(())
    }

    /// Writes rejected records with reasons to rejection log file.
    pub fn write_rejected_jsonl<P: AsRef<Path>>(
        path: P,
        records: &[(String, Vec<String>)],
    ) -> Result<()> {
        let mut file = File::create(path)?;
        for (raw_line, reasons) in records {
            let obj = serde_json::json!({
                "record": raw_line,
                "rejection_reasons": reasons
            });
            writeln!(file, "{}", obj)?;
        }
        Ok(())
    }
}
