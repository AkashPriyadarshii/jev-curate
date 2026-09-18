use crate::client::JevClient;
use crate::presets::PresetConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Outcome of evaluating a single record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurateVerdict {
    pub passed: bool,
    pub rejection_reasons: Vec<String>,
    pub scores: HashMap<String, f64>,
    pub nouls: HashMap<String, f64>,
}

/// Filter engine running host sanity checks and Jev fan-out evaluations.
pub struct CurateFilter {
    client: JevClient,
    preset: PresetConfig,
    max_chars_per_row: usize,
}

impl CurateFilter {
    pub fn new(client: JevClient, preset: PresetConfig) -> Self {
        Self {
            client,
            preset,
            max_chars_per_row: 32_000, // ~8,000 tokens ceiling to prevent context rot
        }
    }

    /// Fast host-side sanity check before spending any API tokens.
    /// Returns Ok(clean_text) or Err(rejection_reason).
    pub fn pre_filter_sanity(&self, text: &str) -> Result<String, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("Empty or whitespace-only record".to_string());
        }

        if trimmed.len() < 10 {
            return Err("Record too short (< 10 chars)".to_string());
        }

        // Check for repetitive character flood (e.g. "===========" or "-----------")
        let non_symbol_count = trimmed.chars().filter(|c| c.is_alphanumeric()).count();
        if non_symbol_count == 0 || (trimmed.len() > 100 && (non_symbol_count as f64 / trimmed.len() as f64) < 0.20) {
            return Err("Repetitive non-alphanumeric symbol flood".to_string());
        }

        // Enforce hard ceiling to prevent context rot per TypeSafe skill
        if trimmed.len() > self.max_chars_per_row {
            return Ok(trimmed[..self.max_chars_per_row].to_string());
        }

        Ok(trimmed.to_string())
    }

    /// Evaluates a single record against the configured preset.
    pub async fn evaluate_record(&self, text: &str) -> Result<CurateVerdict> {
        // Stage 1: Host-side sanity check
        let clean_text = match self.pre_filter_sanity(text) {
            Ok(cleaned) => cleaned,
            Err(reason) => {
                return Ok(CurateVerdict {
                    passed: false,
                    rejection_reasons: vec![format!("Host sanity failure: {}", reason)],
                    scores: HashMap::new(),
                    nouls: HashMap::new(),
                });
            }
        };

        // Stage 2: Speculative parallel fan-out evaluation
        let state = serde_json::json!({
            "text": clean_text
        });

        let answers = self.client.evaluate(state, &self.preset.questions).await?;

        let mut passed = true;
        let mut rejection_reasons = Vec::new();
        let mut scores = HashMap::new();
        let mut nouls = HashMap::new();

        // Check Noul rejection thresholds
        for (q_name, threshold) in &self.preset.reject_nouls {
            if let Some(ans) = answers.get(q_name) {
                if let Some(noul_val) = ans.noul {
                    nouls.insert(q_name.clone(), noul_val);
                    if noul_val >= *threshold {
                        passed = false;
                        rejection_reasons.push(format!(
                            "{}: probability {:.2} exceeded rejection ceiling {:.2}",
                            q_name, noul_val, threshold
                        ));
                    }
                }
            }
        }

        // Check Score minimum thresholds
        for (q_name, min_score) in &self.preset.min_scores {
            if let Some(ans) = answers.get(q_name) {
                if let Some(score_val) = ans.score {
                    scores.insert(q_name.clone(), score_val);
                    if score_val < *min_score {
                        passed = false;
                        rejection_reasons.push(format!(
                            "{}: score {:.1} below minimum requirement {:.1}",
                            q_name, score_val, min_score
                        ));
                    }
                }
            }
        }

        Ok(CurateVerdict {
            passed,
            rejection_reasons,
            scores,
            nouls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_filter_drops_blanks_and_symbol_floods() {
        let client = JevClient::new("dummy".to_string());
        let filter = CurateFilter::new(client, PresetConfig::reasoning_math());

        assert!(filter.pre_filter_sanity("").is_err());
        assert!(filter.pre_filter_sanity("   \n\t ").is_err());
        assert!(filter.pre_filter_sanity("==========================================================================").is_err());
        assert!(filter.pre_filter_sanity("Valid mathematical reasoning step: Let x = 5.").is_ok());
    }
}
