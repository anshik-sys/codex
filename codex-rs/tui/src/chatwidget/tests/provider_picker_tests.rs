//! `/provider` stages: provider readiness, the use-once/default choice, and load failures.

use super::*;
use codex_app_server_protocol::ModelProvider;
use codex_app_server_protocol::ModelProviderListResponse;
use pretty_assertions::assert_eq;

fn provider(id: &str, name: &str, ready: bool, env_var: Option<&str>) -> ModelProvider {
    ModelProvider {
        id: id.to_string(),
        display_name: name.to_string(),
        is_default: id == "openai",
        credential_ready: ready,
        credential_env_var: env_var.map(str::to_string),
    }
}

#[tokio::test]
async fn provider_popup_shows_readiness_and_selects_provider() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.on_model_providers_loaded(Ok(ModelProviderListResponse {
        data: vec![
            provider(
                "openai", "OpenAI", /*ready*/ true, /*env_var*/ None,
            ),
            provider(
                "openrouter",
                "OpenRouter",
                /*ready*/ false,
                Some("OPENROUTER_API_KEY"),
            ),
        ],
    }));

    assert_snapshot!(
        "provider_popup_readiness",
        render_bottom_popup(&chat, /*width*/ 80)
    );

    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_matches!(
        rx.try_recv(),
        Ok(AppEvent::FetchProviderModels { provider }) if provider == "openai"
    );
}

#[tokio::test]
async fn provider_apply_prompt_offers_use_once_and_default() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.open_provider_apply_prompt("openrouter".to_string(), "x/agent".to_string());

    assert_snapshot!(
        "provider_apply_prompt",
        render_bottom_popup(&chat, /*width*/ 80)
    );

    chat.handle_key_event(KeyEvent::from(KeyCode::Down));
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    let Ok(AppEvent::ApplyProviderSelection {
        provider,
        model,
        persist,
    }) = rx.try_recv()
    else {
        panic!("expected ApplyProviderSelection");
    };
    assert_eq!(
        (provider.as_str(), model.as_str(), persist),
        ("openrouter", "x/agent", true)
    );
}

#[tokio::test]
async fn provider_model_load_failure_reports_error_without_popup() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(Some("gpt-5.5")).await;
    chat.on_provider_models_loaded(
        "openrouter".to_string(),
        Err("Missing environment variable: `OPENROUTER_API_KEY`.".to_string()),
    );

    assert!(chat.no_modal_or_popup_active());
    let history = drain_insert_history(&mut rx)
        .into_iter()
        .map(|lines| lines_to_single_string(&lines))
        .collect::<String>();
    assert!(
        history.contains("Failed to load models for openrouter"),
        "{history}"
    );
}
