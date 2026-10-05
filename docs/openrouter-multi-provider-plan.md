# OpenRouter and multi-provider selection handoff

Status (2026-10-05): implemented, tested, and pushed to the fork. This covers OpenRouter, `--provider`, exec model validation, `/provider` (continue, new chat, set default), `/model` after a switch, and per-provider model catalogs. The app-server RPC tests are in `app-server/tests/suite/v2/model_provider_list.rs`. Remaining: step 1 below (live tool-marker and stale-cache checks). See PROGRESS.md for what each piece verified and what it did not.

## Goal

Add an experimental `multi_provider_selection` feature, disabled by default. When enabled, users can choose a provider with `--provider <ID>` or `/provider`, then choose a compatible model. The first new built-in provider is OpenRouter, using `OPENROUTER_API_KEY`, `https://openrouter.ai/api/v1`, the Responses API, and live model discovery.

The final work should remain one cohesive change. Do not create the commit: `AGENTS.md` requires handing the user a ready-to-paste commit command and message.

## Product decisions already approved

- Managed provider requirements have highest precedence.
- Invocation flags override profile and user defaults.
- `codex exec --provider openrouter` requires `--model` unless the persisted default provider/model pair is already OpenRouter-compatible.
- `/provider` lists built-in and valid configured providers with credential readiness, never secret values.
- Provider selection fetches that provider's models before starting it and only shows models suitable for Codex agent workflows.
- Changing providers starts a fresh thread.
- The TUI offers “use once” and “set as default.” Default persistence writes `model_provider` and `model` in one config batch.
- Preserve `--oss`, `--local-provider`, custom providers, Bedrock, and existing OpenAI authentication.
- V1 adds only OpenRouter; no per-provider remembered model map.
- Never write API keys to config or accept them as CLI values.

## Work already applied

The working tree currently contains partial edits in these files:

- `codex-rs/utils/cli/src/shared_options.rs`: added `--provider` and root/subcommand inheritance.
- `codex-rs/exec/src/cli.rs`: made the provider flag global for exec subcommands.
- `codex-rs/exec/src/lib.rs`: routes explicit provider selection into `ConfigOverrides` and rejects combining `--provider` with `--oss`.
- `codex-rs/tui/src/startup_orchestration.rs`: routes the provider override during startup and rejects `--provider` with `--oss`.
- `codex-rs/features/src/lib.rs`: added disabled experimental `MultiProviderSelection`.
- `codex-rs/model-provider-info/src/lib.rs`: added the built-in OpenRouter definition and constants.
- `codex-rs/core/src/config/mod.rs`: removes the OpenRouter built-in when the feature is disabled.
- `codex-rs/model-provider/src/models_endpoint.rs`: started an OpenRouter `data[]` catalog adapter that filters for text input/output and tool support, then maps entries to `ModelInfo`.
- `codex-rs/app-server-protocol/src/protocol/v2/model.rs`: started provider list protocol types and added optional `modelProvider` to `model/list`.
- `codex-rs/app-server-protocol/src/protocol/common.rs`: registered `modelProvider/list`.
- `codex-rs/app-server/src/message_processor.rs`: added dispatch for `modelProvider/list`.

These edits are intentionally uncommitted. `AGENTS.md` is also untracked and belongs to the user; preserve it.

## Completed in the second session

- `CatalogRequestProcessor::model_provider_list` and provider-targeted `model_list` (`catalog_processor.rs`). Both reject when `multi_provider_selection` is disabled. Targeted listing rejects unknown providers, fails early with the provider's `env_key_instructions` when its key is missing, and runs `check_thread_model_provider` on a cloned config before building a throwaway models manager. The cloned config drops `model_catalog` because a configured catalog belongs to the active provider.
- Readiness: `requires_openai_auth` providers are ready when cached auth exists; `env_key` providers when the variable is non-empty; every other provider (local, Bedrock, command auth) is reported ready without probing.
- `ModelListParams` literals updated; JSON/TypeScript/Python SDK schemas, the experimental precomputed exports, and `config.schema.json` regenerated.
- `apply_subcommand_overrides` now carries `--provider`; previously `codex <sub> --provider X` dropped it.
- OpenRouter adapter drops empty IDs and duplicate slugs; decoding tests cover filtering, field mapping, and that malformed bodies are not echoed in errors.

## Remaining steps

1. Confirm the exact OpenRouter tool-capability marker against a live catalog (current filter accepts `tools` or `tool_choice`), and test stale-cache fallback plus the missing `OPENROUTER_API_KEY` path through the app-server RPC.
2. ~~Add app-server RPC tests for `modelProvider/list` and provider-targeted `model/list`.~~ Done 2026-10-05: `app-server/tests/suite/v2/model_provider_list.rs`.

3. ~~Enforce exec model compatibility.~~ Done: `require_model_for_provider` in `exec/src/lib.rs`, covering new, resume, and fork.
4. ~~Implement `/provider` in the TUI.~~ Done, without restarting the app-server: `thread/start` accepts `model_provider` per thread (see PROGRESS.md 2026-10-03 "/provider in the TUI"). `/model` follows the switch as of 2026-10-05; as of 2026-10-05 threads also use their own provider's catalog for model metadata and the sub-agent list.
5. Add tests and documentation.
   - Feature gating, registration, unknown provider, config precedence, CLI inheritance, and `--oss` conflict.
   - Bearer authentication and Responses streaming/tool-call fixtures for OpenRouter.
   - App-server provider list and provider-targeted model list schema/RPC tests.
   - Regressions for OpenAI, Bedrock, Ollama, LM Studio, custom providers, resume filters, and `/model`.
   - Update architecture/config documentation only where the feature introduces non-obvious constraints.

## Verification sequence

Start with formatting and focused compilation because the current partial tree likely has compile errors from new protocol fields and an unfinished handler:

```sh
just fmt
cargo test -p codex-model-provider-info
cargo test -p codex-model-provider
cargo test -p codex-app-server-protocol
cargo test -p codex-app-server
cargo test -p codex-utils-cli
cargo test -p codex-exec
cargo test -p codex-tui
just fmt-check
just clippy
just test
```

Also run the repository's schema generation/check task discovered through `just -l`. Record any checks that cannot complete rather than calling the feature verified.

## Reference

- OpenRouter models: <https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties>
- OpenRouter Responses API: <https://openrouter.ai/docs/api/api-reference/responses/create-responses>
