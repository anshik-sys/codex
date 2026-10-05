//! `modelProvider/list` and provider-targeted `model/list` behind `multi_provider_selection`.

use std::time::Duration;

use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::write_models_cache;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::ModelListParams;
use codex_app_server_protocol::ModelListResponse;
use codex_app_server_protocol::ModelProviderListParams;
use codex_app_server_protocol::ModelProviderListResponse;
use codex_app_server_protocol::RequestId;
use codex_protocol::openai_models::ModelsResponse;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;
const FEATURE_ON: &str = "[features]\nmulti_provider_selection = true\n";

async fn start(
    config_toml: &str,
    env: &[(&str, Option<&str>)],
) -> Result<(TempDir, TestAppServer)> {
    let codex_home = TempDir::new()?;
    write_models_cache(codex_home.path()).await?;
    std::fs::write(codex_home.path().join("config.toml"), config_toml)?;
    let mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .without_auto_env()
        .with_env_overrides(env)
        .build_initialized()
        .await?;
    Ok((codex_home, mcp))
}

async fn request_error(
    mcp: &mut TestAppServer,
    method: &str,
    params: serde_json::Value,
) -> Result<JSONRPCError> {
    let request_id = mcp.send_request(method, Some(params)).await?;
    timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await?
}

#[tokio::test]
async fn provider_rpcs_are_rejected_when_the_feature_is_disabled() -> Result<()> {
    let (_home, mut mcp) = start("", &[]).await?;

    for (method, params) in [
        ("modelProvider/list", json!({})),
        ("model/list", json!({"modelProvider": "openrouter"})),
    ] {
        let error = request_error(&mut mcp, method, params).await?;
        assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE, "{method}");
        assert_eq!(
            error.error.message, "multi_provider_selection feature is disabled",
            "{method}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn provider_list_reports_readiness_without_exposing_keys() -> Result<()> {
    for key in [None, Some("sk-or-test-secret")] {
        let (_home, mut mcp) = start(FEATURE_ON, &[("OPENROUTER_API_KEY", key)]).await?;

        let response: ModelProviderListResponse = mcp
            .request(|request_id| ClientRequest::ModelProviderList {
                request_id,
                params: ModelProviderListParams {},
            })
            .await?;

        let ids = response
            .data
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "providers are sorted by id");
        let openrouter = response
            .data
            .iter()
            .find(|p| p.id == "openrouter")
            .expect("openrouter is listed when the feature is on");
        assert_eq!(openrouter.credential_ready, key.is_some());
        assert_eq!(
            openrouter.credential_env_var.as_deref(),
            Some("OPENROUTER_API_KEY")
        );
        assert!(!openrouter.is_default);
        assert!(
            response
                .data
                .iter()
                .any(|p| p.id == "openai" && p.is_default)
        );
        assert!(!serde_json::to_string(&response)?.contains("sk-or-test-secret"));
    }
    Ok(())
}

#[tokio::test]
async fn targeted_model_list_rejects_unknown_provider_and_missing_key() -> Result<()> {
    let (_home, mut mcp) = start(FEATURE_ON, &[("OPENROUTER_API_KEY", None)]).await?;

    let unknown = request_error(
        &mut mcp,
        "model/list",
        json!({"modelProvider": "no-such-provider"}),
    )
    .await?;
    assert_eq!(unknown.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        unknown.error.message,
        "unknown model provider: no-such-provider"
    );

    let missing_key = request_error(
        &mut mcp,
        "model/list",
        json!({"modelProvider": "openrouter"}),
    )
    .await?;
    assert_eq!(missing_key.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(
        missing_key.error.message.contains("OPENROUTER_API_KEY"),
        "{}",
        missing_key.error.message
    );
    Ok(())
}

#[tokio::test]
async fn targeted_model_list_fetches_a_non_active_providers_catalog() -> Result<()> {
    let server = MockServer::start().await;
    let mut remote_model = codex_models_manager::bundled_models_response()?
        .models
        .remove(0);
    remote_model.slug = "catalog-only-model".into();
    remote_model.visibility = codex_protocol::openai_models::ModelVisibility::List;
    remote_model.supported_in_api = true;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", "Bearer catalog-key"))
        .respond_with(
            ResponseTemplate::new(/*s*/ 200).set_body_json(ModelsResponse {
                models: vec![remote_model],
            }),
        )
        .mount(&server)
        .await;
    let uri = server.uri();
    let (_home, mut mcp) = start(
        &format!(
            r#"{FEATURE_ON}
[model_providers.catalog-test]
name = "Catalog Test"
base_url = "{uri}/v1"
env_key = "CATALOG_TEST_KEY"
model_catalog_url = "{uri}/v1/models"
"#
        ),
        &[("CATALOG_TEST_KEY", Some("catalog-key"))],
    )
    .await?;

    let targeted: ModelListResponse = mcp
        .request(|request_id| ClientRequest::ModelList {
            request_id,
            params: ModelListParams {
                model_provider: Some("catalog-test".to_string()),
                include_hidden: Some(true),
                ..Default::default()
            },
        })
        .await?;
    let active: ModelListResponse = mcp
        .request(|request_id| ClientRequest::ModelList {
            request_id,
            params: ModelListParams::default(),
        })
        .await?;

    assert_eq!(
        targeted
            .data
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["catalog-only-model"]
    );
    assert!(
        active.data.iter().all(|m| m.id != "catalog-only-model"),
        "the active provider's list is unchanged"
    );
    Ok(())
}
