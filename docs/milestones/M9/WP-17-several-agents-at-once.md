# WP-17 — Several agents at once, and agents asked for by a program

**Wave 3 · Folder / branch:** `bugaboo.win5` / `WIN5` · **Model:** Opus
**Features:** AI-30 (agents), AI-37 (*My agents*), AI-20 and AI-22 (the MCP server and how apps sign in), AUTH-07 (access keys). [ADR-0029](../../adr/0029-ai-agents.md), amendment M9 (several agents and requests); [ADR-0042](../../adr/0042-mcp-server-and-how-ai-apps-sign-in.md); [architecture §12](../../architecture/12-security.md) (*Making a key asks for the password again*).

## Goal

A person who runs a team of agents, one agent and one key for each job, makes them in one step. Today every agent key needs the person's password, typed in the browser, one key at a time, and no program can ask for agents. After this package a person creates several agents at once on *My agents*, each with its key, and enters the password once; and a program (an AI app or a script) asks for the agents it needs with a reason, the person approves the request with the password once, and the program collects the new keys once. Each job stays its own agent with its own key, for the history, pausing and limits: one key shared under many names is not this package.

## Owns

- `crates/bb-core/src/agent.rs` (the named limits of this package) and `crates/bb-core/src/agent_request.rs` (new: the request, its entries, its states and its periods).
- Migration 61 (`m0061_agent_requests.rs`: the tables `agent_requests` and `agent_request_entries`). Migration 59 is WP-08's on this branch and 60 is WP-06's (the removal of a person), which lands first; WP-16 also planned 59 and takes the next free number when it rebases (whoever lands later renumbers, as for every shared migration number).
- `crates/bb-app/src/agents/several.rs` (new: the one planned change that makes several agents and their keys), with what it takes out of `manage.rs` (`create_agent`) and `crates/bb-app/src/auth/access_keys.rs` (`create_access_key`), so the single use cases and the new ones run the same code.
- `crates/bb-app/src/agents/requests.rs` (new: ask, list, approve, decline, collect) and the store port and its two databases in `crates/bb-store/src/agent_requests.rs` (new).
- The tools `ask_for_agents` and `collect_agent_keys`: `bb_core::ai::Tool`, `crates/bb-app/src/ai/tools/` (`inputs.rs`, `registry.rs`, a new `programs.rs`).
- The routes `POST /api/v1/agents/several`, `GET /api/v1/agents/requests`, `POST /api/v1/agents/requests/approve` and `POST /api/v1/agents/requests/{request}/decline` (`crates/bb-api/src/api/agents.rs` or a new `agent_requests.rs`).
- The notification *A program asks for agents* (`NotificationKind::AgentsAskedFor`) and its two settings.
- The keep rule *Requests for agents* in `KEEP_KINDS` and the daily delete.
- The screens: `web/app/src/features/agents/` (*Create several agents*, the keys shown once, *Asked for by a program* with *Approve* and *Decline*), words in `messages/en.json`, theme rules in `design/theme.kdl` only if a part is new.
- Docs: the ADR-0029 amendment M9 (several agents and requests), architecture §12 (rows of the threat table and of personal data), the glossary, `userDocs/agents/`.

## What is decided

### 1. Create several agents (the person)

- **Where:** *My agents*, the button *Create several agents* beside *Create an agent*. The window takes a list of rows; each row is an agent with the same choices as making one agent and its first key: the name of the agent, and when its key runs out (the organization's default when left empty, at most its longest time). The key is called after its agent. The limits of each agent start at the organization's starting limits, as for one agent, and are changed on its page.
- **The password once** for the whole list, asked as for a single key (architecture §12: *making a key asks for the password again*). Nothing else of a key changes: an agent key is the agent's, does what its work allows (`KeyChoice::SeeAndChange`, cut by `Narrowing::agent`) and always runs out.
- **One change, all or nothing.** Every agent and its key are written in one change. When one row fails (a name used twice in the list or already used by the person, an end the organization does not allow, more agents than `MOST_AGENTS` or than `MOST_AGENTS_AT_ONCE` in one list, a person who holds *Create agents that work for you* in no project), nothing is made and the answer names the row (its place in the list) and why.
- **Reuse, no copy.** `create_agent` and `create_access_key` are split into a plan (check the input, build the record and its event) and the step inside the change (`insert_agent`, `insert_key`, which count against `MOST_AGENTS` and `MOST_AGENT_KEYS` under the lock of the person's agents). The single use cases run the plan for one, the new use case `create_several_agents` for many, in one `record_changes` with every event. There is one rule of names, one of limits, one of ends and one of counting.
- **The keys are shown once**, all together, in the order of the rows, with one button that copies all of them (each line `name: key`) and one per key, as when one key is made; leaving the window forgets them.
- **The record.** Each agent has its event (as today), each key its event and its own entry in the security log (*Access key created*, as today).

### 2. Agents asked for by a program

- **Ask (`ask_for_agents`).** A new MCP tool that changes something (with `preview`). It takes a list of agents (each a name, what it is for (*purpose*: a few words, not a role of permissions), and how many days its key should last, at most the organization's longest time) and a short reason, and creates one **request for agents**. Nothing else is made: no agent, no key. The answer names the request and says that the person must approve it in the browser, and how long it waits.
- **Who may ask.** A caller of kind *AI app* (`ToolCaller::AiApp`): an access key of the person whose choice may change things (*See and change*, or *More options* with *Create agents that work for you*), or the grant of an AI app with *See and change*. In both cases the narrowed person must hold *Create agents that work for you* in some project (`Need::Holds(Permission::CreateAgents)`). **Not** an agent (an agent never creates an agent, ADR-0029 decision 3; `CreateAgents` is in `NOT_FOR_AGENTS`, and an agent key or an agent's grant is refused by kind as well), **not** a key that can only look (*See only*), and **not** the assistant (it only proposes in the panel of a person who can use the screen, and it has no credential a pickup could be bound to). Why: the asking credential is already able to change things in the person's name, so asking adds no power; a look-only key must stay unable to cause anything; and an agent asking for agents would let one agent widen the team it is part of.
- **Bounded.** A request holds at most `MOST_AGENTS_AT_ONCE` agents, the reason at most the length of every reason (the one helper of reasons), the names and what each is for the rules of a name. One key or grant may have at most `MOST_WAITING_REQUESTS` (3) requests waiting at once; one more is refused with `request.too-many`. The person's `MOST_AGENTS` is judged again when the request is approved.
- **The person sees it.** *My agents* shows *Asked for by a program* on top, with what waits for the person: per request the app's name or the key's name, the reason, the agents with what each is for, and when it runs out. The person gets the notification *A program asks for agents* (in the app; by e-mail if they switch it on).
- **Approve or decline.** The person ticks one or several requests. *Approve* asks for the password once and runs feature 1's planned change for every agent of the ticked requests, in one change, all or nothing, and marks each request approved, with its agents. **The keys are not made then:** the approval answers the agents, not keys, so no key is shown to the person or kept anywhere. *Decline* needs the session only (it takes away, never adds) and ends the request.
- **Collect once (`collect_agent_keys`).** The program asks with the request's ID. Only the **same credential** that asked may collect (the same access key, or the same grant of the app; a key renewed, taken back or run out, or a grant taken back, can never collect). The first call after the approval makes one key for each agent of the request through the same planned change of keys (each with its event and its security-log entry), marks the request collected in the same change, and answers the keys, once. Before the approval it answers *waiting*; after a decline *declined*; after its time *ran out*; a second call *collected*, with no key. A preview says what a call would do and makes nothing.
- **Why keys are made only at the pickup.** A key made at the approval would have to wait somewhere until the program collects it, readable by whoever reads the database and the key file. Made at the pickup, no secret of a key is ever stored: only its hash, as for every key.
- **Time.** A request waits `REQUEST_HOURS` (24) for an answer; an approved request waits another `REQUEST_HOURS` to be collected, after which its agents stay without keys (the person can make keys on their pages). A request is deleted by itself `REQUEST_KEEP_DAYS` (7) after it was asked, a row of the keep table (*Requests for agents*, deleted by itself).
- **The security line.** A key or a grant may **ask**; it can never make a key by itself. Every key still comes from the person's password: typed for one key, for a list, or for an approval, and a collected key is one the person approved for an agent the approval made, once, for the credential that asked. Written into ADR-0029 (amendment M9) and architecture §12.

### 3. Security and defaults

- SEC-MATRIX: rows for the four new routes and the two tools, for every starting role and every kind of key (a session, a person's key *See only* and *See and change*, an app's grant, an agent's key), on both databases. The routes take a session only; a key of any kind is refused.
- Nothing starts on: agents are off in a new installation (WP-08), and the request needs agents on, as every agent does.
- The security log has an entry for every agent key made (as today), for each request asked, approved, declined and collected, and for a collect refused to another credential.

## Tasks

1. Write the acceptance tests below.
2. The plan of agents and keys taken out of `create_agent` and `create_access_key`; `create_several_agents`; its route.
3. Migration 61, the store on both databases; the request, its use cases, the routes; the notification and its settings; the keep rule.
4. The tools `ask_for_agents` and `collect_agent_keys`.
5. The screens: *Create several agents*, the keys shown once, *Asked for by a program*; their pictures in four themes on computer and phone (when the browser tests may run).
6. SEC-MATRIX examples; the user docs.

## Acceptance tests (write first)

- `ai_30_several_agents_are_made_with_one_key_each_and_the_password_once`.
- `ai_30_several_agents_are_all_or_nothing` (a name twice, a taken name, a wrong end: nothing made, the row named).
- `ai_30_several_agents_stay_within_the_most_agents_and_the_most_at_once`.
- `ai_30_several_agents_need_the_password_and_a_session`.
- `ai_30_each_agent_and_key_of_several_has_its_event_and_its_security_log_entry`.
- `ai_30_one_agent_and_one_key_are_made_by_the_same_plan_as_several` (the single routes answer as before).
- `ai_30_a_program_asks_for_agents_and_nothing_is_made`.
- `ai_30_only_a_key_or_grant_that_may_change_and_create_agents_asks`.
- `ai_30_an_agent_never_asks_for_agents`.
- `ai_30_a_key_or_grant_has_at_most_three_requests_waiting`.
- `ai_30_the_person_is_told_and_sees_the_waiting_requests`.
- `ai_30_approving_requests_makes_their_agents_in_one_change_with_the_password_once`.
- `ai_30_approving_shows_no_key_to_the_person`.
- `ai_30_a_declined_request_makes_nothing`.
- `ai_30_only_the_credential_that_asked_collects_the_keys_once`.
- `ai_30_a_request_runs_out_after_a_day_and_its_keys_cannot_be_collected_after_a_day`.
- `ai_30_a_key_never_makes_a_key_by_itself` (no path from a key or a grant to a key without the person's password: the approval, the single and the several routes refuse every key).
- `priv_05_requests_for_agents_are_deleted_after_their_keep_period`.
- Store, both databases: `agent_requests_are_stored_and_read_back`, `the_waiting_requests_of_a_credential_are_counted`.
- MCP: `ai_21_ask_for_agents_previews_and_saves`, `ai_21_collect_agent_keys_previews_without_making_a_key`.
- SEC-MATRIX: rows for the four routes and the two tools, both databases.
- Browser (later, when the lead says, `--workers=2`): `ai_37 several agents are created at once and their keys copied`, `ai_37 a request of a program is approved and declined` (four themes, computer and phone).

## Checks

ARCH-AI-TOOLS (both tools declare that they change something and have a preview test), ARCH-PERMS, ARCH-EVENTS, ARCH-AUTHZ-ONLY, ARCH-API-OUT, DATA-MARKS, DATA-MIGRATIONS, SEC-MATRIX, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, THEME-ONLY, PERF-N1, TEST-A11Y. No new check: the rules above are held by these and the named tests.

## Review

Code review with the SSOT/DRY/ODR gate: one plan of agents and keys for one and for many, one rule of names, limits, ends and counts. **Security review:** the security line (no key from a key without the password), the binding of the pickup to the credential that asked, a pickup that leaks a key to someone else, requests that flood a person, and a declined or run-out request that still makes something. **Privacy review:** the request's text and its keep period. **UX review** of the pictures.

## Depends on

WP-08 (`c54d9ea54`, AI apps and agents off in a new installation), on its way to `WIN1`. Runs beside WP-16 (`win4`), which also amends ADR-0029 and the glossary: each keeps its own section and rows.
