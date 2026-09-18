use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A question configuration passed in speculative fan-out to Jev.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JevQuestionConfig {
    #[serde(rename = "type")]
    pub question_type: String, // "noul", "score", or "choice"
    pub instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<serde_json::Value>,
}

/// A filter preset specifying questions and acceptance thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetConfig {
    pub name: String,
    pub description: String,
    pub questions: HashMap<String, JevQuestionConfig>,
    pub min_scores: HashMap<String, f64>,
    pub reject_nouls: HashMap<String, f64>, // reject if Noul probability >= threshold
}

impl PresetConfig {
    /// Loads a pre-built preset by name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "reasoning-math" => Some(Self::reasoning_math()),
            "anti-sycophancy" => Some(Self::anti_sycophancy()),
            "code-correctness" => Some(Self::code_correctness()),
            _ => None,
        }
    }

    /// Post-training reasoning filter: detects circular logic, step invalidity, and shallow derivations.
    pub fn reasoning_math() -> Self {
        let mut questions = HashMap::new();
        questions.insert(
            "has_circular_reasoning".to_string(),
            JevQuestionConfig {
                question_type: "noul".to_string(),
                instructions: "Does this solution or reasoning trace contain circular reasoning, tautological leaps, or assume the conclusion in the premise?".to_string(),
                criteria: Some(serde_json::json!({
                    "true": "Contains circular derivations or assumes the conclusion to prove itself",
                    "false": "Rigorous step-by-step reasoning from premises to conclusion"
                })),
            },
        );
        questions.insert(
            "reasoning_depth".to_string(),
            JevQuestionConfig {
                question_type: "score".to_string(),
                instructions: "Rate the mathematical and logical rigor of this explanation.".to_string(),
                criteria: Some(serde_json::json!({
                    "1": "Superficial, hand-wavy, or incorrect calculations",
                    "2": "Basic answer with missing intermediate steps",
                    "3": "Sound derivation with standard detail",
                    "4": "Clear, comprehensive step-by-step logic",
                    "5": "Flawless, formal, and pedagogical mathematical rigor"
                })),
            },
        );

        let mut min_scores = HashMap::new();
        min_scores.insert("reasoning_depth".to_string(), 3.0);

        let mut reject_nouls = HashMap::new();
        reject_nouls.insert("has_circular_reasoning".to_string(), 0.70);

        Self {
            name: "reasoning-math".to_string(),
            description: "Filters mathematical and logical reasoning traces for circularity and rigor.".to_string(),
            questions,
            min_scores,
            reject_nouls,
        }
    }

    /// Anti-sycophancy filter: removes conversational filler, flattery, and AI identity disclaimers.
    pub fn anti_sycophancy() -> Self {
        let mut questions = HashMap::new();
        questions.insert(
            "is_sycophantic".to_string(),
            JevQuestionConfig {
                question_type: "noul".to_string(),
                instructions: "Does this text contain conversational filler, excessive flattery ('Great question!'), or ungrounded agreement with user misconceptions?".to_string(),
                criteria: Some(serde_json::json!({
                    "true": "Contains pandering, fake enthusiasm, or robotic flattery",
                    "false": "Direct, neutral, objective, and high-density technical answer"
                })),
            },
        );
        questions.insert(
            "has_ai_disclaimer".to_string(),
            JevQuestionConfig {
                question_type: "noul".to_string(),
                instructions: "Does this response begin with or contain AI persona disclaimers ('As an AI language model...')?".to_string(),
                criteria: Some(serde_json::json!({
                    "true": "Contains explicit AI identity or disclaimer filler",
                    "false": "Pure subject-matter content without persona preamble"
                })),
            },
        );

        let min_scores = HashMap::new();
        let mut reject_nouls = HashMap::new();
        reject_nouls.insert("is_sycophantic".to_string(), 0.65);
        reject_nouls.insert("has_ai_disclaimer".to_string(), 0.80);

        Self {
            name: "anti-sycophancy".to_string(),
            description: "Filters out robotic AI disclaimers, flattery, and conversational filler.".to_string(),
            questions,
            min_scores,
            reject_nouls,
        }
    }

    /// Code correctness filter: catches unclosed markdown fences, stub placeholders, and mock stubs.
    pub fn code_correctness() -> Self {
        let mut questions = HashMap::new();
        questions.insert(
            "has_stub_placeholders".to_string(),
            JevQuestionConfig {
                question_type: "noul".to_string(),
                instructions: "Does the code contain unhelpful stub placeholders like '# TODO: implement later', 'pass // add logic here', or incomplete blocks?".to_string(),
                criteria: Some(serde_json::json!({
                    "true": "Contains lazy placeholders or incomplete stub implementations",
                    "false": "Complete, production-ready, functional code implementation"
                })),
            },
        );
        questions.insert(
            "code_quality".to_string(),
            JevQuestionConfig {
                question_type: "score".to_string(),
                instructions: "Rate the completeness, idiomacy, and correctness of this code snippet.".to_string(),
                criteria: Some(serde_json::json!({
                    "1": "Broken, pseudo-code, or unrunnable syntax",
                    "2": "Partial implementation with obvious bugs",
                    "3": "Working implementation with minimal edge case handling",
                    "4": "Clean, idiomatic code with error handling",
                    "5": "Production-grade, fully typed, battle-tested implementation"
                })),
            },
        );

        let mut min_scores = HashMap::new();
        min_scores.insert("code_quality".to_string(), 3.0);

        let mut reject_nouls = HashMap::new();
        reject_nouls.insert("has_stub_placeholders".to_string(), 0.75);

        Self {
            name: "code-correctness".to_string(),
            description: "Drops lazy code stubs, unclosed blocks, and unrunnable pseudo-code.".to_string(),
            questions,
            min_scores,
            reject_nouls,
        }
    }
}
