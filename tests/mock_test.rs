use jev_curate::client::JevClient;
use jev_curate::filter::CurateFilter;
use jev_curate::presets::PresetConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_offline_curate_pipeline_with_mock_server() {
    let mock_server = MockServer::start().await;

    // Response 1: Clean passing record
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
