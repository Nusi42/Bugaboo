# WP-15 — Each kind of AI its own switch, the look-ups as a setting, pictures in Paper

**Wave 5 · Folder / branch:** `bugaboo.win6` / `WIN6`
**Features:** AI-02 (changed), AI-10 (changed), AI-22, AI-30. ADR-0013, amendment "WP-15"; ADR-0042 decision 4 and ADR-0029, with their amendments of this package (the maintainer's comments of 2026-11-09). Builds on WP-10, WP-12 and WP-14.

## Goal

Agents, AI apps and the assistant are three kinds of AI, and each is its own switch, for the organization and for each project: switching the assistant off in a project no longer shuts agents and AI apps out of it, and the other way round. How many look-ups the assistant may use for one question stops being a number in the code: an administrator sets the most (also *No limit*), and a person may choose a lower number for themselves. Wherever the three are named together, the order is agents, AI apps, and the assistant last, and the guide and the settings pages say at the top that each can be switched on or off by itself. The pictures of the specs are taken in the light theme *Paper*.

## Owns

`crates/bb-app/src/ai/kinds.rs` (new: the switch of a kind, the nearest value along the chain of a project), `never_sent.rs` (`withheld(kind)`), `agents/actor.rs`, `auth/oauth*.rs` (the door of AI apps), `ai/assistant/{ask,step,look_ups}.rs`; `crates/bb-core/src/ai/assistant.rs` (`LookUps`), `crates/bb-core/src/settings/{ai_apps,assistant}.rs` and their words in `messages/en.json`; the project card *Agents, AI apps and the assistant*, the page *Set up the assistant*, the new page *My settings › Assistant* in `web/app/src/features/{settings,my-settings,projects}` and the routes; `userDocs/ai/*` and the generator of its numbers page (`xtask/.../user_pages.rs`); `web/app/e2e/support/screenshots.ts`; the picture set `.work/shots/M7/ai-switches/`.

## What is decided (ADR-0013, amendment WP-15)

- **Each kind follows only its own switch.** The assistant: `ai.enabled`; AI apps: `ai-apps.enabled` (gains the project level); agents: `agents.enabled`. *Never sent to AI* keeps all three out. Nothing stored changes, so there is **no migration** (the next free number stays 49 for the next package).
- **One rule for the chain.** The nearest value along the chain of a project decides; the code is written once (`Ctx::projects_with_kind_off`) and used by the narrowing of agents and by `withheld(kind)`, which every tool, the context builder and MCP read. `ToolCaller` is the kind.
- **The doors.** OAuth and an access key at `/mcp` follow `ai-apps.enabled`; an agent key follows `agents.enabled`; the assistant follows `ai.enabled`.
- **The project settings** show the three switches in one card *Agents, AI apps and the assistant*, in the order agents, AI apps, assistant, each named for its kind, with an intro that says each can be switched by itself and that *Never sent to AI* holds for all three. Agents and AI apps are the registry settings drawn with the choice *As the organization (on|off)* / *On* / *Off*; the assistant keeps its switch.
- **The look-ups.** `assistant.look-ups-most` (organization, administrators; 2, 4, 8, 12, 20, 50, *No limit*; 8 at the start) and `assistant.look-ups` (each person, *My settings › Assistant*; *As the organization* or a number). The lower of the two counts, read when the question is asked. *No limit* is no limit: nothing stops the look-ups of a question. No help text writes a bound.
- **Order and words.** Agents, AI apps, the assistant, in the guide, the README of `userDocs/ai`, the page of numbers, the pages of settings and the glossary (new words: *Kinds of AI*, *AI apps in this project*, *Assistant in this project*, *Look-up*).
- **Pictures.** One place (`pictureLightTheme` in `screenshots.ts`) decides that the light look of a picture is *Paper*. The specs that picture the themes themselves (`inFourThemes` and the like) keep all four.

## Tasks

1. **Write first** (this file, the README of M7, the amendments of the ADRs, the glossary, the follow-ups), then **tests first** on both databases (below).
2. **Core:** `LookUps` (`Most(n)`, `NoLimit`, `lowered_by`, `calls`, `rounds`), `TOOL_CALLS_AT_START` replaces `TOOL_CALLS_MAX` and `ROUNDS_MAX`; the settings and their words; `ai-apps.enabled` at the project level.
3. **App:** `kinds.rs`; `withheld(kind)`; the door of AI apps and of `/mcp`; the assistant reads its look-ups once for a question.
4. **Screens:** the card *Agents, AI apps and the assistant*; the page *Set up the assistant* (AI apps first, then the assistant with its look-ups, a link to the page of agents); the page *My settings › Assistant*; the intro of the agents page.
5. **Docs:** `userDocs/ai/*`, the generated page of numbers, the AI guide at `/help/ai` (it renders `literacy.md`).
6. **Pictures** in Paper: the card *Agents, AI apps and the assistant*, *Set up the assistant*, the agents page and *My settings › Assistant*, computer and phone, in `.work/shots/M7/ai-switches/`, looked at one by one at full size.

## Acceptance tests (write first)

- `m7_wp15_a_project_switch_shuts_out_only_its_own_kind` and `m7_wp15_a_switch_follows_the_nearest_value_along_the_project_chain` (all three callers, one off at a time, a sub-project below a parent).
- `m7_wp15_never_sent_to_ai_holds_for_all_three_kinds`.
- `m7_wp15_the_assistant_off_leaves_ai_apps_and_agents_on_for_the_organization` and `m7_wp15_ai_apps_off_leaves_agents_and_the_assistant_on_for_the_organization`; the door of `/mcp` (`ai_22_an_access_key_is_turned_away_at_mcp_while_apps_are_off_and_an_agent_key_while_agents_are`).
- `m7_wp15_a_project_with_apps_off_and_the_organization_with_apps_off_are_kept_from_an_app` (HTTP, both databases).
- The look-ups: `m7_wp15_the_organization_sets_the_most_look_ups`, `..._is_eight_at_the_start`, `..._no_limit_lets_a_question_look_up_more_than_the_usual_most`, `..._a_person_can_set_a_lower_number_for_themselves`, `..._a_person_cannot_go_above_the_most_of_the_organization`, `..._only_administrators_set_the_most`; unit tests of `LookUps`.
- Browser (computer and phone): the three switches of a project in order, the page of the assistant, the look-ups (`web/app/e2e/ai-switches.spec.ts`); unit tests of the order of the switches.

## Checks

ARCH-SETTINGS, TXT-CATALOG (no bounds in a help text), TXT-TERMS (glossary and user docs), TXT-LITERAL, SEC-MATRIX, ARCH-DB-PARITY, TEST-A11Y, LINT-LINKS, the generated files in step.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the nearest value along the chain is written once; the look-ups are read through `LookUps` in one place; the order of the three has one source for each place it is written. A security reviewer reads: no kind can reach a project through the switch of another; *Never sent to AI* holds for all three; a person cannot go above the most of the organization. A UX reviewer looks at the pictures.

## Depends on

WP-10 (the setup page), WP-12 (the page of agents) and WP-14 merged. No migration.
