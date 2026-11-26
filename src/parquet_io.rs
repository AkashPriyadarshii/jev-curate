use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Reader yielding row texts and original lines/JSON strings.
///
/// Batch-oriented Parquet decoding + line-buffered JSONL. For large datasets
/// prefer the `stream_*` methods which avoid materializing the full `Vec`.
pub struct DatasetReader;

impl DatasetReader {
    /// Reads records from a JSONL or Parquet file.
    /// Auto-detects format from file extension.
    pub fn read_dataset<P: AsRef<Path>>(path: P) -> Result<Vec<(String, String)>> {
        let p = path.as_ref();
        if let Some(ext) = p.extension().and_then(|s| s.to_str())
            && ext.eq_ignore_ascii_case("parquet")
        {
            return Self::read_parquet(p);
        }
        Self::read_jsonl(p)
    }

    /// Streams records via callback without materializing the full `Vec`.
    /// Auto-detects format from file extension. Bounded memory: only
    /// current batch/line is held plus worker queue.
    pub fn stream_dataset<P: AsRef<Path>, F>(path: P, f: F) -> Result<()>
    where
        F: FnMut(String, String) -> Result<()>,
    {
        let p = path.as_ref();
        if let Some(ext) = p.extension().and_then(|s| s.to_str())
            && ext.eq_ignore_ascii_case("parquet")
        {
            return Self::stream_parquet(p, f);
        }
        Self::stream_jsonl(p, f)
    }

    /// Returns the Arrow schema of a Parquet file.
    pub fn parquet_schema<P: AsRef<Path>>(path: P) -> Result<arrow::datatypes::SchemaRef> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open parquet file {}", path.as_ref().display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        Ok(builder.schema().clone())
    }

    /// Streams Parquet records via callback, also yielding the 1-row `RecordBatch` slice for schema-preserving output.
    pub fn stream_parquet_rows<P: AsRef<Path>, F>(path: P, on_row: F) -> Result<()>
    where
        F: FnMut(String, String, arrow::record_batch::RecordBatch) -> Result<()>,
    {
        Self::stream_parquet_rows_with_cols(path, None, on_row)
    }

    /// Streams Parquet records with optional column selector, yielding the 1-row `RecordBatch` slice.
    pub fn stream_parquet_rows_with_cols<P: AsRef<Path>, F>(
        path: P,
        cols: Option<&[String]>,
        mut on_row: F,
    ) -> Result<()>
    where
        F: FnMut(String, String, arrow::record_batch::RecordBatch) -> Result<()>,
    {
        use arrow::array::{Array, AsArray};
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open parquet file {}", path.as_ref().display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;
        let cols: Option<&[String]> = None;
        for batch_res in reader {
            let batch = batch_res?;
            let schema = batch.schema();
            let mut text_col_idx = None;
            if let Some(wanted) = cols
                && !wanted.is_empty()
            {
                for w in wanted {
                    if let Some((idx, _)) = schema
                        .fields()
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.name().eq_ignore_ascii_case(w))
                    {
                        text_col_idx = Some(idx);
                        break;
                    }
                }
            }
            if text_col_idx.is_none() {
                for (idx, field) in schema.fields().iter().enumerate() {
                    let name = field.name().to_lowercase();
                    if name == "text"
                        || name == "content"
                        || name == "instruction"
                        || name == "response"
                        || name == "prompt"
                    {
                        text_col_idx = Some(idx);
                        break;
                    }
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
                        on_row(val, raw, batch.slice(i, 1))?;
                    }
                }
            } else if let Some(arr) = col.as_string_opt::<i64>() {
                for i in 0..arr.len() {
                    if arr.is_valid(i) {
                        let val = arr.value(i).to_string();
                        let raw = serde_json::json!({ "text": &val }).to_string();
                        on_row(val, raw, batch.slice(i, 1))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Streams Parquet records via callback (batch-decoded, not Vec).
    pub fn stream_parquet<P: AsRef<Path>, F>(path: P, mut on_record: F) -> Result<()>
    where
        F: FnMut(String, String) -> Result<()>,
    {
        use arrow::array::{Array, AsArray};
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open parquet file {}", path.as_ref().display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;
        let cols: Option<&[String]> = None;
        for batch_res in reader {
            let batch = batch_res?;
            let schema = batch.schema();
            let mut text_col_idx = None;
            if let Some(wanted) = cols
                && !wanted.is_empty()
            {
                for w in wanted {
                    if let Some((idx, _)) = schema
                        .fields()
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.name().eq_ignore_ascii_case(w))
                    {
                        text_col_idx = Some(idx);
                        break;
                    }
                }
            }
            if text_col_idx.is_none() {
                for (idx, field) in schema.fields().iter().enumerate() {
                    let name = field.name().to_lowercase();
                    if name == "text"
                        || name == "content"
                        || name == "instruction"
                        || name == "response"
                        || name == "prompt"
                    {
                        text_col_idx = Some(idx);
                        break;
                    }
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
                        on_record(val, raw)?;
                    }
                }
            } else if let Some(arr) = col.as_string_opt::<i64>() {
                for i in 0..arr.len() {
                    if arr.is_valid(i) {
                        let val = arr.value(i).to_string();
                        let raw = serde_json::json!({ "text": &val }).to_string();
                        on_record(val, raw)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Streams JSONL records via callback (line-buffered, not Vec).
    pub fn stream_jsonl<P: AsRef<Path>, F>(path: P, on_record: F) -> Result<()>
    where
        F: FnMut(String, String) -> Result<()>,
    {
        Self::stream_jsonl_with_cols(path, None, on_record)
    }

    /// Streams JSONL records via callback with optional column selector.
    pub fn stream_jsonl_with_cols<P: AsRef<Path>, F>(
        path: P,
        cols: Option<&[String]>,
        on_record: F,
    ) -> Result<()>
    where
        F: FnMut(String, String) -> Result<()>,
    {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open file {}", path.as_ref().display()))?;
        Self::stream_jsonl_reader(BufReader::new(file), cols, on_record)
    }

    /// Extracts text from a single JSONL line using optional column names or default heuristics.
    pub fn extract_jsonl_text(line: &str, cols: Option<&[String]>) -> String {
        let trimmed = line.trim();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(wanted) = cols
                && !wanted.is_empty()
            {
                let parts: Vec<String> = wanted
                    .iter()
                    .filter_map(|c| {
                        json.get(c).and_then(|v| v.as_str()).map(|s| {
                            if wanted.len() > 1 {
                                format!("{}: {}", c, s)
                            } else {
                                s.to_string()
                            }
                        })
                    })
                    .collect();
                if !parts.is_empty() {
                    return parts.join("\n\n");
                }
            }
            if let (Some(inst), Some(resp)) = (
                json.get("instruction")
                    .or_else(|| json.get("prompt"))
                    .and_then(|v| v.as_str()),
                json.get("response")
                    .or_else(|| json.get("output"))
                    .and_then(|v| v.as_str()),
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
        }
    }

    /// Streams JSONL records from any `BufRead` (file, stdin, etc.).
    pub fn stream_jsonl_reader<R: BufRead, F>(
        reader: R,
        cols: Option<&[String]>,
        mut on_record: F,
    ) -> Result<()>
    where
        F: FnMut(String, String) -> Result<()>,
    {
        for line in reader.lines() {
            let line_str = line?;
            let trimmed = line_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            let text = Self::extract_jsonl_text(&line_str, cols);
            on_record(text, line_str)?;
        }
        Ok(())
    }

    /// Reads records from a Parquet file, extracting text columns.
    pub fn read_parquet<P: AsRef<Path>>(path: P) -> Result<Vec<(String, String)>> {
        use arrow::array::{Array, AsArray};
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open parquet file {}", path.as_ref().display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut records = Vec::new();
        let cols: Option<&[String]> = None;

        for batch_res in reader {
            let batch = batch_res?;
            let schema = batch.schema();

            // Find primary text column index
            let mut text_col_idx = None;
            if let Some(wanted) = cols
                && !wanted.is_empty()
            {
                for w in wanted {
                    if let Some((idx, _)) = schema
                        .fields()
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.name().eq_ignore_ascii_case(w))
                    {
                        text_col_idx = Some(idx);
                        break;
                    }
                }
            }
            if text_col_idx.is_none() {
                for (idx, field) in schema.fields().iter().enumerate() {
                    let name = field.name().to_lowercase();
                    if name == "text"
                        || name == "content"
                        || name == "instruction"
                        || name == "response"
                        || name == "prompt"
                    {
                        text_col_idx = Some(idx);
                        break;
                    }
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
                    json.get("instruction")
                        .or_else(|| json.get("prompt"))
                        .and_then(|v| v.as_str()),
                    json.get("response")
                        .or_else(|| json.get("output"))
                        .and_then(|v| v.as_str()),
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

/// Parquet writer: schema-preserving when input is parquet + --format parquet.
pub struct ParquetWriter {
    writer: Option<parquet::arrow::arrow_writer::ArrowWriter<File>>,
    #[allow(dead_code)]
    schema: arrow::datatypes::SchemaRef,
    #[allow(dead_code)]
    path: std::path::PathBuf,
}

impl ParquetWriter {
    pub fn new<P: AsRef<Path>>(
        out_dir: P,
        schema: arrow::datatypes::SchemaRef,
        name: &str,
    ) -> Result<Self> {
        let path = out_dir.as_ref().join(name);
        let file =
            File::create(&path).with_context(|| format!("Failed to create {}", path.display()))?;
        let writer =
            parquet::arrow::arrow_writer::ArrowWriter::try_new(file, schema.clone(), None)?;
        Ok(Self {
            writer: Some(writer),
            schema,
            path,
        })
    }
    pub fn write_batch(&mut self, batch: &arrow::record_batch::RecordBatch) -> Result<()> {
        if let Some(w) = self.writer.as_mut() {
            w.write(batch)?;
        }
        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        if let Some(w) = self.writer.take() {
            w.close()?;
        }
        Ok(())
    }
}

/// Streaming dataset writer that writes directly to disk with buffered I/O, avoiding in-memory Vec accumulation.
pub struct DatasetWriter {
    clean: std::io::BufWriter<File>,
    rejected: std::io::BufWriter<File>,
    errors: std::io::BufWriter<File>,
}

impl DatasetWriter {
    pub fn new<P: AsRef<Path>>(out_dir: P) -> Result<Self> {
        Self::open(out_dir, false)
    }

    pub fn open<P: AsRef<Path>>(out_dir: P, append: bool) -> Result<Self> {
        let clean_path = out_dir.as_ref().join("clean.jsonl");
        let rejected_path = out_dir.as_ref().join("rejected.jsonl");
        let errors_path = out_dir.as_ref().join("errors.jsonl");

        let open_file = |path: &Path| -> Result<File> {
            if append {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .with_context(|| format!("Failed to open {} for append", path.display()))
            } else {
                File::create(path).with_context(|| format!("Failed to create {}", path.display()))
            }
        };

        let clean_file = open_file(&clean_path)?;
        let rejected_file = open_file(&rejected_path)?;
        let errors_file = open_file(&errors_path)?;

        Ok(Self {
            clean: std::io::BufWriter::new(clean_file),
            rejected: std::io::BufWriter::new(rejected_file),
            errors: std::io::BufWriter::new(errors_file),
        })
    }

    pub fn write_clean_record(&mut self, record: &str) -> Result<()> {
        writeln!(self.clean, "{}", record)?;
        Ok(())
    }

    pub fn write_rejected_record(&mut self, record: &str, reasons: &[String]) -> Result<()> {
        let obj = serde_json::json!({
            "record": record,
            "rejection_reasons": reasons
        });
        writeln!(self.rejected, "{}", obj)?;
        Ok(())
    }

    pub fn write_error_record(&mut self, record: &str, error: &str) -> Result<()> {
        let obj = serde_json::json!({
            "record": record,
            "error": error
        });
        writeln!(self.errors, "{}", obj)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.clean.flush()?;
        self.rejected.flush()?;
        self.errors.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_writer_streams_clean_and_rejected_to_disk() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut writer = DatasetWriter::new(temp_dir.path()).unwrap();

        writer
            .write_clean_record("{\"text\": \"valid clean record\"}")
            .unwrap();
        writer
            .write_rejected_record(
                "{\"text\": \"corrupt record\"}",
                &["Reason 1".to_string(), "Reason 2".to_string()],
            )
            .unwrap();
        writer.flush().unwrap();

        let clean_records = DatasetReader::read_jsonl(temp_dir.path().join("clean.jsonl")).unwrap();
        assert_eq!(clean_records.len(), 1);
        assert_eq!(clean_records[0].0, "valid clean record");

        let rej_content = std::fs::read_to_string(temp_dir.path().join("rejected.jsonl")).unwrap();
        assert!(rej_content.contains("Reason 1"));
        assert!(rej_content.contains("Reason 2"));
    }
}
