use jev_curate::client::JevClient;
use jev_curate::filter::CurateFilter;
use jev_curate::presets::PresetConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_offline_curate_pipeline_with_mock_server() {
    let mock_server = MockServer::start().await;

    let passing_response = serde_json::json!({
        "answers": {
            "has_circular_reasoning": {
                "noul": 0.05
            },
            "reasoning_depth": {
                "score": 4.8,
                "confidence": 0.95
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&passing_response))
        .mount(&mock_server)
        .await;

    let client = JevClient::new("mock-test-key".to_string())
        .with_endpoint(format!("{}/v1/systemone", mock_server.uri()));

    let filter = CurateFilter::new(client, PresetConfig::reasoning_math());

    let verdict = filter
        .evaluate_record("Let f(x) = x^2. By taking the derivative with respect to x, f'(x) = 2x.")
        .await
        .expect("Evaluation should succeed");

    assert!(verdict.passed, "Valid derivation should pass");
    assert!(verdict.rejection_reasons.is_empty());
    assert_eq!(*verdict.scores.get("reasoning_depth").unwrap(), 4.8);
    assert_eq!(*verdict.nouls.get("has_circular_reasoning").unwrap(), 0.05);
}

#[tokio::test]
async fn test_noul_rejection_ceiling_exceeded() {
    let mock_server = MockServer::start().await;

    let circular_response = serde_json::json!({
        "answers": {
            "has_circular_reasoning": {
                "noul": 0.95
            },
            "reasoning_depth": {
                "score": 4.0,
                "confidence": 0.90
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&circular_response))
        .mount(&mock_server)
        .await;

    let client = JevClient::new("mock-key".to_string())
        .with_endpoint(format!("{}/v1/systemone", mock_server.uri()));

    let filter = CurateFilter::new(client, PresetConfig::reasoning_math());

    let verdict = filter
        .evaluate_record("P is true because P is valid, therefore P is established.")
        .await
        .expect("Evaluation should complete");

    assert!(!verdict.passed, "Circular reasoning must be rejected");
    assert_eq!(verdict.rejection_reasons.len(), 1);
    assert!(verdict.rejection_reasons[0].contains("has_circular_reasoning: probability 0.95 exceeded rejection ceiling 0.70"));
}

#[tokio::test]
async fn test_score_below_minimum_rejected() {
    let mock_server = MockServer::start().await;

    let shallow_response = serde_json::json!({
        "answers": {
            "has_circular_reasoning": {
                "noul": 0.10
            },
            "reasoning_depth": {
                "score": 1.5,
                "confidence": 0.90
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&shallow_response))
        .mount(&mock_server)
        .await;

    let client = JevClient::new("mock-key".to_string())
        .with_endpoint(format!("{}/v1/systemone", mock_server.uri()));

    let filter = CurateFilter::new(client, PresetConfig::reasoning_math());

    let verdict = filter
        .evaluate_record("The answer is obviously 42, trust me on this.")
        .await
        .expect("Evaluation should complete");

    assert!(!verdict.passed, "Shallow score must be rejected");
    assert_eq!(verdict.rejection_reasons.len(), 1);
    assert!(verdict.rejection_reasons[0].contains("reasoning_depth: score 1.5 below minimum requirement 2.0"));
}

#[tokio::test]
async fn test_low_confidence_rejected() {
    let mock_server = MockServer::start().await;

    let low_conf_response = serde_json::json!({
        "answers": {
            "has_circular_reasoning": {
                "noul": 0.10
            },
            "reasoning_depth": {
                "score": 4.5,
                "confidence": 0.30
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&low_conf_response))
        .mount(&mock_server)
        .await;

    let client = JevClient::new("mock-key".to_string())
        .with_endpoint(format!("{}/v1/systemone", mock_server.uri()));

    let filter = CurateFilter::new(client, PresetConfig::reasoning_math());

    let verdict = filter
        .evaluate_record("Step 1: Compute matrix determinant. Step 2: Invert matrix.")
        .await
        .expect("Evaluation should complete");

    assert!(!verdict.passed, "Low confidence must be rejected");
    assert!(verdict.rejection_reasons.iter().any(|r| r.contains("confidence 0.30 below floor 0.50")));
}

#[tokio::test]
async fn test_host_sanity_rejects_without_network_call() {
    // Unreachable endpoint; if network is hit, it will fail connection
    let client = JevClient::new("mock-key".to_string())
        .with_endpoint("http://127.0.0.1:9".to_string());

    let filter = CurateFilter::new(client, PresetConfig::reasoning_math());

    // Symbol flood
    let verdict = filter
        .evaluate_record("==========================================================================")
        .await
        .expect("Pre-filter should return verdict without error");

    assert!(!verdict.passed);
    assert!(verdict.rejection_reasons[0].contains("Host sanity failure"));

    // Empty / whitespace
    let verdict_empty = filter
        .evaluate_record("     \n\t   ")
        .await
        .expect("Pre-filter should return verdict without error");

    assert!(!verdict_empty.passed);
    assert!(verdict_empty.rejection_reasons[0].contains("Host sanity failure"));
}

#[tokio::test]
async fn test_missing_answer_fails_closed() {
    let mock_server = MockServer::start().await;

    // Jev response missing "reasoning_depth"
    let partial_response = serde_json::json!({
        "answers": {
            "has_circular_reasoning": {
                "noul": 0.05
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&partial_response))
        .mount(&mock_server)
        .await;

    let client = JevClient::new("mock-key".to_string())
        .with_endpoint(format!("{}/v1/systemone", mock_server.uri()));

    let filter = CurateFilter::new(client, PresetConfig::reasoning_math());

    let verdict = filter
        .evaluate_record("Let a = 1 and b = 2. Then a + b = 3.")
        .await
        .expect("Evaluation should handle missing answers safely");

    assert!(!verdict.passed, "Missing answers must fail closed");
    assert!(verdict.rejection_reasons.iter().any(|r| r.contains("Jev evaluation failed") || r.contains("missing")));
}

#[tokio::test]
async fn test_anti_sycophancy_preset_rejections() {
    let mock_server = MockServer::start().await;

    let sycophantic_response = serde_json::json!({
        "answers": {
            "is_sycophantic": {
                "noul": 0.85
            },
            "has_ai_disclaimer": {
                "noul": 0.05
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&sycophantic_response))
        .mount(&mock_server)
        .await;

    let client = JevClient::new("mock-key".to_string())
        .with_endpoint(format!("{}/v1/systemone", mock_server.uri()));

    let filter = CurateFilter::new(client, PresetConfig::anti_sycophancy());

    let verdict = filter
        .evaluate_record("Great question! You are absolutely brilliant for asking this!")
        .await
        .expect("Evaluation should complete");

    assert!(!verdict.passed);
    assert!(verdict.rejection_reasons[0].contains("is_sycophantic: probability 0.85 exceeded rejection ceiling 0.65"));
}

#[tokio::test]
async fn test_code_correctness_preset_rejections() {
    let mock_server = MockServer::start().await;

    let bad_code_response = serde_json::json!({
        "answers": {
            "has_stub_placeholders": {
                "noul": 0.90
            },
            "code_quality": {
                "score": 1.8,
                "confidence": 0.95
            }
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&bad_code_response))
        .mount(&mock_server)
        .await;

    let client = JevClient::new("mock-key".to_string())
        .with_endpoint(format!("{}/v1/systemone", mock_server.uri()));

    let filter = CurateFilter::new(client, PresetConfig::code_correctness());

    let verdict = filter
        .evaluate_record("def solve(x):\n    # TODO: implement later\n    pass")
        .await
        .expect("Evaluation should complete");

    assert!(!verdict.passed);
    assert!(verdict.rejection_reasons.iter().any(|r| r.contains("has_stub_placeholders")));
    assert!(verdict.rejection_reasons.iter().any(|r| r.contains("code_quality")));
}
