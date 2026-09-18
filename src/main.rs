use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use jev_curate::client::JevClient;
use jev_curate::filter::CurateFilter;
use jev_curate::parquet_io::DatasetReader;
use jev_curate::presets::PresetConfig;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(
    name = "jev-curate",
    author = "Akash Priyadarshi",
    version = "0.1.0",
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
        } => {
            let api_key = std::env::var("TYPESAFE_API_KEY").unwrap_or_else(|_| {
                eprintln!("\x1b[33mWarning: TYPESAFE_API_KEY not set. Using dummy key for dry-run/mock.\x1b[0m");
                "dummy".to_string()
            });

            let preset_cfg = PresetConfig::from_name(&preset).ok_or_else(|| {
                anyhow::anyhow!("Unknown preset '{}'. Available: reasoning-math, anti-sycophancy, code-correctness", preset)
            })?;

            println!("\x1b[1;36m=== jev-curate v0.1.0 ===\x1b[0m");
            println!("Input:       {}", input.display());
            println!("Preset:      {} ({})", preset_cfg.name, preset_cfg.description);
            println!("Output Dir:  {}", out.display());
            println!("Concurrency: {}", concurrency);

            fs::create_dir_all(&out)?;

            // Read records
            let records = DatasetReader::read_jsonl(&input)?;
            let total = records.len();
            println!("Loaded {} records for evaluation.\n", total);

            if total == 0 {
                println!("No records found to filter.");
                return Ok(());
            }

            let client = JevClient::new(api_key);
            let filter = Arc::new(CurateFilter::new(client, preset_cfg));

            let pb = ProgressBar::new(total as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) | Pass: {msg}")
                    .unwrap()
                    .progress_chars("#>-"),
            );

            let passed_count = Arc::new(AtomicUsize::new(0));
            let rejected_count = Arc::new(AtomicUsize::new(0));

            let (tx, mut rx) = mpsc::channel(100);
            let filter_arc = filter.clone();

            // Spawn worker queue
            let records_iter = records.into_iter();
            let mut tasks = Vec::new();

            for (text, raw_line) in records_iter {
                let filter_worker = filter_arc.clone();
                let tx_worker = tx.clone();
                let passed_cnt = passed_count.clone();
                let rejected_cnt = rejected_count.clone();
                let pb_worker = pb.clone();

                let task = tokio::spawn(async move {
                    let verdict = filter_worker.evaluate_record(&text).await;
                    match verdict {
                        Ok(v) => {
                            if v.passed {
                                passed_cnt.fetch_add(1, Ordering::Relaxed);
                                let _ = tx_worker.send((true, raw_line, Vec::new())).await;
                            } else {
                                rejected_cnt.fetch_add(1, Ordering::Relaxed);
                                let _ = tx_worker.send((false, raw_line, v.rejection_reasons)).await;
                            }
                        }
                        Err(e) => {
                            rejected_cnt.fetch_add(1, Ordering::Relaxed);
                            let _ = tx_worker
                                .send((false, raw_line, vec![format!("API Error: {}", e)]))
                                .await;
                        }
                    }

                    let p = passed_cnt.load(Ordering::Relaxed);
                    let r = rejected_cnt.load(Ordering::Relaxed);
                    let pct = if (p + r) > 0 { (p as f64 / (p + r) as f64) * 100.0 } else { 0.0 };
                    pb_worker.set_message(format!("{:.1}% ({} clean, {} rejected)", pct, p, r));
                    pb_worker.inc(1);
                });
                tasks.push(task);
            }
            drop(tx);

            let mut clean_records = Vec::new();
            let mut rejected_records = Vec::new();

            while let Some((passed, raw, reasons)) = rx.recv().await {
                if passed {
                    clean_records.push(raw);
                } else {
                    rejected_records.push((raw, reasons));
                }
            }

            for t in tasks {
                let _ = t.await;
            }

            pb.finish_with_message("Done!");

            let clean_path = out.join("clean.jsonl");
            let rejected_path = out.join("rejected.jsonl");

            DatasetReader::write_clean_jsonl(&clean_path, &clean_records)?;
            DatasetReader::write_rejected_jsonl(&rejected_path, &rejected_records)?;

            let p = passed_count.load(Ordering::Relaxed);
            let r = rejected_count.load(Ordering::Relaxed);

            println!("\n\x1b[1;32m=== Sift Complete ===\x1b[0m");
            println!("Clean Output:    {} ({} rows)", clean_path.display(), p);
            println!("Rejected Log:    {} ({} rows)", rejected_path.display(), r);
            println!("Pass Rate:       {:.2}%", (p as f64 / total as f64) * 100.0);
            println!("Estimated Cost:  ${:.5} (at $0.042/Mtok)", (total as f64 * 350.0 / 1_000_000.0) * 0.042);
        }
    }

    Ok(())
}
