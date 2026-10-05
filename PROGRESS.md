# Progress

This file is committed so daily context follows the code across worktrees.

## 2026-10-03 — working tree lost to a failed nested clone, rebuilt

At 15:33 IST the whole checkout disappeared, including three unpushed local commits and the uncommitted `/provider` work. Cause: at 13:14 a separate Codex session ran `git clone git@github.com:anshik-sys/codex.git .` in the *parent* folder `PROJECTS/codex`. That turn was interrupted, but the clone kept running in the background while this checkout was cloned into `PROJECTS/codex/codex`. After 2 h 19 min GitHub dropped the connection (`fatal: early EOF`). A failed clone into an existing directory empties that directory, so the nested checkout went with it. Files removed this way skip the Trash, and the Time Machine destination was not mounted.

Rebuilt the same day on a fresh `--depth 50` clone at `19e554bb7`:
- The first session's partial diff was re-applied from a `git diff` captured in the Claude Code session log.
- Every later edit was replayed from that log, and the schemas were regenerated.
- `AGENTS.md` was restored from the copy in the session's instructions.

State when the session ended (2026-10-03):
- **Verified after the rebuild:** everything compiles, tests included. Test counts match the pre-loss runs: app-server-protocol 319, exec 64 + 1 + 103, model-provider 104 (including the OpenRouter tests), utils-cli 42, plus 33 and 21.
- **Verified on 2026-10-05:**
  - The `/provider` TUI tests pass (3/3). Their two snapshots were regenerated and match the pre-loss output line for line.
  - The app-server `model_list` suite passes 18/20 at `--test-threads=2`. The two Bedrock `bedrock_model_list_advertises_ultrafast_without_changing_default` cases fail under load but pass alone, the same timeout pattern as before the loss.
  - The debug binary builds.
- **Not re-run:** the live OpenRouter smoke test.
- **Commits:** the earlier local commits (`f9081bd2c`, `b45b74b89`, `615cc727c`) no longer exist. The rebuilt work was committed as `0b54bc0` and pushed to the fork; the snapshot files follow in a separate commit.

Lesson: clone into a new named subdirectory, never `.`, and never run two clones where one target is inside the other. A full-history clone of this repo is slow enough to outlive the session that started it.

## 2026-10-03 — /provider in the TUI

The handoff plan assumed a provider change needed a restart, because "provider runtime objects are fixed at thread/app-server startup". That holds for a running thread, not for new ones. `thread/start` already takes `model_provider` and builds that thread's config from it; only `thread/resume` rejects a mismatch. So `/provider` sets `harness_overrides.model_provider`/`model` plus `AppServerSession::model_provider_override`, then calls the same `start_fresh_session` as `/new`. No new protocol was needed.

All three settings are required. The harness override alone would leave `thread_start_params_from_config` sending no provider, because `explicit_provider` only reports session-flag and profile layers. The app-server would then use its startup default. "Use once" therefore lasts until Codex exits, not for exactly one thread. "Set as default" writes `model_provider`, `model`, and clears `model_reasoning_effort` in one `ConfigBatchWrite`, since an OpenAI effort default may not apply to the new provider.

The command is hidden unless `multi_provider_selection` is on, following the `/goal` flag plumbing (`provider_command_enabled`). It is treated like `/new`: unavailable during a task, and it stops queued-input draining.

Known gaps: after switching, `/model` and the model display still read the startup provider's catalog, because the TUI caches models from app-server startup. The model-list stage has no success snapshot, because the protocol `Model` type has no cheap constructor; readiness, the apply prompt, and the load-error path are covered. Unverified: the interactive flow in a real terminal, and an inference turn through OpenRouter.

## 2026-10-03 — exec requires a model with --provider

Without this, `codex exec --provider openrouter` kept the OpenAI default model. That only failed once OpenRouter rejected the name, after startup and auth had already run. Now `--provider X` without `-m` is accepted only when the merged config (config files and `-c` overrides) sets `model_provider = "X"` together with a model.

The check runs in `run_main` before the new/resume/fork split, so one call covers all three. All three were checked against the built binary, and each fails before any network request. Known conservative edge: a legacy `profile =` whose profile table sets the pair is not seen at the top level of `ConfigToml`, so it is rejected; the user passes `-m` instead. The TUI does not apply this check; its `/provider` flow will pick the model explicitly.

## 2026-10-03 — provider list and targeted model list in app-server

The first session's tree did not compile beyond the low-level crates. It also hid a real bug: `apply_subcommand_overrides` ignored `--provider`, so `codex <sub> --provider X` silently ran on the default provider. A first `cargo check` reported `codex-utils-cli` clean, but that was wrong; only a later `--tests` check over the dependents surfaced the error.

Targeted `model/list` builds a throwaway models manager from a cloned config rather than adding a per-provider manager cache. This was chosen because it is the shortest path and reuses `build_models_manager` plus the existing on-disk cache, which is already provider-scoped by identity hash. Revisit if the picker shows noticeable latency. The handler checks the provider's API key before discovery because, with a missing key, the provider-catalog manager would otherwise return an empty list instead of a useful error.

Credential readiness for providers without `env_key` or OpenAI auth (local, Bedrock, command auth) is reported as ready without probing. This is a deliberate shortcut, flagged with a `ponytail:` comment.

Response types may not use `#[ts(optional = nullable)]`; the protocol crate's export test enforces that it appears only in `*Params`. The experimental precomputed exports need a separate `write_schema_fixtures.py --experimental` run.

The 7 failures in the 20-test app-server `model_list` filter under default parallelism pass in isolation and pass together at `--test-threads=2`, so they are load timeouts rather than regressions. Live smoke test against the built binary, run as an `app-server` stdio session with a throwaway `CODEX_HOME`:
- With the feature off, both RPCs are rejected.
- With it on and no key, the provider list reports OpenRouter as not ready, and the targeted model list returns the `OPENROUTER_API_KEY` instructions.
- With a real key, the live OpenRouter catalog is returned, filtered and mapped.

Still unverified: an actual inference turn through OpenRouter's Responses endpoint, and whether `tools`/`tool_choice` is the right capability marker.

In `codex-tui --lib`, `analytics::plan::tests::plan_gate_prevents_requests_and_unavailable_plan_does_not_block_other_reports` overflows the default debug-build stack and aborts the whole test binary; this is unrelated to provider work. With `RUST_MIN_STACK=16777216` the suite runs. Two app tests failed under full parallelism and pass in isolation.

## 2026-10-03 — begin openrouter provider selection

Codex could configure custom response-compatible providers, but it had no feature-gated provider picker or built-in OpenRouter catalog adapter. The approved direction keeps provider choice paired with model choice because carrying an OpenAI default model across providers can fail late and ambiguously. OpenRouter discovery is normalized at the provider boundary so the existing model manager, cache, protocol, and picker can continue using `ModelInfo`.

Implementation stopped before compilation because the session usage limit was near exhaustion. The current tree is explicitly unverified and incomplete: app-server handlers, TUI `/provider`, exec pair validation, protocol generation, tests, formatting, linting, and runtime checks remain. See `docs/openrouter-multi-provider-plan.md` for the continuation checklist and current file-level state.
