# WP-13 — Sample organization part 6, agents under load, speed, the AI literacy guide, the exit demo

**Wave 4 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** COMP-11; AI-30 to AI-38 and AI-20 under load; PERF-01. Architecture §11 (*Sample organization*, *Ten people at the same time*, *Large dataset*).

## Goal

M7 is shown working on the sample and proven under load and on 1 M issues: the exit demo runs; ten people and their agents work at the same moments and every rule holds; *changes since*, the search of agent states and *My agents* meet PERF-01. Administrators and people read a short guide to AI in their own words.

## Owns

`tools/dev/sample.json` and `bb-sample` (AI and agents), `dataset large` (agent work), the ten-people scenarios of agents (`bb-load-check`, the browser scenarios), the speed scenarios of M7 in `tools/checks/perf-bench.toml`, the M7 exit demo spec and its section in architecture §11, `userDocs/ai/**` (the AI literacy guide and the user docs of M7, with numbers taken from the code).

## Tasks

1. **Sample part 6:** AI on for the organization with the test fake and the agreement confirmed by an administrator; *Cost center* and one project *Never sent to AI*; two agents of one Team member (architecture §11), each with a key; hand-overs in every agent state, with questions, an answer, a sent-back attempt and an accepted one; a standing rule over *new bugs in Website* that proposes; one AI app allowed by a Contributor. The sample's own rules (TEST-SAMPLE-DATA) check that no agent holds a role, is *Assigned to* anything or works on an issue its person cannot change, that every standing rule runs out within 90 days and never names a people field, and that every mark of AI-written text matches its event.
2. **Ten people with agents** (architecture §11): agents take, report and ask while their people answer and review; two agents take the same issue at the same moment; an agent works while its person loses access. In the load run on both databases and in the browser scenarios on SQLite; `bb-load-check` judges: one taker per hand-over, no change by an agent beyond its person, every change with its event and its run, every undone run complete or named.
3. **Speed on 1 M issues:** `dataset large` adds agent work on 1 % of the issues and a match record of standing rules (the data is made by the code that is measured); the scenarios `agent-changes-since`, `search-agent-state` and `my-agents` within PERF-01 on both databases in the nightly run; numbers into §11.
4. **The AI literacy guide** (COMP-11): what the assistant, AI apps and agents can and cannot do, where data goes and on which agreement, *Never sent to AI*, how to check what an AI wrote (the marks, the runs, *Undo the run*), what Bugaboo never lets AI do (COMP-10). Linked from the assistant panel and from *Set up the assistant* (WP-10). Plus the user docs of the assistant, AI apps (how to connect Claude Code), agents, hand-overs, standing rules and *My agents*.
5. **The exit demo** (04-milestones M7, with the maintainer's answer to M7-Q9): an MCP client in the test connects as a Contributor and does not see *Cost center*; the assistant answers "Summarize WEB-142" (the fake answers) and proposes a priority that is saved only after *Apply*; Ada hands two issues to her two agents; one asks a question that Ada answers on the phone; the other is sent back once and then accepted; a third attempt is thrown away and its changes are undone in one step; a standing rule over new bugs proposes and changes nothing. Each step a test, with pictures in the four themes on computer and phone.

## Acceptance tests (write first)

- TEST-SAMPLE-DATA: the rules of part 6.
- TEST-CONCURRENT: "two agents take the same issue at the same moment" and "an agent works while its person loses access", on both databases.
- PERF-BENCH: the nightly run green on both databases with the three new scenarios.
- The exit demo spec, step by step (`web/app/e2e/m7-exit-demo.spec.ts`).

## Checks

TEST-SAMPLE-DATA, TEST-CONCURRENT, PERF-BENCH, TEST-A11Y, TXT-TERMS (the user docs), LINT-LINKS.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the sample through the use cases, the large dataset through the one bulk writer of M5 WP-03, the numbers and limits in the user docs read from the code. A UX reviewer looks at the demo pictures; a words reviewer reads the guide for plain language. Then the exit visual pass of the milestone (README).

## Depends on

WP-09, WP-10, WP-11 and WP-12 merged, and M6 WP-12 merged (sample part 5 and the M6 scenarios it builds on).
