use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use jev_curate::client::JevClient;
use jev_curate::filter::CurateFilter;
use jev_curate::parquet_io::DatasetReader;
use jev_curate::presets::PresetConfig;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(
    name = "jev-curate",
    author = "Akash Priyadarshi",
    version = env!("CARGO_PKG_VERSION"),
    about = "High-Throughput Synthetic & Pretraining Dataset Sifter Powered by TypeSafe AI (Jev)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Stream and sift a dataset using TypeSafe AI Jev evaluation rubrics
    Filter {
        /// Path to input .parquet or .jsonl dataset
        input: PathBuf,

        /// Pre-built evaluation preset: reasoning-math, anti-sycophancy, or code-correctness
        #[arg(short, long, default_value = "reasoning-math")]
        preset: String,

        /// Output directory for clean and rejected dataset files
        #[arg(short, long, default_value = "./curated/")]
        out: PathBuf,

        /// Worker concurrency
        #[arg(short, long, default_value_t = 32)]
        concurrency: usize,

        /// Dry-run mode: host pre-filters only, no API calls (honest semantics)
        #[arg(long)]
        dry_run: bool,

        /// Model name (or set JEV_MODEL env). Default jev-latest.
        #[arg(long)]
        model: Option<String>,

        /// Custom API endpoint URL for offline mock testing (or set TYPESAFE_ENDPOINT env var)
        #[arg(long)]
        endpoint: Option<String>,

        /// Output format: jsonl (default) or parquet
        #[arg(long, default_value = "jsonl")]
        format: String,

        /// Resume from existing clean/rejected/errors.jsonl without truncating
        #[arg(long)]
        resume: bool,

        /// Emit clean rows to stdout (for Unix pipe streaming)
        #[arg(long)]
        stdout: bool,

        /// Column name(s) to extract text from (single or comma-separated)
        #[arg(long)]
        column: Option<String>,
    },
}

fn resolve_remote_url(input: &str) -> Option<String> {
    if let Some(rest) = input.strip_prefix("hf://datasets/") {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() == 3 {
            return Some(format!(
                "https://huggingface.co/datasets/{}/{}/resolve/main/{}",
                parts[0], parts[1], parts[2]
            ));
        }
    } else if input.starts_with("https://") {
        return Some(input.to_string());
    }
    None
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Filter {
            input,
            preset,
            out,
            concurrency,
            dry_run,
            model,
            endpoint,
            format,
            resume,
            stdout,
            column,
        } => {
            let effective_endpoint = endpoint.or_else(|| std::env::var("TYPESAFE_ENDPOINT").ok());
            let effective_model = model
                .or_else(|| {
                    std::env::var("JEV_MODEL")
                        .ok()
                        .filter(|s| !s.trim().is_empty())
                })
                .unwrap_or_else(|| "jev-latest".to_string());
            let api_key = match std::env::var("TYPESAFE_API_KEY") {
                Ok(k) if !k.trim().is_empty() => k,
                _ if dry_run || effective_endpoint.is_some() => "dummy".to_string(),
                _ => {
                    anyhow::bail!(
                        "TYPESAFE_API_KEY not set. Refusing non-dry-run without a key. Set TYPESAFE_API_KEY or use --dry-run."
                    );
                }
            };

            let input_str = input.to_string_lossy().to_string();
            let is_stdin = input_str == "-";
            let remote_url = resolve_remote_url(&input_str);
            if !is_stdin && remote_url.is_none() && !input.exists() {
                anyhow::bail!(
                    "Input not found: {}. Check path and try again.",
                    input.display()
                );
            }

            if stdout && format == "parquet" {
                anyhow::bail!("--stdout is only supported with --format jsonl");
            }

            let is_parquet_input = !is_stdin
                && input
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("parquet"));
            if format == "parquet" && !is_parquet_input {
                anyhow::bail!(
                    "--format parquet is only supported for .parquet inputs (got {})",
                    input.display()
                );
            } else if format != "jsonl" && format != "parquet" {
                anyhow::bail!("Unsupported format '{}'. Use 'jsonl' or 'parquet'.", format);
            }

            let columns: Option<Vec<String>> = column.as_deref().map(|c| {
                c.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            });

            let preset_cfg = PresetConfig::load(&preset)?;

            use std::io::IsTerminal;
            let use_color =
                std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
            let version = env!("CARGO_PKG_VERSION");
            if use_color {
                eprintln!("\x1b[1;36m=== jev-curate v{} (beta) ===\x1b[0m", version);
            } else {
                eprintln!("=== jev-curate v{} (beta) ===", version);
            }
            eprintln!("Input:       {}", input.display());
            eprintln!(
                "Preset:      {} ({})",
                preset_cfg.name, preset_cfg.description
            );
            eprintln!("Model:       {}", effective_model);
            eprintln!("Output Dir:  {}", out.display());
            eprintln!("Format:      {}", format);
            eprintln!("Concurrency: {}", concurrency);
            if let Some(ref ep) = effective_endpoint {
                eprintln!("Endpoint:    {}", ep);
            }
            if dry_run {
                eprintln!("Mode:        dry-run (host-only, no Jev calls)");
            }

            fs::create_dir_all(&out)?;

            let mut client = JevClient::new_with_model(api_key, effective_model.clone());
            if let Some(ep) = effective_endpoint {
                client = client.with_endpoint(ep);
            }
            let filter = Arc::new(CurateFilter::new(client, preset_cfg));

            let passed_count = Arc::new(AtomicUsize::new(0));
            let rejected_count = Arc::new(AtomicUsize::new(0));
            let error_count = Arc::new(AtomicUsize::new(0));
            let total_count = Arc::new(AtomicUsize::new(0));
            let tokens_total = Arc::new(AtomicUsize::new(0));
            let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

            let seen: std::collections::HashSet<String> = if resume {
                let mut s = std::collections::HashSet::new();
                for name in ["clean.jsonl", "rejected.jsonl", "errors.jsonl"] {
                    if let Ok(content) = std::fs::read_to_string(out.join(name)) {
                        for line in content.lines() {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                                let raw = v.get("record").and_then(|x| x.as_str()).unwrap_or(line);
                                let hash = format!("{:x}", Sha256::digest(raw.as_bytes()));
                                s.insert(hash);
                            } else {
                                let hash = format!("{:x}", Sha256::digest(line.as_bytes()));
                                s.insert(hash);
                            }
                        }
                    }
                }
                s
            } else {
                Default::default()
            };

            use jev_curate::parquet_io::{DatasetWriter, ParquetWriter};
            let mut writer = DatasetWriter::open(&out, resume)?;
            let mut parquet_writer = if format == "parquet" {
                let schema = DatasetReader::parquet_schema(&input)?;
                Some(ParquetWriter::new(&out, schema, "clean.parquet")?)
            } else {
                None
            };
            let mut audit_file = if resume {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(out.join("audit.jsonl"))?
            } else {
                std::fs::File::create(out.join("audit.jsonl"))?
            };

            let cap = concurrency.max(1) * 2;
            let (record_tx, mut record_rx) =
                mpsc::channel::<(String, String, Option<arrow::record_batch::RecordBatch>)>(cap);
            let (result_tx, mut result_rx) = mpsc::channel::<(
                u8,
                String,
                Vec<String>,
                u64,
                String,
                Option<arrow::record_batch::RecordBatch>,
            )>(cap);

            let input_clone = input.clone();
            let use_parquet_rows = format == "parquet";
            let cols_for_producer = columns.clone();
            let producer = if let Some(url) = remote_url {
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let mut req = client.get(&url);
                    if url.starts_with("https://huggingface.co/")
                        && let Ok(token) = std::env::var("HF_TOKEN")
                        && !token.trim().is_empty()
                    {
                        req = req.bearer_auth(token);
                    }
                    let mut res = req.send().await?.error_for_status()?;
                    let mut buf = String::new();
                    while let Some(chunk) = res.chunk().await? {
                        buf.push_str(&String::from_utf8_lossy(&chunk));
                        while let Some(pos) = buf.find('\n') {
                            let line = buf[..pos].trim_end_matches('\r').to_string();
                            buf.drain(..=pos);
                            if !line.trim().is_empty() {
                                let text = DatasetReader::extract_jsonl_text(
                                    &line,
                                    cols_for_producer.as_deref(),
                                );
                                record_tx
                                    .send((text, line, None))
                                    .await
                                    .map_err(|_| anyhow::anyhow!("record channel closed"))?;
                            }
                        }
                    }
                    if !buf.trim().is_empty() {
                        let line = buf.trim_end_matches('\r').to_string();
                        let text =
                            DatasetReader::extract_jsonl_text(&line, cols_for_producer.as_deref());
                        record_tx
                            .send((text, line, None))
                            .await
                            .map_err(|_| anyhow::anyhow!("record channel closed"))?;
                    }
                    Ok(())
                })
            } else {
                tokio::task::spawn_blocking(move || {
                    if is_stdin {
                        let stdin = std::io::stdin();
                        DatasetReader::stream_jsonl_reader(
                            stdin.lock(),
                            cols_for_producer.as_deref(),
                            |text, raw| {
                                record_tx
                                    .blocking_send((text, raw, None))
                                    .map_err(|_| anyhow::anyhow!("record channel closed"))?;
                                Ok(())
                            },
                        )
                    } else if use_parquet_rows {
                        DatasetReader::stream_parquet_rows_with_cols(
                            &input_clone,
                            cols_for_producer.as_deref(),
                            |text, raw, batch| {
                                record_tx
                                    .blocking_send((text, raw, Some(batch)))
                                    .map_err(|_| anyhow::anyhow!("record channel closed"))?;
                                Ok(())
                            },
                        )
                    } else if is_parquet_input {
                        DatasetReader::stream_parquet_rows_with_cols(
                            &input_clone,
                            cols_for_producer.as_deref(),
                            |text, raw, _batch| {
                                record_tx
                                    .blocking_send((text, raw, None))
                                    .map_err(|_| anyhow::anyhow!("record channel closed"))?;
                                Ok(())
                            },
                        )
                    } else {
                        DatasetReader::stream_jsonl_with_cols(
                            &input_clone,
                            cols_for_producer.as_deref(),
                            |text, raw| {
                                record_tx
                                    .blocking_send((text, raw, None))
                                    .map_err(|_| anyhow::anyhow!("record channel closed"))?;
                                Ok(())
                            },
                        )
                    }
                })
            };

            let model_for_audit = effective_model.clone();
            let writer_handle = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                use std::io::Write;
                let mut stdout_lock = if stdout {
                    Some(std::io::BufWriter::new(std::io::stdout().lock()))
                } else {
                    None
                };
                let mut row_number: u64 = 0;
                while let Some((kind, raw, reasons, tok, hash, batch_opt)) =
                    result_rx.blocking_recv()
                {
                    row_number += 1;
                    match kind {
                        0 => {
                            if let Some(out_w) = stdout_lock.as_mut() {
                                writeln!(out_w, "{}", raw)?;
                            } else if let (Some(pw), Some(batch)) =
                                (parquet_writer.as_mut(), batch_opt.as_ref())
                            {
                                pw.write_batch(batch)?;
                            } else {
                                writer.write_clean_record(&raw)?;
                            }
                        }
                        1 => writer.write_rejected_record(&raw, &reasons)?,
                        _ => writer.write_error_record(&raw, &reasons.join("; "))?,
                    }
                    let audit = serde_json::json!({
                        "row_id": hash,
                        "row_number": row_number,
                        "decision": if kind == 0 { "keep" } else if kind == 1 { "reject" } else { "error" },
                        "rejection_reasons": reasons,
                        "input_tokens": tok,
                        "model": model_for_audit,
                    });
                    writeln!(audit_file, "{}", audit)?;
                }
                if let Some(mut out_w) = stdout_lock {
                    out_w.flush()?;
                }
                writer.flush()?;
                audit_file.flush()?;
                if let Some(mut pw) = parquet_writer {
                    pw.close()?;
                }
                Ok(())
            });

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {pos} rows | Pass: {msg}")
                    .unwrap(),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let mut worker_handles = Vec::new();
            let mut interrupted = false;
            loop {
                let item = tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        interrupted = true;
                        break;
                    }
                    msg = record_rx.recv() => msg,
                };
                let Some((text, raw_line, batch_opt)) = item else {
                    break;
                };
                total_count.fetch_add(1, Ordering::Relaxed);
                let hash = format!("{:x}", Sha256::digest(raw_line.as_bytes()));
                if seen.contains(&hash) {
                    continue;
                }

                let permit = semaphore.clone().acquire_owned().await?;
                let filter_worker = filter.clone();
                let tx_worker = result_tx.clone();
                let passed_cnt = passed_count.clone();
                let rejected_cnt = rejected_count.clone();
                let error_cnt = error_count.clone();
                let tok_cnt = tokens_total.clone();
                let pb_worker = pb.clone();

                let h = tokio::spawn(async move {
                    let _permit = permit;
                    let verdict = if dry_run {
                        if let Some(reason) = CurateFilter::secret_scan(&text) {
                            Ok(jev_curate::filter::CurateVerdict {
                                passed: false,
                                scores: std::collections::HashMap::new(),
                                nouls: std::collections::HashMap::new(),
                                rejection_reasons: vec![format!("Secret scan: {}", reason)],
                                input_tokens: 0,
                            })
                        } else {
                            match filter_worker.pre_filter_sanity(&text) {
                                Ok(_) => Ok(jev_curate::filter::CurateVerdict {
                                    passed: true,
                                    scores: std::collections::HashMap::new(),
                                    nouls: std::collections::HashMap::new(),
                                    rejection_reasons: Vec::new(),
                                    input_tokens: 0,
                                }),
                                Err(rej) => Ok(jev_curate::filter::CurateVerdict {
                                    passed: false,
                                    scores: std::collections::HashMap::new(),
                                    nouls: std::collections::HashMap::new(),
                                    rejection_reasons: vec![format!(
                                        "Host sanity failure: {}",
                                        rej
                                    )],
                                    input_tokens: 0,
                                }),
                            }
                        }
                    } else {
                        filter_worker.evaluate_record(&text).await
                    };

                    match verdict {
                        Ok(v) => {
                            let tok = v.input_tokens as usize;
                            if tok > 0 {
                                tok_cnt.fetch_add(tok, Ordering::Relaxed);
                            }
                            if v.passed {
                                passed_cnt.fetch_add(1, Ordering::Relaxed);
                                let _ = tx_worker
                                    .send((
                                        0,
                                        raw_line,
                                        Vec::new(),
                                        v.input_tokens,
                                        hash,
                                        batch_opt,
                                    ))
                                    .await;
                            } else if v.rejection_reasons.iter().any(|r| {
                                r.contains("Jev evaluation failed") || r.contains("API Error")
                            }) {
                                error_cnt.fetch_add(1, Ordering::Relaxed);
                                let _ = tx_worker
                                    .send((
                                        2,
                                        raw_line,
                                        v.rejection_reasons,
                                        v.input_tokens,
                                        hash,
                                        batch_opt,
                                    ))
                                    .await;
                            } else {
                                rejected_cnt.fetch_add(1, Ordering::Relaxed);
                                let _ = tx_worker
                                    .send((
                                        1,
                                        raw_line,
                                        v.rejection_reasons,
                                        v.input_tokens,
                                        hash,
                                        batch_opt,
                                    ))
                                    .await;
                            }
                        }
                        Err(e) => {
                            error_cnt.fetch_add(1, Ordering::Relaxed);
                            let _ = tx_worker
                                .send((
                                    2,
                                    raw_line,
                                    vec![format!("API Error: {}", e)],
                                    0,
                                    hash,
                                    batch_opt,
                                ))
                                .await;
                        }
                    }

                    let p = passed_cnt.load(Ordering::Relaxed);
                    let r = rejected_cnt.load(Ordering::Relaxed);
                    let e = error_cnt.load(Ordering::Relaxed);
                    let t = p + r + e;
                    let pct = if t > 0 {
                        (p as f64 / t as f64) * 100.0
                    } else {
                        0.0
                    };
                    pb_worker.set_message(format!(
                        "{:.1}% ({} clean, {} rejected, {} errors)",
                        pct, p, r, e
                    ));
                    pb_worker.inc(1);
                });
                worker_handles.push(h);
            }

            drop(record_rx);
            if !interrupted {
                match producer.await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => anyhow::bail!("Failed to read dataset: {}", e),
                    Err(e) => anyhow::bail!("Reader task failed: {}", e),
                }
            }
            for h in worker_handles {
                let _ = h.await;
            }
            drop(result_tx);
            match writer_handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => anyhow::bail!("Writer failed: {}", e),
                Err(e) => anyhow::bail!("Writer task failed: {}", e),
            }
            pb.finish_with_message(if interrupted {
                "Interrupted (partial progress flushed)!"
            } else {
                "Done!"
            });

            let clean_path = out.join("clean.jsonl");
            let rejected_path = out.join("rejected.jsonl");
            let errors_path = out.join("errors.jsonl");

            let p = passed_count.load(Ordering::Relaxed);
            let r = rejected_count.load(Ordering::Relaxed);
            let e = error_count.load(Ordering::Relaxed);
            let total = total_count.load(Ordering::Relaxed);
            let tok = tokens_total.load(Ordering::Relaxed) as f64;

            if total == 0 {
                eprintln!("No records found to filter.");
                if interrupted {
                    std::process::exit(130);
                }
                return Ok(());
            }

            if use_color {
                eprintln!("\n\x1b[1;32m=== Sift Complete ===\x1b[0m");
            } else {
                eprintln!("\n=== Sift Complete ===");
            }
            eprintln!("Clean Output:    {} ({} rows)", clean_path.display(), p);
            eprintln!("Rejected Log:    {} ({} rows)", rejected_path.display(), r);
            eprintln!("Errors Log:      {} ({} rows)", errors_path.display(), e);
            eprintln!("Model:           {}", effective_model);
            eprintln!("Pass Rate:       {:.2}%", (p as f64 / total as f64) * 100.0);
            if tok > 0.0 {
                eprintln!("Input Tokens:  {} (actual from API)", tok as u64);
                eprintln!(
                    "Estimated Cost:  ${:.5} (at $0.042/Mtok)",
                    tok * 0.042 / 1_000_000.0
                );
            } else {
                eprintln!(
                    "Estimated Cost:  ${:.5} (at $0.042/Mtok, 350 tok/row est., dry-run or no usage)",
                    (total as f64 * 350.0 / 1_000_000.0) * 0.042
                );
            }
            let manifest = serde_json::json!({
                "tool_version": env!("CARGO_PKG_VERSION"),
                "model": effective_model,
                "preset": preset,
                "format": format,
                "input": input.display().to_string(),
                "input_rows": total,
                "clean_rows": p,
                "rejected_rows": r,
                "errors": e,
                "input_tokens": tok as u64,
                "estimated_cost_usd": if tok > 0.0 { tok * 0.042 / 1_000_000.0 } else { (total as f64 * 350.0 / 1_000_000.0) * 0.042 },
            });
            let _ = std::fs::write(
                out.join("manifest.json"),
                serde_json::to_string_pretty(&manifest).unwrap(),
            );
            if interrupted {
                std::process::exit(130);
            }
            if e > 0 {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
