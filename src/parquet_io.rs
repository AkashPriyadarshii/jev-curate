use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Stream reader yielding row texts and original lines/JSON strings.
pub struct DatasetReader;

impl DatasetReader {
    /// Reads records from a JSONL or Parquet file.
    /// Auto-detects format from file extension.
    pub fn read_dataset<P: AsRef<Path>>(path: P) -> Result<Vec<(String, String)>> {
        let p = path.as_ref();
        if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("parquet") {
                return Self::read_parquet(p);
            }
        }
        Self::read_jsonl(p)
    }

    /// Reads records from a Parquet file, extracting text columns.
    pub fn read_parquet<P: AsRef<Path>>(path: P) -> Result<Vec<(String, String)>> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use arrow::array::{Array, AsArray};

        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open parquet file {}", path.as_ref().display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let mut reader = builder.build()?;

        let mut records = Vec::new();

        while let Some(batch_res) = reader.next() {
            let batch = batch_res?;
            let schema = batch.schema();

            // Find primary text column index
            let mut text_col_idx = None;
            for (idx, field) in schema.fields().iter().enumerate() {
                let name = field.name().to_lowercase();
                if name == "text" || name == "content" || name == "instruction" || name == "response" || name == "prompt" {
                    text_col_idx = Some(idx);
                    break;
                }
            }

            let col_idx = text_col_idx.unwrap_or(0);
            if batch.num_columns() == 0 || batch.num_rows() == 0 {
                continue;
            }

            let col = batch.column(col_idx);
            if let Some(arr) = col.as_string_opt::<i32>() {
                for i in 0..arr.len() {
                    if arr.is_valid(i) {
                        let val = arr.value(i).to_string();
                        let raw = serde_json::json!({ "text": &val }).to_string();
                        records.push((val, raw));
                    }
                }
            } else if let Some(arr) = col.as_string_opt::<i64>() {
                for i in 0..arr.len() {
                    if arr.is_valid(i) {
                        let val = arr.value(i).to_string();
                        let raw = serde_json::json!({ "text": &val }).to_string();
                        records.push((val, raw));
                    }
                }
            }
        }

        Ok(records)
    }

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
                if let (Some(inst), Some(resp)) = (
                    json.get("instruction").or_else(|| json.get("prompt")).and_then(|v| v.as_str()),
                    json.get("response").or_else(|| json.get("output")).and_then(|v| v.as_str()),
                ) {
                    format!("Instruction:\n{}\n\nResponse:\n{}", inst, resp)
                } else {
                    json.get("text")
                        .or_else(|| json.get("content"))
                        .or_else(|| json.get("response"))
                        .or_else(|| json.get("output"))
                        .or_else(|| json.get("instruction"))
                        .or_else(|| json.get("prompt"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(trimmed)
                        .to_string()
                }
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

