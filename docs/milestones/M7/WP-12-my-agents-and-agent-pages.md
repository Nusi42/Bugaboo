# WP-12 — *My agents*, an agent's page, standing rules, the administrators' list

**Wave 3 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** AI-37, AI-30 (on screen), AI-32 (the standing-rule window). UX design §4.17, §5.1, the pictures WP-01 draws; ADR-0029 decisions 2, 3 and 8.

## Goal

One person can drive several agents and see at a glance what waits for them: *My agents* lists questions, plans to approve, results to review and standing rules about to run out first, then each agent with what it works on, then the standing rules. Each agent can be paused or stopped, and *Stop all my agents* stops every one in one step. On a phone, *Waiting for you* comes first, so a question can be answered on the way. Administrators see every agent of the organization and can stop any of them.

## Owns

`web/app/src/features/agents/**` (*My agents*, *Create an agent*, an agent's page with its keys and limits, the standing-rule window), *My agents* in the sidebar and the phone's bottom bar, `web/app/src/features/settings/agents*` (*Organization settings › Agents* and the project's switch), the theme rules of these parts.

## What is decided (ADR-0029, UX §4.17)

- ***My agents*** (computer): *Waiting for you* on top (each row: the issue, the agent, what waits, a button to the place where it is answered), then one section per agent (its state, what it works on with the agent state of each issue, *Pause* or *Resume*, *Stop*), then *Standing rules* (the search, the agent, what it may do, when it runs out, *Renew*), then *Accepted by itself* while a person has not looked. *Stop all my agents* at the top, with a question first.
- ***My agents*** (phone): *Waiting for you* first, the agents below; the same buttons at 44 px.
- ***Create an agent***: a name and nothing else at first; its page then shows its keys (*Create an agent key*, the secret shown once, *Renew*, *Take back*), its limits within the organization's maximum, its work, and *Hand to another person*.
- **The standing-rule window** (§6.3): the saved search (pick one or save the current one), the agent, *Propose* (default) or the fields it may change from the short list, or the project to create issues in, the end (30 days, at most 90).
- ***Organization settings › Agents***: every agent with its person (by *Who sees people*; administrators see names), its state and last use; *Stop*; *Agents* on or off for the organization; the project settings get the switch *Agents in this project*.
- **The sidebar** gets *My agents* only when the person has an agent or may create one; the sidebar still fits a window 600 px high (M5-F150).

## Tasks

1. ***My agents*** on computer and phone, with the agent mark of WP-11 from `main`.
2. ***Create an agent***, its page, its keys and limits.
3. **The standing-rule window**, from *My agents* and from a saved search's menu.
4. ***Organization settings › Agents*** and the project switch, through the one settings form.
5. **Pictures** of every changed screen in four themes on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `ai_37_my_agents_shows_waiting_for_you_first`.
- `ai_37_on_a_phone_my_agents_shows_waiting_for_you_first`.
- `ai_37_stop_all_my_agents_asks_first_and_stops_every_one`.
- `ai_37_pause_and_resume_an_agent`.
- `ai_30_an_agent_key_is_shown_once_and_can_be_taken_back`.
- `ai_32_a_standing_rule_offers_only_the_fields_of_the_short_list_and_never_people_or_the_status`.
- `ai_32_a_standing_rule_ends_at_most_90_days_ahead`.
- `ai_30_an_administrator_stops_any_agent_from_organization_settings`.
- `ux_01_the_sidebar_with_my_agents_fits_a_window_600_px_high`.
- TEST-A11Y on every changed screen; the keyboard alone stops, pauses and makes a rule.

## Checks

TEST-A11Y, THEME-ONLY, STY-PARTS, STY-BOLD, STY-TOKENS, TXT-TERMS, TXT-DATES, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the agent mark and the state words from WP-11, the key part shared with access keys (no second key screen), the settings through the one settings form, the saved-search picker reused. A UX reviewer compares every picture with UX §4.17, §1.1 and §7. A words reviewer reads every new message.

## Depends on

Wave 2 merged (WP-08; WP-04 from wave 1). Merges last in wave 3, after WP-11.
