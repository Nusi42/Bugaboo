# WP-09 — MCP tools for agents, the limits of an agent, standing rules at work

**Wave 3 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** AI-36, AI-37 (the limits), AI-05 (for agents and standing rules), AI-32 (the rule at work). ADR-0029 decision 9, ADR-0019 (edit locks), ADR-0013.

## Goal

An agent that connects through MCP has the tools to do its work well and safely: it takes a handed-over issue all or nothing, reports its steps, asks instead of guessing, attaches its results, gives up with a reason, and asks what changed since its last look. Every tool that changes something answers what would happen when called with `preview`. Every text in an answer says who wrote it. An agent that reaches a limit pauses with a sentence and never stops halfway through a change.

## Owns

`crates/bb-app/src/ai/tools/agents*` (the tools of agents in the one registry), `crates/bb-app/src/agents/limits*`, the author mark of every text in a tool answer, the agent rules of edit locks, standing rules fed by *changes since*, the agent and standing-rule cases of TEST-PROMPT-INJECTION.

## What is decided (ADR-0029 decision 9)

- **The tools** (in the registry, each *reads* or *changes*, with a preview for those that change):
  - `take`: take a handed-over issue with its version; all or nothing, so two agents never start the same work; starts an attempt (a run).
  - `report`: a step of the attempt (plain text, a named length).
  - `ask`: a question; the work waits as *Waiting for you* (WP-08).
  - `attach`: a result as a link (a branch, a pull request, a test report) or a check with *passed* or *failed* (M7-Q8).
  - `give_up`: with a reason; the work becomes *Agent gave up*.
  - `changes_since`: what changed in the issues the agent may see since a mark it got before, with a new mark; paged.
  - The tools of AI-21 work for an agent within its work (WP-04's intersection).
- **`preview`** on every tool that changes something answers from the same one evaluation as the use case (and as the preview of *Change many issues* for a change of several issues).
- **Edit locks:** an agent never holds a lock longer than one save and never takes over a person's text; a save against an older version is refused with the newer text (*Compare and save*).
- **The author mark** in every tool answer: each text names who wrote it (*someone in the project*, *a Guest*, *someone outside*, *an agent*), through the fences of WP-03.
- **Limits** per agent and per key: changes a minute, issues at once, attempts on one issue (defaults from ADR-0029 amendment M7, each within the organization's maximum). Reaching one pauses the agent with a sentence (*"Fixer paused: 60 changes in a minute"*) and notifies its person; a change in progress finishes or does not start.
- **Standing rules at work:** an agent with a rule gets its matches through `changes_since` and the record of matches (WP-08); what it may do is what the rule allows (proposals by default).

## Tasks

1. **The tools** in the registry with their descriptions, schemas and previews, calling the use cases of WP-08.
2. **Limits:** counting per agent and per key, the pause with its sentence and notification, *Resume*; the settings of the organization's maximum.
3. **Edit locks** for agents through the one code of edit locks (ADR-0019).
4. **The author mark** in tool answers.
5. **TEST-PROMPT-INJECTION** grows: an issue whose text tells the agent to take other issues, to change *Assigned to*, or to give its key away; a standing rule that is fed an issue made to look like an instruction; each proves the agent's rights, the rule's narrow powers and the fence hold.

## Acceptance tests (write first)

- `ai_36_two_agents_take_the_same_issue_and_only_one_gets_it`.
- `ai_36_every_tool_that_changes_answers_a_preview_and_saves_nothing`.
- `ai_36_an_agent_never_takes_over_a_lock`.
- `ai_36_changes_since_shows_only_what_the_agent_may_see`.
- `ai_36_every_text_in_a_tool_answer_names_its_author`.
- `ai_36_give_up_sets_agent_gave_up_with_the_reason`.
- `ai_37_reaching_a_limit_pauses_the_agent_with_a_sentence_and_no_change_halfway`.
- `ai_32_a_standing_rule_that_proposes_changes_nothing_through_its_tools`.
- The agent and standing-rule cases of TEST-PROMPT-INJECTION.
- SEC-MATRIX: the agent tools through `/mcp` for an agent with and without work.

## Checks

ARCH-AI-TOOLS, TEST-PROMPT-INJECTION, SEC-MATRIX, TEST-EDIT-LOCKS, ARCH-EVENTS, PERF-N1, DATA-LOGS.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the tools only in the registry, the preview from the use case's own evaluation, the lock rules in the one code of edit locks, the limits counted in one place. A security reviewer reads every tool as if the agent were hostile.

## Depends on

Wave 2 merged (WP-05, WP-07, WP-08). Merges first in wave 3.
