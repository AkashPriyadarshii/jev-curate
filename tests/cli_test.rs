use arrow::array::{ArrayRef, Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::sync::Arc;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jev-curate"))
}

#[test]
fn test_cli_dry_run_streaming_over_300_rows_and_secret_scan() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.jsonl");
    let out = dir.path().join("out");

    {
        let mut f = File::create(&input).unwrap();
        for i in 0..310 {
            writeln!(
                f,
                "{{\"text\": \"Valid reasoning step {} with task-1234567890123456 and disk-cache and flask-app\"}}",
                i
            )
            .unwrap();
        }
        writeln!(
            f,
            "{{\"text\": \"Leaked key sk-proj-abcdefghijklmnopqrstuvwxyz123456 should be rejected\"}}"
        )
        .unwrap();
    }

    let output = bin()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--dry-run",
            "--concurrency",
            "8",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let clean = std::fs::read_to_string(out.join("clean.jsonl")).unwrap();
    let rejected = std::fs::read_to_string(out.join("rejected.jsonl")).unwrap();
    let audit = std::fs::read_to_string(out.join("audit.jsonl")).unwrap();

    assert_eq!(clean.lines().count(), 310);
    assert_eq!(rejected.lines().count(), 1);
    assert!(rejected.contains("Secret scan"));
    assert_eq!(audit.lines().count(), 311);
}

#[test]
fn test_cli_resume_non_destructive() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.jsonl");
    let out = dir.path().join("out");

    {
        let mut f = File::create(&input).unwrap();
        for i in 0..3 {
            writeln!(f, "{{\"text\": \"Valid resume record number {}\"}}", i).unwrap();
        }
    }

    let status1 = bin()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--dry-run",
            "--out",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status1.success());
    assert_eq!(
        std::fs::read_to_string(out.join("clean.jsonl"))
            .unwrap()
            .lines()
            .count(),
        3
    );

    {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&input)
            .unwrap();
        for i in 3..5 {
            writeln!(f, "{{\"text\": \"Valid resume record number {}\"}}", i).unwrap();
        }
    }

    let status2 = bin()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--dry-run",
            "--resume",
            "--out",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status2.success());

    let clean = std::fs::read_to_string(out.join("clean.jsonl")).unwrap();
    assert_eq!(clean.lines().count(), 5);
}

#[test]
fn test_cli_format_parquet_preserves_schema_and_rejects_jsonl() {
    let dir = tempfile::tempdir().unwrap();
    let pq_in = dir.path().join("input.parquet");
    let out = dir.path().join("out");

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("text", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(vec![1, 2])) as ArrayRef,
            Arc::new(StringArray::from(vec![
                "Valid parquet record with enough length",
                "bad",
            ])) as ArrayRef,
        ],
    )
    .unwrap();

    {
        let file = File::create(&pq_in).unwrap();
        let mut writer = ArrowWriter::try_new(file, schema.clone(), None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
    }

    let output = bin()
        .args([
            "filter",
            pq_in.to_str().unwrap(),
            "--dry-run",
            "--format",
            "parquet",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let pq_out = out.join("clean.parquet");
    assert!(pq_out.exists());
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(&pq_out).unwrap())
        .unwrap()
        .build()
        .unwrap();
    let mut total_rows = 0;
    for b in reader {
        let b = b.unwrap();
        assert_eq!(b.schema().fields().len(), 2);
        assert_eq!(b.schema().field(0).name(), "id");
        assert_eq!(b.schema().field(1).name(), "text");
        total_rows += b.num_rows();
    }
    assert_eq!(total_rows, 1);

    // JSONL input with --format parquet must return a clear error
    let jsonl_in = dir.path().join("input.jsonl");
    std::fs::write(&jsonl_in, "{\"text\": \"Valid jsonl record here\"}\n").unwrap();
    let err_out = bin()
        .args([
            "filter",
            jsonl_in.to_str().unwrap(),
            "--dry-run",
            "--format",
            "parquet",
            "--out",
            dir.path().join("out2").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!err_out.status.success());
    let stderr = String::from_utf8_lossy(&err_out.stderr);
    assert!(stderr.contains("--format parquet"));
}

#[test]
fn test_cli_custom_yaml_preset_and_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.jsonl");
    let valid_yaml = dir.path().join("rubric.yaml");
    let invalid_yaml = dir.path().join("bad.yaml");
    let out = dir.path().join("out");

    std::fs::write(
        &input,
        "{\"text\": \"Valid record for custom yaml preset\"}\n",
    )
    .unwrap();
    std::fs::write(
        &valid_yaml,
        "name: custom-rubric\ndescription: Custom test rubric\nquestions: {}\nmin_scores: {}\nreject_nouls: {}\nmin_confidence: 0.5\n",
    )
    .unwrap();
    std::fs::write(&invalid_yaml, "name: [unclosed\n").unwrap();

    let ok_out = bin()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--dry-run",
            "--preset",
            valid_yaml.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        ok_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ok_out.stderr)
    );

    let bad_out = bin()
        .args([
            "filter",
            input.to_str().unwrap(),
            "--dry-run",
            "--preset",
            invalid_yaml.to_str().unwrap(),
            "--out",
            dir.path().join("out2").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!bad_out.status.success());
    let stderr = String::from_utf8_lossy(&bad_out.stderr);
    assert!(stderr.contains("invalid rubric yaml"));
}

#[test]
fn test_cli_stdin_stdout_pipe_and_column_selector() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");

    let mut child = bin()
        .args([
            "filter",
            "-",
            "--dry-run",
            "--stdout",
            "--column",
            "body",
            "--out",
            out.to_str().unwrap(),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(
            stdin,
            "{{\"body\": \"Valid stdin pipe record with enough characters\", \"other\": \"x\"}}"
        )
        .unwrap();
        writeln!(
            stdin,
            "{{\"body\": \"short\", \"other\": \"long enough ignored\"}}"
        )
        .unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout_str.lines().count(), 1);
    assert!(stdout_str.contains("Valid stdin pipe record"));
}
