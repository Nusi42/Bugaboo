# M7 — Assistant, AI apps and agents: work packages

Goal, features and exit demo: see [04-milestones.md](../../04-milestones.md) (M7). Rules: "Working in a folder", "Life of a work package" and "Definition of done" in [04-milestones.md](../../04-milestones.md), the [coding standards](../../coding-standards.md) (the SSOT/DRY/ODR review gate of §1 blocks a merge, in every review and every fix round) and the [glossary](../../glossary.md).

**Goal:** AI helps as the person and never does more than the person may: an assistant that only proposes, outside AI apps that connect through MCP and act for a person, and agents that work on their own, but only on the issues a person hands over or a standing rule gives them, with every result reviewed by a person and every run undone in one step. AI is off until an administrator switches it on, fields and projects can be kept from AI, and no AI decides about people (AI-01 to AI-05, AI-10, AI-20 to AI-23, AI-30 to AI-38, COMP-08 to COMP-11, PRIV-08; ADR-0013, ADR-0029).

M7 starts from M6 (`m6-done`; API v1 frozen at `m4-done`: new routes and optional members are additions, nothing is renamed, removed or tightened). What exists:

- **The AI crates are empty.** `bb-ai` and `bb-mcp` hold one line each. The module map draws `mcp --> app & ai` and `ai -. implements ports .-> app`; nothing in the workspace calls out over HTTP yet, and `rmcp` is in the tech stack but not in `Cargo.toml`.
- **Who did it:** the event's `Actor` is `Person` or `System`; its `Channel` already has `mcp` and `assistant` (`bb-core/src/event.rs`). The `Actor` of `bb-app` (`access/mod.rs`) is a person with an optional access key that narrows them (ADR-0025, decision 8).
- **Runs:** *Change many issues* keeps a run in `change_run` with its steps and its *Stopped* state, but a run cannot be undone (ADR-0029, decision 7, builds on it).
- **Saved searches** (ADR-0034) are the base of standing rules; **notifications** (ADR-0033), **jobs** (ADR-0017), the **settings registry** (ADR-0018) and **stored secrets** under the master key (architecture §7) are there to use.
- **Checks:** ARCH-AI-TOOLS is *planned*; SEC-MATRIX names the entry point of each row (`api` only, M2-F65).
- **Words and screens:** the glossary has the words of agents (ADR-0029); UX design §4.12 (the assistant, with one picture) and §4.17 (agents, words only) describe the screens, but the hand-over window, agent work on the issue, *My agents* on the phone, *Organization settings › Connections › Agents*, setting up AI and the sign-in of an AI app are not drawn.

Decided already: ADR-0013 (one provider interface, one tool registry, proposals, the context builder, what is designed out), ADR-0029 (the two ways in, agents, hand-overs and standing rules, agent work, questions, review, runs, *My agents*, tools and limits), ADR-0025 (roles, *where it counts*, access keys only narrow, decision 8), ADR-0011 (the one Markdown writer), ADR-0034 (saved searches), ADR-0035 decision 9 (no AI tool reads working hours), architecture §7 *AI* and *MCP*, §5 *The assistant proposes, the person decides*.

M7 runs in **five waves**, then the exit visual pass and the exit waves. A wave starts when the previous one is merged to `main`. At most four builders run at once. **Wave 1 starts while M6's last package (WP-12) is still building:** none of its packages touches the sample, the load run, the speed scenarios or the exit demo of M6, and none needs them; `bugaboo.win2` joins with wave 2.

| Wave | WP | Name | Who | Folder / branch |
|------|----|------|-----|-----------------|
| 1 | [WP-01](WP-01-m7-docs.md) | Decision records, words, the security and privacy chapter, the UX gaps, the maintainer's answers | Lead | `bugaboo.win1` / `WIN1` |
| 1 | [WP-02](WP-02-ai-provider-and-setup.md) | The AI provider: one port, Anthropic and OpenAI-compatible adapters, off by default, the agreement step | Builder | `bugaboo.win3` / `WIN3` |
| 1 | [WP-03](WP-03-tools-context-and-never-sent-to-ai.md) | The one tool registry, the context builder, *Never sent to AI* and fenced text | Builder | `bugaboo.win4` / `WIN4` |
| 1 | [WP-04](WP-04-agents-and-the-agent-actor.md) | Agents and the agent actor: at most its person's rights, agent keys, the new permission, the administrators' list | Builder | `bugaboo.win5` / `WIN5` |
| 2 | [WP-05](WP-05-mcp-server-and-sign-in.md) | The MCP server and how an AI app signs in: OAuth 2.1 with PKCE, access keys, the consent step | Builder | `bugaboo.win2` / `WIN2` |
| 2 | [WP-06](WP-06-assistant-on-the-server.md) | The assistant on the server: questions, read tools, proposals and *Apply* | Builder | `bugaboo.win3` / `WIN3` |
| 2 | [WP-07](WP-07-runs-and-ai-marks.md) | Runs undone in one step, and the marks of text written with AI | Builder | `bugaboo.win4` / `WIN4` |
| 2 | [WP-08](WP-08-hand-overs-standing-rules-and-agent-work.md) | Hand-overs, standing rules and agent work on the server: attempts, questions, plans, review | Builder | `bugaboo.win5` / `WIN5` |
| 3 | [WP-09](WP-09-mcp-tools-for-agents-and-limits.md) | MCP tools for agents, the limits of an agent, standing rules at work | Builder | `bugaboo.win2` / `WIN2` |
| 3 | [WP-10](WP-10-assistant-panel-and-setting-up-ai.md) | The assistant panel and setting up AI on screen | Builder | `bugaboo.win3` / `WIN3` |
| 3 | [WP-11](WP-11-agent-work-on-the-issue.md) | Agent work on screen: the hand-over window, the issue page, the review, list rows | Builder | `bugaboo.win4` / `WIN4` |
| 3 | [WP-12](WP-12-my-agents-and-agent-pages.md) | *My agents* on computer and phone, an agent's page, standing rules, *Organization settings › Connections › Agents* | Builder | `bugaboo.win5` / `WIN5` |
| 4 | [WP-13](WP-13-sample-load-speed-and-exit-demo.md) | Sample organization part 6, agents under load, speed, the AI literacy guide, the exit demo | Builder | `bugaboo.win2` / `WIN2` |
| 5 | [WP-14](WP-14-rules-that-hand-over.md) | Standing rules that hand over, and rules with no end | Builder | `bugaboo.win5` / `WIN5` |

Progress is kept only in [`docs/STATUS.md`](../../STATUS.md). WP-01 writes the decision records, the words and the port of runs before any builder of wave 2 starts; wave 1's code packages need only what is decided already (ADR-0013, ADR-0029 and the placement below) and start while the lead writes the rest.

## Where the parts live (decided by this plan; WP-01 writes it into architecture §4 and ADR-0013)

The module map says `bb-ai` holds the provider adapters, the context builder and the tool registry. But every tool is a `bb-app` use case run as the actor, and the assistant is a `bb-app` use case too (architecture §5), and `bb-app` may not import `bb-ai` (architecture §4, rule 2). So:

- **The tool registry and the context builder live in `bb-app`** (`crates/bb-app/src/ai/`), next to the use cases they call. They do no I/O of their own.
- **`bb-ai` holds the provider adapters** (Anthropic, OpenAI-compatible) behind the port `AiProvider` of `bb-app`, and the test fake (behind the feature `testing`, like `bb-files`). `bb-server` wires the adapter.
- **`bb-mcp` turns the registry into MCP** with `rmcp` (streamable HTTP); it depends on `bb-app` only, so the edge `mcp --> ai` of the map goes (M4-F133).
- ARCH-AI-TOOLS reads `crates/bb-app/src/ai/tools/**` and `crates/bb-mcp/**`.

## The packages in short

- **WP-01 (lead):**
  - The maintainer's answers to M7-Q1…Q10, each written where it belongs.
  - A new decision record *The MCP server and how AI apps sign in* (ADR-0042): the OAuth 2.1 authorization server inside Bugaboo, PKCE, which apps may register (M7-Q4), token lifetimes and rotation, read and write scopes that only narrow, access keys as the other way in, the consent step and *Connected apps* (M7-Q3), every MCP session one run.
  - Amendments: ADR-0013 (the placement above; the settings of AI; the agreement step; what *Never sent to AI* covers in M7; names of people, M7-Q5; what is logged of a tool call, M7-Q6), ADR-0029 (M7-Q1, Q7, Q8), ADR-0025 (the permission *Create agents that work for you*, what an agent never holds), ADR-0017 (the jobs of M7), ADR-0033 (the notification kinds of M7).
  - The glossary words of M7, the M7 section of the security chapter and *Personal data in M7*.
  - The UX gaps: setting up AI with the agreement step, *Never sent to AI* on a field and a project, the consent step of an AI app and *Connected apps*, the hand-over window, agent work on the issue page, the review window, *My agents* on computer and phone, an agent's page with its keys and limits, the standing-rule window, *Organization settings › Connections › Agents*.
  - The shape of the port of runs (`RunStore`), so WP-05, WP-07 and WP-08 run side by side: WP-07 implements it, WP-05 and WP-08 test against a fake of it.
- **WP-02 (AI-01, AI-02, PRIV-08, COMP-08):** the port `AiProvider`; adapters for Anthropic and OpenAI-compatible endpoints (local models), with time limits, size limits and errors that never echo the provider; the fake; the settings of AI in the registry (off for the organization, a switch per project that follows the one above); the agreement step before an outside provider; the API key as a stored secret; *What goes where*; the security log.
- **WP-03 (AI-03, AI-04, AI-05, AI-21, COMP-10):** the one tool registry (each tool: name, *reads* or *changes*, the use case, the scope it needs, its preview); the read tools of AI-21; the write tools as declarations with their previews; the context builder (as the actor, hidden and *Never sent to AI* fields dropped, people's text fenced with who wrote it); *Never sent to AI* on fields and projects; the log of every tool call; ARCH-AI-TOOLS active; the prompt-injection test set (TEST-PROMPT-INJECTION, new).
- **WP-04 (AI-30, AI-04):** the agent as a kind of actor in `bb-policy`, judged as the intersection of its person's rights, its work and its limits, and nothing of kind *Manage*; agents with one responsible person, never a role, a group or *Assigned to*; agent keys that always run out; *Create agents that work for you*; administrators list and stop every agent and switch agents off; the event's actor names an agent and an app; the port of an agent's work (`AgentWorkStore`), which WP-08 fills.
- **WP-05 (AI-20, AI-21, AI-22, AI-23):** the MCP server on the server's own port (streamable HTTP); OAuth 2.1 with PKCE as ADR-0042 says; access keys as bearer; read and write scopes; the consent step and *Connected apps*; every call through the registry and the policy engine; the history says *"Ada Brandt via Claude Code"*; each session one run; rate limits.
- **WP-06 (AI-10, AI-05, COMP-08, COMP-09 on the server):** the assistant: a question with what the panel looks at, the context of WP-03, the provider of WP-02; read tools run, write tools come back as proposals; *Apply* runs the normal use case as the person, marked *Written with the assistant*; answers written by the one Markdown writer with no outside pictures and every link in full; nothing about people decided.
- **WP-07 (AI-38, COMP-08, COMP-09):** every MCP session and every attempt is one run on `change_run`; the list of a run's changes; *Undo the run* in one step, which skips and names what somebody changed since; the marks *Written with the assistant*, *Written by an AI app*, *Written by an agent* stored with the text, in the history, the API and every export, and shown beside the text.
- **WP-08 (AI-31, AI-32, AI-33, AI-34, AI-35 on the server):** hand-overs (brief, what the agent may do, budget, plan approval, 14 days), standing rules (a saved search, proposals, 30 days up to 90; WP-14 adds rules that hand over what they find and rules that never run out, and drops the few named fields; every new match written down), agent work with its attempts, states, questions and answers, plans, proposals of standing rules, and the review (*Accept*, *Send back*, *Throw away* through the runs); *worked on by*; the notifications and jobs; the search terms `agent:` and `agent-state:`.
- **WP-09 (AI-36, AI-37 limits, AI-05 for agents):** the MCP tools of agents (*take*, *report*, *ask*, *attach*, *give up*, *changes since*), `preview` on every write tool, no edit-lock takeover, the author mark on every text in a tool answer, the limits per agent and per key that pause an agent with a sentence, standing rules fed by *changes since*; the prompt-injection set for agents and standing rules.
- **WP-10 (AI-10, AI-02, AI-03, PRIV-08, COMP-08 on screen):** the assistant panel on computer and phone (what it looks at, where answers come from, *Never sent to AI*, proposal cards with *Apply* and *Discard* as a before and after), gone when AI is off; *Organization settings › Connections › AI* (*Set up the assistant*, the agreement step, *What goes where*); the project switch; *Never sent to AI* on the *Fields* page and in the project settings.
- **WP-11 (AI-31, AI-33, AI-34, AI-35 on screen):** the *Hand over to an agent* window; the agent-work block on the issue page with the brief, attempts, questions answered there and *Approve plan*; the review window; *worked on by*; the agent mark on list rows; the *Written by …* marks beside text and the run of an attempt.
- **WP-12 (AI-37, AI-30, AI-32 on screen):** *My agents* (*Waiting for you* first, work per agent, standing rules, *Pause*, *Stop*, *Stop all my agents*) on computer and phone; *Create an agent*, its page with agent keys and limits; the standing-rule window; *Organization settings › Connections › Agents* (every agent, *Stop*, agents off for the organization or a project).
- **WP-13 (exit; COMP-11, AI under load, PERF-01):** sample organization part 6 (AI on with the test fake, fields kept from AI, two agents of one person, hand-overs in every state, a standing rule); agents in the ten-people run; *changes since* and the agent-state search on 1 M issues; the AI literacy guide and the user docs of M7; the exit demo.
- **WP-14 (AI-32, AI-31, AI-35, AI-37; the maintainer's decision of 2026-11-09, agents with no person in the loop):** a standing rule can *Hand over what it finds* (each found issue becomes a hand-over of its agent, once, capped by the limits of the agent), the option *Change only these fields* is dropped (on an issue only a rule found an agent reads and proposes), an organization setting lets rules *Never run out* (30 days and at most 90 stay the usual), and *Accepted by itself* in *My agents* lists what an unattended loop accepted.

## Why the waves are in this order

- **Wave 1** is the ground every other package stands on, and its three code packages touch different files. The provider (WP-02) and the registry with the context builder (WP-03) are the two halves the assistant and MCP need; the agent actor (WP-04) is the one change to `bb-policy`, which every route of agents, MCP and runs judges with. The only lines two of them share are the module lines of `bb-app/src/lib.rs` and `bb-app/src/ai/mod.rs` (WP-02 makes the folder `ai/`, WP-03 adds its modules to it); the lead resolves those at the rebase. The order of merging is WP-02, WP-03, WP-04.
- **Wave 2** is the server of each way in. MCP (WP-05) needs the registry and the actor kinds; the assistant (WP-06) needs the provider and the registry; runs and marks (WP-07) need the actor in the event; hand-overs and agent work (WP-08) need the agent and fill its work. WP-05 and WP-08 use the port of runs that WP-01 writes and WP-07 implements, so the order of merging is WP-07, then WP-05, WP-06 and WP-08.
- **Wave 3** is the agents' MCP tools and the screens. WP-09 needs the MCP server and agent work; WP-10 the assistant; WP-11 and WP-12 the routes of agent work and of agents. WP-10 owns the proposal card that WP-11 also shows, and WP-11 owns the agent-state mark that WP-12 also shows; each takes the part from `main` after its owner is merged (the order of merging is WP-09, WP-10, WP-11, WP-12).
- **Wave 4** puts it all on the sample, under load and on the large dataset, and runs the demo.
- **Wave 5** (WP-14) changes what wave 2 and 3 built: it needs the refusal of changes on issues only a rule found (WP-13, M7-F120) and the rule window (WP-12), and it ends the option that refusal made empty.
- **The exit visual pass** takes pictures of every changed screen in four themes on computer and phone, split into sets with a list of what was looked at, and a reviewer looks at each against UX §1.1 and §7. Open rows of earlier milestones whose target is "the next round of pictures" are taken when their screens changed in M7, and moved on by name otherwise.
- **Exit waves** follow the exit reviews, as in M4 to M6. The lead plans them from the findings.

## File ownership

No two packages of one wave own the same file. A package that needs a file another package of its wave owns asks the lead first.

| Package | Owns |
|---------|------|
| WP-01 | `docs/adr/**`, `docs/glossary.md`, `docs/architecture/**`, `docs/02-ux-design.html`, `docs/01-feature-list.md` and `docs/04-milestones.md` (only where an answer moves a feature), the follow-up tables, the port `RunStore` in `crates/bb-app/src/ports.rs` (shape only) |
| WP-02 | `crates/bb-ai/**`, the port `AiProvider` in `bb-app`, `crates/bb-app/src/ai/mod.rs` and `crates/bb-app/src/ai/setup*`, the settings of AI in `crates/bb-core/src/settings/ai*`, `crates/bb-api/src/api/ai_setup*`, the wiring of the adapter in `bb-server`, the register entry of the HTTP client, the port offset of the fake provider in `tools/dev/ports.json` if a browser test needs one |
| WP-03 | `crates/bb-app/src/ai/tools/**`, `crates/bb-app/src/ai/context/**`, `crates/bb-app/src/ai/never_sent*`, *Never sent to AI* in the store and the API of fields and projects (`crates/bb-store/src/ai*`, `crates/bb-api/src/api/never_sent*`), the log of tool calls, its migrations, `xtask/src/checks/arch_ai_tools*`, the check and the test set of TEST-PROMPT-INJECTION, both checks from *planned* to *active* in the catalogue (WP-01 writes their planned entries first) |
| WP-04 | the actor kinds of `bb-policy` (`evaluate.rs`, `narrowing.rs`, the permission in `catalogue.rs`), the `Actor` of `bb-app` (`access/mod.rs`), the event's `Actor` in `bb-core/src/event.rs`, `crates/bb-app/src/agents/**`, `crates/bb-store/src/agents*`, `crates/bb-api/src/api/agents*`, agent keys in `bb-auth/src/access_key.rs`, `bb-app/src/auth/access_keys.rs` and `bb-api/src/api/access_keys.rs`, the port `AgentWorkStore` (shape), its migrations |
| WP-05 | `crates/bb-mcp/**`, `crates/bb-auth/src/oauth*`, `crates/bb-app/src/auth/oauth*`, `crates/bb-store/src/oauth*`, `crates/bb-api/src/api/oauth*` and the routes `/mcp` and `/.well-known/…`, the consent step in `web/app/src/features/connect/**`, *Connected apps* in `web/app/src/features/my-settings/connected-apps*`, its migrations, the `mcp` rows of SEC-MATRIX |
| WP-06 | `crates/bb-app/src/ai/assistant/**`, `crates/bb-api/src/api/assistant*`, the option of `bb-markdown` for AI answers, the `assistant` rows of SEC-MATRIX |
| WP-07 | `crates/bb-app/src/runs/**`, the run of an event and *Undo the run* in `bb-store` (`change_run` and its steps), the text mark (store, API member, history, exports), `web/app/src/shared/written-by*`, `web/app/src/features/runs/**`, its migrations, the implementation of `RunStore` |
| WP-08 | `crates/bb-app/src/agent_work/**`, `crates/bb-store/src/agent_work*`, `crates/bb-api/src/api/agent_work*`, the notification kinds and job kinds of agent work, the terms `agent:` and `agent-state:` in `bb-searchlang` and the search compiler, its migrations, the implementation of `AgentWorkStore` |
| WP-09 | `crates/bb-app/src/ai/tools/agents*`, `crates/bb-app/src/agents/limits*`, the author mark of texts in tool answers, the agent rules of edit locks, the standing rules fed by *changes since*, the agent and standing-rule cases of TEST-PROMPT-INJECTION |
| WP-10 | `web/app/src/features/assistant/**`, `web/app/src/features/settings/ai*`, the *Assistant* button of the top bar in `features/shell`, *Never sent to AI* on the *Fields* page and in the project settings, the theme rules of these parts |
| WP-11 | `web/app/src/features/agent-work/**`, the agent-work block and *worked on by* on the issue page in `features/issues`, the agent mark on list rows, the theme rules of these parts |
| WP-12 | `web/app/src/features/agents/**`, *My agents* in the sidebar and the phone's bottom bar, `web/app/src/features/settings/agents*`, the theme rules of these parts |
| WP-13 | `tools/dev/sample.json` and `bb-sample` (AI and agents), `dataset large` (agent work), the ten-people scenarios of agents, the speed scenarios of M7, the M7 exit demo, `userDocs/ai/**` (the AI literacy guide and the user docs of M7) |
| WP-14 | `agent_work/rules*.rs`, the rule members of `bb-core` and `bb-store` (migration 47), the setting `agents.rules-never-run-out`, the standing-rule window and rule cards in `web/app/src/features/agents/**`, `userDocs/agents/**` |

Migrations get their numbers from the lead, in the order of merging. Ports come from each folder's block in `tools/dev/ports.json` (ADR-0023); the MCP server answers on the server's own port, and M7 adds at most one offset, for a fake provider that answers over HTTP in browser tests (WP-02).

## Follow-ups handled in M7

| Follow-up | Package |
|-----------|---------|
| M4-F133 (the module map has edges no crate has, `server --> ai`), M5-F110 (the tests-only edges) | WP-01 writes the map; WP-02 makes the edges real or removes them, and ARCH-DEPS holds the tests-only edges |
| M4-F190 (the start of an access key written twice), M4-F195 (the earliest day of a key on a 23-hour day), M2-F93 (`KeyProjectRef` renamed to a neutral name) | WP-04 (it changes access keys) |
| M2-F64 (ARCH-AUTHZ-ONLY and the person's ID apart from the actor) | WP-04 (the agent actor needs the person apart from the actor) |
| M3-F121 (an access key cannot start a change to more than 1,000 issues) | WP-04 decides it for agent keys, WP-07 for runs; see [follow-ups.md](follow-ups.md) |
| M2-F61 (ARCH-API-OUT and a second entry point that answers about the caller) | WP-05 |
| M2-F65, the rest (the columns of the assistant and of AI apps in SEC-MATRIX) | WP-05 (`mcp`), WP-06 (`assistant`), WP-04 (the actor kinds) |
| M6-F35 (the small leftovers of the busy write in changes to many issues) | WP-07 (it changes runs) |
| M5-F21, M5-F49 and the other "next package that changes …" rows | the M7 package that touches those files next; else the exit wave |

## Checks switched on or grown in M7

- **ARCH-AI-TOOLS** (planned → active, WP-03): every tool is in the one registry, declares *reads* or *changes*, names its use case and scope, and has a preview test when it changes something; no tool reaches a use case of working hours, of other people's time entries, of roles or of who is *Assigned to* by AI decision (COMP-10, ADR-0035 decision 9); `bb-mcp` and the assistant call tools only through the registry.
- **TEST-PROMPT-INJECTION** (new, planned → active, WP-03; grows in WP-06 and WP-09): the prompt-injection test set exists by name and runs: issue text, comments, field values, file names and tool answers that carry instructions, for the assistant, MCP, agents and standing rules; with the fake provider it proves that nothing changes without *Apply* or a granted scope, no outside picture or hidden link is written, fenced text stays fenced, and no field kept from AI is sent.
- **SEC-MATRIX** grows (WP-04, WP-05, WP-06, WP-08): the actor kinds *person via app* and *agent* on every route; the entry points `mcp` and `assistant`; an agent never does more than its person.
- **ARCH-PERMS** grows (WP-04): *Create agents that work for you*; what an agent never holds.
- **ARCH-API-OUT** grows (WP-05): MCP answers through the same safe types (M2-F61).
- **ARCH-SETTINGS** grows (WP-02, WP-08): the settings of AI and of agents.
- **DATA-MARKS** grows (WP-02, WP-03, WP-04, WP-05, WP-07, WP-08): every new table marked; agent work, briefs and questions as people's text; OAuth grants and agent keys as secrets.
- **DATA-LOGS** grows (WP-03): the log of a tool call holds who, which tool, size and time, never the content.
- **SEC-RICHTEXT** grows (WP-06): AI answers through the one Markdown writer, with no outside pictures and links written in full.
- **ARCH-MAP-SYNC** and **ARCH-DEPS** grow (WP-01, WP-02): the map of the placement above; the tests-only edges (M5-F110).
- **PERF-N1** grows (WP-04, WP-05, WP-07, WP-08): every new list route counts its statements.
- **PERF-BUNDLE** (WP-10, WP-11, WP-12): the assistant panel, agent work and *My agents* are lazy areas with their own message parts (ADR-0028); the first page stays within its budget.
- **PERF-BENCH** grows (WP-13): *changes since*, the agent-state search and *My agents* on 1 M issues.
- **TEST-CONCURRENT** grows (WP-13): "two agents take the same issue at the same moment" and "an agent works while its person loses access" on both databases.
- **TEST-SAMPLE-DATA** grows (WP-13): the agents, hand-overs and standing rules of the sample hold the rules of ADR-0029.

## Reviews

Every package gets a reviewer with a fresh context that applies the **SSOT/DRY/ODR merge gate of coding-standards §1** to the diff and the code it touches; a finding there blocks the merge, and every fix round gets its own gate check by a fresh reviewer, never by the lead or the builder who wrote it (04-milestones, "Life of a work package"). On top:

- **Security and privacy review:** every package of M7 (AI, permissions, sign-in and personal data, 04-milestones step 5), with COMP-10 read on every tool, rule and screen: nothing ranks, evaluates or picks people.
- **UX review from pictures** (four themes × computer and phone): WP-05 (the consent step, *Connected apps*), WP-07 (the marks, the run page), WP-10, WP-11, WP-12, WP-13.
- **Words review:** every package with new messages (glossary words only, plain language for people who are not technical; the AI is always called the assistant, an AI app or an agent, never by a vendor's word).

## The exit demo

The demo is written once, in [04-milestones](../../04-milestones.md) (M7, *Demo (exit)*). Each step is a test (`web/app/e2e/m7-exit-demo.spec.ts`) with an MCP client in the test and the fake provider, with pictures in the four themes on computer and phone. How the maintainer tries it with a real AI app and a real provider (M7-Q9) is written in [architecture §11](../../architecture/11-testing.md#the-m7-exit-demo-on-the-sample): the review page shows how to connect the real Claude Code to `cargo xtask dev` with one command, and how to set up the assistant with the maintainer's own Anthropic key or a local model; the sample starts with the fake provider, so nothing leaves the computer until the maintainer changes it.

## Questions for the maintainer

Each question has the lead's recommendation. Accepting all of them takes every recommendation as written. WP-01 writes each answer where it belongs.

**Decided (2026-11-07): the maintainer accepted every recommendation below.**

- **M7-Q1: Who may create agents.** ADR-0029 makes *Create agents that work for you* an organization permission that *Project lead* and *Team member* hold at the start. But an organization permission counts only from a role given for the whole organization (ADR-0025, decision 1), and those two roles are given on projects, so nearly nobody would have it.
  - **Recommendation:** a **project** permission (kind *Change*), held at the start by *Administrator*, *Project lead* and *Team member*. A person who holds it in at least one project may create agents. An agent may be handed issues, or given a standing rule, only in the projects where its person holds it. A project lead thus decides where agents work.
- **M7-Q2: What the assistant does in M7.** The demo has the assistant summarize an issue, and UX §4.12 draws *Summarize comments*, *Write it up for me* and *Suggest fields*. Those are AI-12, AI-13 and AI-14, which are 1.0 features (M12).
  - **Recommendation:** in M7 the panel answers what the person types about what it looks at (the issue, the list, the project), using the read tools, and proposes changes with the write tools; "Summarize the comments on WEB-142" works as a typed question. The buttons of AI-12, AI-13 and AI-14, the catch-up and the suggestions beside the fields stay in M12.
- **M7-Q3: Taking back an AI app's access.** Signing in with OAuth needs a consent step, and a person must be able to take an app's access back. *Connected apps* is AI-24, a 1.0 feature.
  - **Recommendation:** M7 builds the consent step (*"Claude Code wants to work with your account: See and change"*, with the scopes) and a plain *Connected apps* list under *My settings › Sign-in and security* with *Take back*. AI-24 keeps what is more: the consent remembered per app and the details of each app's use.
- **M7-Q4: Which AI apps may sign in.** Architecture §7 plans open registration of apps with an allow list kept by administrators.
  - **Recommendation:** an app whose return address is on the person's own computer (as Claude Code and Claude Desktop use) may register by itself; an app on a web address needs an administrator to add its address under *Organization settings › Connections › AI* first. Administrators can switch AI apps off for the organization.
- **M7-Q5: Names of people sent to the AI provider.** The context holds people as the person sees them (*Who sees people*). Names are personal data that then leave the server.
  - **Recommendation:** at the start, people go to an outside provider as their role label (*"a Team member"*, *"you"*), never by name or e-mail. An administrator may switch on *Send names of people* in *Set up the assistant*, and *What goes where* says so. A local model gets names as the person sees them. MCP is not affected: the app acts as the person and reads what the person reads.
- **M7-Q6: Keeping the assistant's conversations.**
  - **Recommendation:** a conversation lives in the open panel only and is not stored on the server; closing the panel ends it. Each tool call is logged with who, which tool, the size and the time, never the content (architecture §7). Content logging for troubleshooting is not built in M7.
- **M7-Q7: An agent's time.** A hand-over always lets the agent "log its time" (ADR-0029, decision 3), but time entries are a person's working data (ADR-0035, PRIV-09).
  - **Recommendation:** the time and cost an agent reports stay on its attempt, marked as reported by the agent. An agent never makes a time entry, and no total of *Time spent* counts it. An AI app acting for a person may book the person's own time (AI-21 `log_time`), with *Book time*.
- **M7-Q8: What "its checks pass" means.** A project can allow *Accept by itself when its checks pass* (ADR-0029, decision 6), but nothing says which checks.
  - **Recommendation:** an agent attaches a result of the kind *Check* with *passed* or *failed* (`attach`). An attempt is accepted by itself only when the project allows it, the hand-over did not turn it off, it has at least one check, and every check of the attempt passed. Bugaboo does not run or verify outside checks, and the window says so. The status after such an accept is the one the workflow proposes.
- **M7-Q9: How the maintainer tries M7.** The automated demo uses a fake provider and an MCP client inside the test.
  - **Recommendation:** the review page shows how to connect the real Claude Code to `cargo xtask dev` (one command), and how to set up the assistant with either the maintainer's own Anthropic key (typed into *Set up the assistant*, stored as a secret in the local data folder, never in Git) or a local model through the OpenAI-compatible choice. The sample starts with AI on and the fake provider, so nothing leaves the computer unless the maintainer changes it.
- **M7-Q10: What stays out of M7.**
  - Usage limits and cost budgets per person and organization (AI-06) stay in M12; M7 has only the limits of agents (AI-37) and of keys.
  - The rest of *Connected apps* (AI-24) stays in M12 (M7-Q3).
  - *Never sent to AI* on restricted issue groups waits for restricted issues (SEC-05, 1.0); M7 covers fields and projects.
  - Agents of the organization (AUTH-08) stay in M12.
  - The assistant's buttons and suggestions (AI-11 to AI-14) stay in M12 (M7-Q2); AI-15 and AI-16 stay later.
  - Bugaboo runs no agents (ADR-0029).
