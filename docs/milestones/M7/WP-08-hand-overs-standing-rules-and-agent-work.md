# WP-08 — Hand-overs, standing rules and agent work on the server

**Wave 2 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** AI-31, AI-32, AI-33, AI-34, AI-35 (the server side). ADR-0029 decisions 3 to 6, ADR-0034 (saved searches), ADR-0033 (notifications), ADR-0017 (jobs), ADR-0005 (the search language); M7-Q7 and Q8 as recommended.

## Goal

A person gives an agent work in exactly two ways: a hand-over of issues by their IDs, or a standing rule over a saved search with narrow powers. What the agent does on an issue is its own record, apart from the comments: the brief, the attempts, the questions and their answers, the plan, and the review. The issue keeps its person and shows *worked on by* the agent. The review ends the work: *Accept*, *Send back* or *Throw away*, which undoes everything the attempts changed in one step.

## Owns

`crates/bb-app/src/agent_work/**`, `crates/bb-store/src/agent_work*`, `crates/bb-api/src/api/agent_work*`, the notification kinds and the job kinds of agent work, the terms `agent:` and `agent-state:` in `bb-searchlang` and the search compiler, the implementation of `AgentWorkStore` (shape from WP-04), its migrations (numbers from the lead).

## What is decided (ADR-0029, M7-Q7, Q8)

- **A hand-over:** one or several issues by ID, one of the person's agents, the brief (taken from the issue: description, *Steps to reproduce*, *Done when*; plus a note; what is missing is named, never a block), what the agent may do (always: read the issue and what it links to, comment, report, attach results, link related issues; ticked: change named fields, change the status, make sub-issues), a budget (attempts, time, cost as the agent reports it), *Plan needs approval*, an end (the default of ADR-0029 unless set). The person needs *Change issues* (or *Change the issues they reported*) on each issue and *Create agents that work for you* in its project (M7-Q1). One open hand-over per issue and agent.
- **A standing rule:** a saved search, one of the person's agents, what it may do (*Propose*, the default; or create issues in one named project; WP-14 adds *Hand over what it finds* and drops the old *change only named fields*), an end (30 days unless set, at most 90; WP-14: or none when an administrator allows it), *Renew*. Never a field that holds people, never the status. Every issue that newly comes under the rule is written down with the moment (the job `agents.rule-matches`, fed by the outbox).
- **Proposals of a standing rule** are records on the issue's agent work (before and after, the agent, the rule) that a person applies or discards, through the same path as the assistant's *Apply*.
- **Agent work** on an issue: the brief and its answers, the attempts (each a run of WP-07: the plan, the steps the agent reports, the results as links, checks with *passed* or *failed*, the time and cost the agent reports, marked as reported by the agent; never a time entry, M7-Q7), the questions and answers, the review. The states: *Waiting for the agent*, *Agent working*, *Waiting for you*, *Ready for review*, *Agent gave up*, *Stopped*. The issue's own status is not an agent state.
- **Questions and plans:** a question sets *Waiting for you* and notifies the responsible person and the person who handed it over; the answer joins the brief. With *Plan needs approval*, an attempt waits for *Approve plan* or a note.
- **Review:** *Accept* (the hand-over ends; the status moves to the one the reviewer picks, the workflow's proposal first, through the workflow's own change), *Send back* with a note (a new attempt; the note joins the brief), *Throw away* (*Undo the run* of every attempt, through `RunStore`; the window's words say a branch outside Bugaboo is the agent's business). *Accept by itself when its checks pass* is a project setting, off at the start, as M7-Q8 says; issues accepted that way wait under *Accepted by itself* until a person looked at them.
- **One summary comment** marked *Written by an agent* when an attempt is ready, linking to the agent work.
- **Stopped:** when the agent is stopped or its person loses access (the event of WP-04), its open work becomes *Stopped*.
- **The agent's narrowing:** `AgentWorkStore` gives WP-04's actor the issues, ticks and fields of the agent's open hand-overs and rules at a place.
- **Search:** `agent:Fixer` and `agent-state:waiting-for-you` (the glossary's words with spaces work too: `agent state is Waiting for you`), with the agent shown as the reader may see it.
- **Notifications** (ADR-0033 amendment M7): *An agent asks you*, *A plan waits for you*, *Ready for review*, *An agent gave up*, *A standing rule runs out soon*, *An agent was stopped*. **Jobs:** `agents.hand-over-runs-out`, `agents.rule-runs-out`, `agents.rule-about-to-run-out` (3 days before), `agents.rule-matches`.

## Tasks

1. **Tables** of hand-overs, standing rules, agent work, attempts, questions, proposals and reviews; every column marked (DATA-MARKS; briefs, notes, questions and answers as people's text).
2. **Use cases and routes** (`/api/v1/issues/{id}/agent-work`, `/api/v1/hand-overs`, `/api/v1/standing-rules`, `/answer`, `/approve-plan`, `/review`), each with its permission, version and event; lists paged with cursors (PERF-N1). The routes the agent itself calls (take, report, ask, attach, give up) are use cases here and tools in WP-09.
3. **The review** through the workflow's change and `RunStore`; *Accept by itself*.
4. **`AgentWorkStore`** for WP-04's actor.
5. **Search terms** in `bb-searchlang` (both readers: the server and the WebAssembly module) and the compiler, TEST-SEARCH-ROUNDTRIP.
6. **Notifications and jobs** with their settings and wording.

## Acceptance tests (write first)

- `ai_31_a_handed_over_agent_sees_only_its_issues`.
- `ai_31_a_hand_over_ends_at_accept`.
- `ai_31_a_hand_over_needs_change_issues_on_each_issue`.
- `ai_31_the_brief_names_what_is_missing_and_does_not_stop_the_hand_over`.
- `ai_31_a_hand_over_runs_out_after_14_days`.
- `ai_32_a_standing_rule_never_changes_people_fields` (and never the status).
- `ai_32_a_standing_rule_runs_out_after_30_days_and_never_after_more_than_90`.
- `ai_32_every_new_match_is_written_down`.
- `ai_32_a_proposal_of_a_standing_rule_changes_nothing_until_a_person_applies_it`.
- `ai_33_the_issue_keeps_its_person_and_shows_worked_on_by`.
- `ai_33_open_work_becomes_stopped_when_the_person_loses_access`.
- `ai_33_an_agent_never_makes_a_time_entry` (M7-Q7).
- `ai_34_a_question_waits_for_an_answer_and_the_answer_joins_the_brief`.
- `ai_34_with_plan_needs_approval_an_attempt_waits_for_approve_plan`.
- `ai_35_accept_by_itself_is_off_at_the_start`.
- `ai_35_throw_away_undoes_every_attempt_in_one_step` (against the fake of `RunStore`, then the real one after WP-07).
- `ai_33_agent_state_waiting_for_you_finds_the_issue_only_for_who_may_see_it`.
- SEC-MATRIX: every route of agent work, every role and actor kind, both databases.

## Checks

SEC-MATRIX, ARCH-PERMS, ARCH-PERSON-REF, ARCH-EVENTS, ARCH-VERSIONED, ARCH-SETTINGS, DATA-MARKS, PERF-N1, ARCH-DB-PARITY, TEST-SEARCH-ROUNDTRIP, ARCH-WASM.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: *Throw away* through the one undo of runs, *Accept* through the workflow's one change, the proposals applied through the assistant's one path, the search terms in the one parser. A security reviewer reads every way the work of an agent could reach an issue its person did not hand over, and COMP-10 (no people fields, never assigned).

## Depends on

Wave 1 merged (WP-04: agents and the port `AgentWorkStore`), WP-01 (the port `RunStore`, the ADR amendments, the words). Runs beside WP-05, WP-06, WP-07; merges after WP-07.
