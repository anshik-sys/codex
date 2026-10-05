//! `/provider`: pick a provider, then one of its models, then how to apply the pair. A running
//! thread keeps the provider it started with, so every path continues on a new thread: a fork of
//! the current conversation, or a new chat.

use super::*;
use crate::app_event::ProviderSwitch;
use codex_app_server_protocol::ModelListResponse;
use codex_app_server_protocol::ModelProviderListResponse;

impl ChatWidget {
    pub(crate) fn open_provider_popup(&mut self) {
        self.app_event_tx.send(AppEvent::FetchModelProviders);
    }

    pub(crate) fn on_model_providers_loaded(
        &mut self,
        result: Result<ModelProviderListResponse, String>,
    ) {
        let providers = match result {
            Ok(response) => response.data,
            Err(err) => {
                self.add_error_message(format!("Failed to load model providers: {err}"));
                return;
            }
        };
        let current = self.config.model_provider_id.clone();
        let items = providers
            .into_iter()
            .map(|provider| {
                let disabled_reason = (!provider.credential_ready).then(|| {
                    provider.credential_env_var.as_ref().map_or_else(
                        || "Sign in first".to_string(),
                        |var| format!("Set {var} first"),
                    )
                });
                let id = provider.id.clone();
                SelectionItem {
                    name: provider.display_name,
                    description: Some(provider.id.clone()),
                    is_current: provider.id == current,
                    is_default: provider.is_default,
                    is_disabled: disabled_reason.is_some(),
                    disabled_reason,
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::FetchProviderModels {
                            provider: id.clone(),
                        });
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                }
            })
            .collect();
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Select a model provider".to_string()),
            subtitle: Some("Switching starts a new chat".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
        self.request_redraw();
    }

    pub(crate) fn on_provider_models_loaded(
        &mut self,
        provider: String,
        result: Result<ModelListResponse, String>,
    ) {
        let models = match result {
            Ok(response) => response.data,
            Err(err) => {
                self.add_error_message(format!("Failed to load models for {provider}: {err}"));
                return;
            }
        };
        if models.is_empty() {
            self.add_error_message(format!("{provider} has no models usable by Codex."));
            return;
        }
        let items = models
            .into_iter()
            .map(|model| {
                let provider = provider.clone();
                let slug = model.model.clone();
                SelectionItem {
                    name: model.display_name,
                    description: Some(model.model),
                    search_value: Some(slug.clone()),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::ProviderModelChosen {
                            provider: provider.clone(),
                            model: slug.clone(),
                        });
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                }
            })
            .collect();
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(format!("Select a {provider} model")),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            is_searchable: true,
            search_placeholder: Some("Type to search models".to_string()),
            ..Default::default()
        });
        self.request_redraw();
    }

    pub(crate) fn open_provider_apply_prompt(&mut self, provider: String, model: String) {
        let item = |name: &str, description: &str, action: ProviderSwitch| {
            let provider = provider.clone();
            let model = model.clone();
            SelectionItem {
                name: name.to_string(),
                description: Some(description.to_string()),
                actions: vec![Box::new(move |tx| {
                    tx.send(AppEvent::ApplyProviderSelection {
                        provider: provider.clone(),
                        model: model.clone(),
                        action,
                    });
                })],
                dismiss_on_select: true,
                ..Default::default()
            }
        };
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(format!("Use {model} on {provider}?")),
            footer_hint: Some(standard_popup_hint_line()),
            items: vec![
                item(
                    "Continue this conversation",
                    "Fork this chat onto the new provider",
                    ProviderSwitch::Continue,
                ),
                item(
                    "Use once",
                    "New chat; config is unchanged",
                    ProviderSwitch::NewChat,
                ),
                item(
                    "Set as default",
                    "New chat; save the pair to config",
                    ProviderSwitch::SetDefault,
                ),
            ],
            ..SelectionViewParams::confirmation()
        });
        self.request_redraw();
    }
}
