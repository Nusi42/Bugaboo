# WP-02 — The AI provider: one port, two adapters, off by default, the agreement step

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** AI-01, AI-02, PRIV-08, COMP-08 (what administrators learn about the model before switching it on). ADR-0013, ADR-0018 (settings registry), architecture §7 *AI* and *Stored secrets*; follow-ups M4-F133, M5-F110.

## Goal

Bugaboo can talk to one AI provider that an administrator chose, and to nothing until then. The provider is Anthropic or any OpenAI-compatible endpoint (which covers a local model). Before an outside provider is switched on, the administrator confirms that a data processing agreement is in place and sees which data goes where. Every project follows the organization's switch unless it has its own.

## Owns

`crates/bb-ai/**` (the adapters and the fake behind the feature `testing`), the port `AiProvider` in `bb-app`, `crates/bb-app/src/ai/mod.rs` and `crates/bb-app/src/ai/setup*`, the settings of AI in `crates/bb-core/src/settings/ai*`, the table of providers in `bb-core`, `crates/bb-api/src/api/ai_setup*`, the wiring of the adapter in `bb-server`, the register entry of the outgoing HTTP client (`docs/third-party/`), the edges of `bb-ai` in the module map (M4-F133) and the tests-only edges in ARCH-DEPS (M5-F110), the port offset of the fake provider in `tools/dev/ports.json` (only if a browser test needs a provider that answers over HTTP).

## What is decided (ADR-0013, PRIV-08, the placement in the README)

- **The port `AiProvider`** in `bb-app` takes a request (the system text, the turns, the list of tools with their schemas, a limit of output) and gives back the answer's text and the tool calls it asks for, with the size in and out. It knows nothing of issues or people; the context builder (WP-03) makes the request. One call at a time per request; no streaming in M7 unless the panel needs it (WP-06 decides with WP-10).
- **Two adapters** in `bb-ai`: Anthropic (the Messages API) and OpenAI-compatible (the chat completions shape, which covers a local model). A fake for tests records every request it gets and answers from a script.
- **Off by default.** Organization settings `ai.enabled` (off), `ai.provider`, `ai.model`, `ai.address` (only for the OpenAI-compatible choice), `ai.key` (a stored secret under the master key, written and never read back), `ai.agreement` (who confirmed, when, for which provider). A project setting follows the project above until it has its own. AI counts at a place only when the organization and that place have it on.
- **The agreement step.** Switching on an outside provider needs the confirmation *"A data processing agreement with Anthropic is in place"*, stored with who and when. A local model needs none and is shown as the private choice. Changing the provider asks again.
- **What goes where** is an answer the server gives: the provider, its country, the model, and which data may go (the projects with AI on, every field not kept from AI, names of people as M7-Q5 says). The table of providers (name, country, where its information about the model is) is written once, in `bb-core`.
- **Safety of the call:** a time limit per call and a limit of size of the answer, both named constants; HTTPS only to an outside provider; an address on the local network only for the OpenAI-compatible choice; changing the provider, the address or the key asks for the password again (the lesson of M4-F123); an error of the provider becomes one sentence (*"The AI provider did not answer"*) and never shows what the provider sent back.
- **The security log** records every change of the settings of AI and every confirmation of the agreement.
- **Outgoing HTTP:** one client for the whole server, a new dependency with its register entry (coding-standards §5); WP-01 adds its row to the tech stack.

## Tasks

1. **The settings** of AI in the registry with their defaults and their check of shape; the project switch that follows the one above; *AI counts here* as one function every way in asks (the assistant, MCP, agents).
2. **The port and the adapters**, with the fake; the mapping of tools and tool calls to each provider's shape, tested against recorded examples of both shapes.
3. **The routes:** read and change the settings of AI (organization and project), confirm the agreement, *What goes where*, *Try the provider* (one short call that says whether it answered, for administrators only). Each with its permission (*Change organization settings*, *Change project settings*), its version and its event.
4. **The wiring** in `bb-server`: the adapter the settings choose, made again when the settings change; the fake in tests.
5. **The module map:** the edges of `bb-ai` and `bb-mcp` as the placement says; ARCH-DEPS holds a tests-only edge to `[dev-dependencies]` (M5-F110).

## Acceptance tests (write first)

- `ai_02_ai_is_off_at_the_start_and_no_call_leaves_the_server` (the fake gets nothing; every way in answers *AI is off*).
- `ai_02_a_project_follows_the_organization_until_it_has_its_own_switch`.
- `priv_08_an_outside_provider_needs_the_agreement_before_it_is_switched_on`.
- `priv_08_a_local_model_needs_no_agreement`.
- `priv_08_what_goes_where_names_the_provider_its_country_and_the_data`.
- `ai_01_the_anthropic_adapter_writes_and_reads_the_messages_shape` and `ai_01_the_openai_compatible_adapter_writes_and_reads_the_chat_shape` (recorded examples).
- `ai_01_the_key_is_never_read_back_and_is_stored_as_a_secret`.
- `ai_01_an_error_of_the_provider_never_reaches_the_person` (the sentence only).
- `ai_01_changing_the_address_asks_for_the_password_again`.
- SEC-MATRIX: the routes of the settings of AI, every role and actor kind that exists, both databases.

## Checks

ARCH-SETTINGS, ARCH-DEPS, ARCH-MAP-SYNC, ARCH-HTTP-EDGE, SEC-SECRETS, SEC-MATRIX, DATA-MARKS, DATA-LOGS (no key, no text sent to the provider in a log), DEP-REGISTER, DEP-LICENSES, ARCH-EVENTS, ARCH-VERSIONED.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one table of providers, one *AI counts here*, the settings only in the registry, one outgoing client. A security reviewer reads the agreement step, the secret, the address rules and every error path for what it tells.

## Depends on

Nothing of M7 but the placement in the README. Runs beside WP-03 and WP-04; merges first in wave 1.
