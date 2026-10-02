pub mod client;
pub mod filter;
pub mod parquet_io;
pub mod presets;
pub mod rate_limiter;

pub use client::JevClient;
pub use filter::{CurateFilter, CurateVerdict};
pub use presets::PresetConfig;
pub use rate_limiter::RateLimiter;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pyclass]
pub struct PyJevCurator {
    filter: filter::CurateFilter,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyJevCurator {
    #[new]
    #[pyo3(signature = (api_key, preset, model=None, endpoint=None))]
    pub fn new(
        api_key: String,
        preset: String,
        model: Option<String>,
        endpoint: Option<String>,
    ) -> PyResult<Self> {
        let preset_cfg = presets::PresetConfig::from_name(&preset).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("Unknown preset: {}", preset))
        })?;
        let mut client = if let Some(m) = model {
            client::JevClient::new_with_model(api_key, m)
        } else {
            client::JevClient::new(api_key)
        };
        if let Some(ep) = endpoint {
            client = client.with_endpoint(ep);
        }
        Ok(Self {
            filter: filter::CurateFilter::new(client, preset_cfg),
        })
    }

    fn filter_text(&self, py: Python<'_>, text: String) -> PyResult<PyObject> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let verdict = rt
            .block_on(self.filter.evaluate_record(&text))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let d = pyo3::types::PyDict::new(py);
        d.set_item("passed", verdict.passed)?;
        d.set_item("rejection_reasons", verdict.rejection_reasons)?;
        d.set_item("scores", verdict.scores)?;
        d.set_item("nouls", verdict.nouls)?;
        Ok(d.into())
    }

    #[pyo3(signature = (input, out="./curated", dry_run=false))]
    fn filter_file(
        &self,
        py: Python<'_>,
        input: String,
        out: String,
        dry_run: bool,
    ) -> PyResult<PyObject> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        // reuse main logic via streaming + filter loop (sync wrapper)
        let input_p = std::path::PathBuf::from(input);
        let out_p = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out_p)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let (passed, rejected, errors, total) = rt
            .block_on(async {
                let mut passed = 0usize;
                let mut rejected = 0usize;
                let mut errors = 0usize;
                let mut total = 0usize;
                let mut writer = parquet_io::DatasetWriter::new(&out_p)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                let mut records: Vec<(String, String)> = Vec::new();
                parquet_io::DatasetReader::stream_dataset(&input_p, |t, r| {
                    records.push((t, r));
                    Ok(())
                })
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                for (text, raw) in records {
                    total += 1;
                    let verdict = if dry_run {
                        match self.filter.pre_filter_sanity(&text) {
                            Ok(_) => filter::CurateVerdict {
                                passed: true,
                                scores: Default::default(),
                                nouls: Default::default(),
                                rejection_reasons: Vec::new(),
                                input_tokens: 0,
                            },
                            Err(rej) => filter::CurateVerdict {
                                passed: false,
                                scores: Default::default(),
                                nouls: Default::default(),
                                rejection_reasons: vec![format!("Host sanity failure: {}", rej)],
                                input_tokens: 0,
                            },
                        }
                    } else {
                        self.filter
                            .evaluate_record(&text)
                            .await
                            .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    };
                    if verdict.passed {
                        passed += 1;
                        writer
                            .write_clean_record(&raw)
                            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    } else if verdict
                        .rejection_reasons
                        .iter()
                        .any(|r| r.contains("Jev evaluation failed"))
                    {
                        errors += 1;
                        writer
                            .write_error_record(&raw, &verdict.rejection_reasons.join("; "))
                            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    } else {
                        rejected += 1;
                        writer
                            .write_rejected_record(&raw, &verdict.rejection_reasons)
                            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    }
                }
                writer.flush().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                Ok::<_, anyhow::Error>((passed, rejected, errors, total))
            })
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let d = pyo3::types::PyDict::new(py);
        d.set_item("passed", passed)?;
        d.set_item("rejected", rejected)?;
        d.set_item("errors", errors)?;
        d.set_item("total", total)?;
        Ok(d.into())
    }
}

#[cfg(feature = "python")]
#[pymodule]
fn jev_curate(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyJevCurator>()?;
    Ok(())
}
