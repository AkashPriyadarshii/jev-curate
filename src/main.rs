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
    version = "0.2.0",
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
        #[arg(long, default_value = "jev-latest")]
        model: String,

        /// Custom API endpoint URL for offline mock testing (or set TYPESAFE_ENDPOINT env var)
        #[arg(long)]
        endpoint: Option<String>,

        /// Output format: jsonl (default) or parquet
        #[arg(long, default_value = "jsonl")]
        format: String,
    },
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
        } => {
            let effective_endpoint = endpoint.or_else(|| std::env::var("TYPESAFE_ENDPOINT").ok());
            let effective_model = std::env::var("JEV_MODEL").unwrap_or(model);
            let api_key = match std::env::var("TYPESAFE_API_KEY") {
                Ok(k) if !k.trim().is_empty() => k,
                _ if dry_run || effective_endpoint.is_some() => "dummy".to_string(),
                _ => {
                    anyhow::bail!(
                        "TYPESAFE_API_KEY not set. Refusing non-dry-run without a key. Set TYPESAFE_API_KEY or use --dry-run."
                    );
                }
            };

            if !input.exists() {
                anyhow::bail!(
                    "Input not found: {}. Check path and try again.",
                    input.display()
                );
            }

            let preset_cfg = PresetConfig::from_name(&preset).ok_or_else(|| {
                anyhow::anyhow!("Unknown preset '{}'. Available: reasoning-math, anti-sycophancy, code-correctness", preset)
            })?;

            println!("\x1b[1;36m=== jev-curate v0.2.0 (beta) ===\x1b[0m");
            println!("Input:       {}", input.display());
            println!(
                "Preset:      {} ({})",
                preset_cfg.name, preset_cfg.description
            );
            println!("Model:       {}", effective_model);
            println!("Output Dir:  {}", out.display());
            println!("Format:      {}", format);
            println!("Concurrency: {}", concurrency);
            if let Some(ref ep) = effective_endpoint {
                println!("Endpoint:    {}", ep);
            }
            if dry_run {
                println!("Mode:        dry-run (host-only, no Jev calls)");
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

            let (record_tx, mut record_rx) = mpsc::channel::<(String, String)>(200);
            let (result_tx, mut result_rx) = mpsc::channel::<(u8, String, Vec<String>, u64)>(200);

            let input_clone = input.clone();
            let producer = tokio::task::spawn_blocking(move || {
                DatasetReader::stream_dataset(&input_clone, |text, raw| {
                    record_tx
                        .blocking_send((text, raw))
                        .map_err(|_| anyhow::anyhow!("record channel closed"))?;
                    Ok(())
                })
            });

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {pos} rows | Pass: {msg}")
                    .unwrap(),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let mut worker_handles = Vec::new();
            while let Some((text, raw_line)) = record_rx.recv().await {
                total_count.fetch_add(1, Ordering::Relaxed);
                let filter_worker = filter.clone();
                let tx_worker = result_tx.clone();
                let passed_cnt = passed_count.clone();
                let rejected_cnt = rejected_count.clone();
                let error_cnt = error_count.clone();
                let tok_cnt = tokens_total.clone();
                let pb_worker = pb.clone();
                let sem = semaphore.clone();

                let h = tokio::spawn(async move {
                    let _permit = sem.acquire().await.ok();
                    let verdict = if dry_run {
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
                                rejection_reasons: vec![format!("Host sanity failure: {}", rej)],
                                input_tokens: 0,
                            }),
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
                                    .send((0, raw_line, Vec::new(), v.input_tokens))
                                    .await;
                            } else if v.rejection_reasons.iter().any(|r| {
                                r.contains("Jev evaluation failed") || r.contains("Secret scan")
                            }) {
                                // secret scan also not bad data, treat as rejected but keep errors for infra only? next line keeps infra errors separate
                                if v.rejection_reasons.iter().any(|r| {
                                    r.contains("Jev evaluation failed") || r.contains("API Error")
                                }) {
                                    error_cnt.fetch_add(1, Ordering::Relaxed);
                                    let _ = tx_worker
                                        .send((2, raw_line, v.rejection_reasons, v.input_tokens))
                                        .await;
                                } else {
                                    rejected_cnt.fetch_add(1, Ordering::Relaxed);
                                    let _ = tx_worker
                                        .send((1, raw_line, v.rejection_reasons, v.input_tokens))
                                        .await;
                                }
                            } else {
                                rejected_cnt.fetch_add(1, Ordering::Relaxed);
                                let _ = tx_worker
                                    .send((1, raw_line, v.rejection_reasons, v.input_tokens))
                                    .await;
                            }
                        }
                        Err(e) => {
                            error_cnt.fetch_add(1, Ordering::Relaxed);
                            let _ = tx_worker
                                .send((2, raw_line, vec![format!("API Error: {}", e)], 0))
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

            match producer.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => anyhow::bail!("Failed to read dataset: {}", e),
                Err(e) => anyhow::bail!("Reader task failed: {}", e),
            }
            drop(result_tx);

            use jev_curate::parquet_io::DatasetWriter;
            let mut writer = DatasetWriter::new(&out)?;
            let mut audit_file = std::fs::File::create(out.join("audit.jsonl"))?;
            let mut row_number: u64 = 0;
            let seen: std::collections::HashSet<String> = if out.join("manifest.json").exists() {
                // best-effort: read clean/rejected/errors and hash raw lines
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

            while let Some((kind, raw, reasons, tok)) = result_rx.recv().await {
                row_number += 1;
                let hash = format!("{:x}", Sha256::digest(raw.as_bytes()));
                if seen.contains(&hash) {
                    continue;
                } // resume skip
                match kind {
                    0 => writer.write_clean_record(&raw)?,
                    1 => writer.write_rejected_record(&raw, &reasons)?,
                    _ => writer.write_error_record(&raw, &reasons.join("; "))?,
                }
                let audit = serde_json::json!({
                    "row_id": hash,
                    "row_number": row_number,
                    "decision": if kind==0 {"keep"} else if kind==1 {"reject"} else {"error"},
                    "rejection_reasons": reasons,
                    "input_tokens": tok,
                    "model": effective_model,
                });
                use std::io::Write;
                writeln!(audit_file, "{}", audit)?;
            }

            for h in worker_handles {
                let _ = h.await;
            }

            writer.flush()?;
            pb.finish_with_message("Done!");

            let clean_path = out.join("clean.jsonl");
            let rejected_path = out.join("rejected.jsonl");
            let errors_path = out.join("errors.jsonl");

            let p = passed_count.load(Ordering::Relaxed);
            let r = rejected_count.load(Ordering::Relaxed);
            let e = error_count.load(Ordering::Relaxed);
            let total = total_count.load(Ordering::Relaxed);
            let tok = tokens_total.load(Ordering::Relaxed) as f64;

            if total == 0 {
                println!("No records found to filter.");
                return Ok(());
            }

            println!("\n\x1b[1;32m=== Sift Complete ===\x1b[0m");
            println!("Clean Output:    {} ({} rows)", clean_path.display(), p);
            println!("Rejected Log:    {} ({} rows)", rejected_path.display(), r);
            println!("Errors Log:      {} ({} rows)", errors_path.display(), e);
            println!("Model:           {}", effective_model);
            println!("Pass Rate:       {:.2}%", (p as f64 / total as f64) * 100.0);
            if tok > 0.0 {
                println!("Input Tokens:  {} (actual from API)", tok as u64);
                println!(
                    "Estimated Cost:  ${:.5} (at $0.042/Mtok)",
                    tok * 0.042 / 1_000_000.0
                );
            } else {
                println!(
                    "Estimated Cost:  ${:.5} (at $0.042/Mtok, 350 tok/row est., dry-run or no usage)",
                    (total as f64 * 350.0 / 1_000_000.0) * 0.042
                );
            }
            let manifest = serde_json::json!({
                "tool_version": "0.2.0",
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
        }
    }

    Ok(())
}
