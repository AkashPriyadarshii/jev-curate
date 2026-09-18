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
    pub fn new(api_key: String, preset: String) -> PyResult<Self> {
        let preset_cfg = presets::PresetConfig::from_name(&preset)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(format!("Unknown preset: {}", preset)))?;
        let client = client::JevClient::new(api_key);
        Ok(Self {
            filter: filter::CurateFilter::new(client, preset_cfg),
        })
    }
}

#[cfg(feature = "python")]
#[pymodule]
fn jev_curate(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyJevCurator>()?;
    Ok(())
}
