# WP-16 — Agents that wait for work, hand over, and undo

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus
**Features:** AI-31 (hand over), AI-35 (review: *Throw away*), AI-36 (tools for agents), AI-37 (*My agents*), AI-38 (runs). [ADR-0029](../../adr/0029-ai-agents.md), amendment M9; [ADR-0042](../../adr/0042-mcp-server-and-how-ai-apps-sign-in.md); [ADR-0047](../../adr/0047-releases-signatures-parts-list-and-support.md) decision 6; follow-up M7-F60 (the part *Throw away* needs).

## Goal

People who run several agents can do with them what they need. An outside test with several agents hit three walls: an agent cannot wait for work, so it asks again and again; an agent cannot hand work to another agent of the same person; and an agent cannot undo what it did, so what it wrote and cannot take back ends up in what the next agent reads. After this package an agent waits for work with one call that holds its answer, an agent hands over to its person's other agents when its person allows it, an agent undoes its own runs when its person allows it, and *Throw away* also takes away the comments the agent wrote on that work. Both permissions are switches of the agent, set by its person, and start off.

## Owns

- The tool `wait_for_work` and the signal it waits on: `crates/bb-app/src/agent_work/waiting.rs` (new), `crates/bb-app/src/ai/tools/agents.rs`, `inputs.rs`, `registry.rs`, `bb_core::ai::Tool`; the share of connections in `AuthServices` (`with_waiting`), set from the listener's limit in `crates/bb-server/src/wiring.rs`.
- The two switches of an agent: migration 62 (`m0062_agent_switches.rs`: `agents.may_hand_over`, `agents.may_undo`, and on `hand_overs` the agent that made it, `by_agent`, and its place in a chain, `depth`), `bb_core::agent::Agent`, `crates/bb-app/src/agents/manage.rs` (`set_agent_switches`), `PATCH /api/v1/agents/{agent}` with the new optional member `switches` (`crates/bb-api/src/api/agents.rs`).
- The hand-over by an agent: `crates/bb-app/src/agent_work/hand_over.rs` (the use case `hand_over` and `Ctx::agent_hand_over`, one path) and the tool `hand_over_work`.
- The undo by an agent: `crates/bb-app/src/runs/use_cases.rs` (`undo_run` takes the key of an agent whose switch is on) and the tool `undo_my_run`.
- *Throw away* of comments: `crates/bb-app/src/runs/service.rs` and `crates/bb-store/src/keeping/trash.rs` (the deletion of comments, taken out into one function both use), `crates/bb-app/src/comments/`.
- The agent's history: `GET /api/v1/agents/{agent}/history` and its use case in `crates/bb-app/src/agents/`.
- The screens: `web/app/src/features/agents/` (the switches on the agent's page and in *My agents*, the history card), their words in `messages/en.json`, the theme rules in `design/theme.kdl` if a part is new.
- Docs: the ADR-0029 amendment M9, the glossary, `userDocs/agents/my-agents.md`, `userDocs/ai/ai-in-bugaboo.md` (D3, E9, E17 to E19, K1), architecture §4 (events of a comment thrown away) where it names the comment's events.

## What is decided

### 1. Waiting for work (`wait_for_work`)

- **One tool that reads.** `wait_for_work` takes the mark of `changes_since` (`since`), a `limit`, and `seconds` (1 to 300, so at most five minutes, and 60 when not given: below the time after which common proxies, such as Cloudflare at about 100 seconds, and many clients give up on a quiet connection; the API's limit of 30 seconds is for `/api` only and does not cover `/mcp`). It answers at once when the agent has work *Waiting for the agent* (handed over and not taken, or sent back with a note). Otherwise it waits until such work comes to the agent or the seconds are over. Then it answers exactly what `changes_since` answers for the same mark, from the same use case (no second query), and one part `waited` that says whether work waits for the agent now.
- **Level, not edge.** The answer depends on what is stored (is there work waiting for this agent?), never on whether a signal was seen, so a signal that comes between two calls is never lost: the next call answers at once.
- **No stream.** It is one request and one answer; `GET /mcp` stays `405`. Real MCP subscriptions (`resources/subscribe`) stay with the live updates of M15.
- **No database while it waits.** The wait holds no connection and no lock: the use case reads (one statement: the open work of the agent), then waits on an in-process signal of that agent (`WorkSignal`, a `tokio::sync::Notify` per agent that somebody waits for), then reads again. The signal is rung after the commit of every change that gives an agent work waiting for it: a hand-over (by a person, a standing rule or an agent, all through `HandOverPlan::record_change`) and *Send back*. Bugaboo runs as one process, so an in-process signal reaches every waiter.
- **Fair to the listener.** A waiting call holds one connection of the listener. One agent waits once at a time: a second call of the same agent while the first waits answers at once. At most a quarter of the listener's connections wait at once (`WAITING_SHARE_OF_CONNECTIONS = 4`, so 256 of the default 1024); a call over that answers at once. Both answers say *did not wait*, and the agent tries again later.
- **Counted like any call.** The call counts once toward the limits of the key (ADR-0042, decision 6) when it comes in, like every tool call. A wait of a minute is one call where asking every second would be 60.
- **Always allowed.** It only reads, so no switch: any agent may wait. People and AI apps are not offered it (`Tool::is_for_agents`).
- **What it costs** is measured by the test `ai_36_many_waits_cost_one_read_each_and_a_hand_over_wakes_only_its_agent` and written into the report and the ADR amendment: memory per waiting call, the reads a hand-over causes (only the waits of that agent read again), and the time from the commit of a hand-over to the answer.

### 2. Hand-over by an agent

- **A switch of the agent:** *May hand over to my other agents*, set by its person on the agent's page, off when the agent is made. Only the person responsible changes it; an administrator does not (it widens what an agent does for that person).
- **One hand-over path.** The use case `hand_over` keeps taking a session; the agent comes in through `Ctx::agent_hand_over`, and both go through `plan_hand_over` and `HandOverPlan::record_change`, the one place that builds and writes a hand-over. The tool `hand_over_work` (changes, with `preview`) calls that use case and nothing else.
- **What it may hand over:** only issues of its own open work (handed to it, not only found by a standing rule), only to another agent of the same person that can act, never to itself. The person must hold on each issue what a hand-over by the person needs (*Change issues* or *Change the issues they reported*, and *Create agents that work for you* in the project, agents on there), checked as the person, with the budget and the end within the same limits. What the new hand-over lets the other agent change is at most what the agent's own hand-over of that issue lets it change (the ticks and the named fields): an agent cannot give more than it has.
- **Recorded as done by the agent:** the event of the hand-over has the agent as its actor (the one event maker does this for every request of an agent), and the hand-over keeps `by_agent`. The note of the brief is the word of *an agent* when the next agent reads it.
- **Against loops:** never to itself; one open hand-over of an issue per agent (as for people), so A cannot hand an issue back to B while B still has it; and a cap on the chain: a hand-over by a person is at depth 0, a hand-over by an agent one deeper than the agent's own hand-over of that issue, and a chain stops at depth 3 (`MOST_HAND_OVER_DEPTH`). Three steps cover the common chain of plan, build and check; a depth rather than a count per run, because a loop is a chain, while one run may well hand many different issues on once each.

### 3. Undo by an agent

- **A switch of the agent:** *May undo its own runs*, set by its person, off when the agent is made.
- **The tool `undo_my_run`** (changes, with `preview`) names the issue of the agent's work and, if it wants, the attempt (the newest when not given). It runs the existing use case `undo_run` (*Undo the run*) with the run of that attempt: the preview is the preview of the undo, and the undo judges every change again as the agent, skips and names what somebody changed since, exactly as for a person.
- **Only its own runs.** `own_run` already gives an agent only the runs of its own attempts, never the runs of its person. The use case refuses an agent whose switch is off with `access.forbidden`; the undo is recorded as done by the agent.

### 4. *Throw away* takes the agent's own comments with it (M7-F60, the part *Throw away* needs)

- **What counts as the agent's own on that work:** a comment the agent made inside a run of one of its attempts on the issue of that work (the event that made the comment carries that run and has the agent as its actor), with every later text of it. Not its own: a comment of a person, also one the agent may have edited; the description and other long texts, which are a person's text that the agent changed (they stay under M7-F60: putting a long text back needs its own rules for the edit lock, and they are named *cannot be put back yet* as before); the attempts, questions, reports and results of the agent work, which are the record of the review and are not shown to the next agent's work.
- **What goes:** the comment with its earlier texts, its mentions, its marks and its words for the search, and the events that carry its text (with their entries in the outbox), as the purge of the trash deletes a comment: one function of the store, taken out of the purge, that both use. **What stays:** one event on the issue, *A comment of Fixer was thrown away*, with the agent and the moment and no text, so the history says a comment was there.
- **Who throws it away:** the undo of a run does it, so a person's *Throw away* (which undoes every attempt), a person's *Undo the run*, and the agent's `undo_my_run` all do it the same way. The window of *Throw away* counts these comments among what goes back.

### 5. The person's screens

- *My agents* and the agent's page show the two switches, in the card *What it may do*, with words from the glossary and the theme's tokens. A stopped agent shows them without a way to change them.
- The agent's page gains *History*: its hand-overs (to which agent, which issues) and its undos (which issue, how many changes went back), newest first, a page at a time.
- Every word is a key of `messages/en.json`.

### 6. Security and defaults

- SEC-MATRIX: rows for the tools `wait_for_work`, `hand_over_work` and `undo_my_run` and for `GET /api/v1/agents/{agent}/history` and the member `switches`, for every starting role, an access key and the actor kinds of agents, on both databases.
- Both switches start off (ADR-0047, decision 6). They are no setting, so the table of secure starting settings does not hold them: `comp_04_the_switches_of_a_new_agent_start_off` (`crates/bb-core/src/settings/tests.rs`) judges them beside it, and `ai_31_both_switches_of_a_new_agent_are_off` holds that a new agent is made with them.

## Tasks

1. Write the acceptance tests below.
2. Waiting: `WorkSignal`, the use case `wait_for_work`, the tool, the share of connections; the measurement.
3. Migration 62; the switches in the agent, the use case and the route.
4. Hand-over by an agent through the one use case; the tool; the depth.
5. Undo by an agent; the tool.
6. *Throw away* of comments: the deletion of comments out of the purge into one function, the undo's part, the event and its words in the history.
7. The screens and their pictures in four themes on computer and phone; the agent's history.
8. SEC-MATRIX examples; the user docs; architecture §4.

## Acceptance tests (write first)

- `ai_36_wait_for_work_answers_at_once_when_work_waits_for_the_agent`.
- `ai_36_wait_for_work_answers_when_work_is_handed_over_while_it_waits`.
- `ai_36_wait_for_work_answers_when_work_is_sent_back_while_it_waits`.
- `ai_36_wait_for_work_answers_after_its_seconds_with_what_changes_since_answers`.
- `ai_36_wait_for_work_takes_one_second_to_five_minutes`.
- `ai_36_a_second_wait_of_the_same_agent_does_not_wait`.
- `ai_36_waits_over_the_share_of_connections_do_not_wait`.
- `ai_36_many_waits_cost_one_read_each_and_a_hand_over_wakes_only_its_agent` (the measurement).
- `ai_31_both_switches_of_a_new_agent_are_off`.
- `ai_31_only_the_person_of_an_agent_changes_its_switches`.
- `ai_31_an_agent_hands_over_only_when_its_switch_is_on`.
- `ai_31_an_agent_hands_over_only_to_another_agent_of_its_person`.
- `ai_31_an_agent_hands_over_only_issues_of_its_own_work_and_gives_no_more_than_it_has`.
- `ai_31_a_hand_over_by_an_agent_is_recorded_as_done_by_the_agent`.
- `ai_31_a_chain_of_hand_overs_stops_at_its_depth`.
- `ai_38_an_agent_undoes_its_own_run_only_when_its_switch_is_on`.
- `ai_38_an_agent_never_undoes_a_run_of_its_person_or_of_another_agent`.
- `ai_38_an_undo_by_an_agent_is_recorded_as_done_by_the_agent`.
- `ai_35_throw_away_takes_the_comments_of_the_agent_and_keeps_that_they_were_there`.
- `ai_35_throw_away_keeps_the_comments_of_people`.
- `ai_38_undo_my_run_takes_the_comments_of_the_run_too`.
- `ai_37_the_history_of_an_agent_lists_its_hand_overs_and_undos`.
- Browser (later, when the lead says, `--workers=2`): `ai_37 the switches of an agent are changed on its page`, `ai_37 the history of an agent shows a hand-over` (four themes, computer and phone).
- SEC-MATRIX: rows for the three tools, the history route and the member `switches`, both databases.

## Checks

ARCH-AI-TOOLS (each new tool declares reads or changes and has a preview test), ARCH-PERMS, ARCH-EVENTS, ARCH-VERSIONED, DATA-MARKS, DATA-MIGRATIONS, SEC-MATRIX, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, THEME-ONLY, PERF-N1, TEST-A11Y. No new check: every rule above is held by these checks and the named tests.

## Review

Code review with the SSOT/DRY/ODR gate: one hand-over path, one undo, one deletion of comments, `wait_for_work` answering from `changes_since` and from no second query. **Security review:** an agent widening its own powers through another agent; loops; a wait that holds the connections of the listener; what an agent learns from waiting. **Privacy review:** that nothing of a thrown-away comment's text stays in the events, the outbox, the search or the history. **UX review** of the pictures.

## Depends on

M9 waves 0 to 2 merged as far as they are on `WIN1` at `80c280527`. Rebases onto WP-08 (`comp_04_the_switches_of_a_new_agent_start_off` beside its test of secure defaults) and onto the MCP fixes of `win2` (answers of `get_issue`, refusals, *Nothing changed*, *Nothing found*, 413 and 415), whose lines it avoids.
