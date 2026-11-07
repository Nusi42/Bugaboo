# WP-04 — Agents and the agent actor

**Wave 1 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** AI-30, AI-04 (the actor kinds). ADR-0029 decisions 1, 2 and 9 (the limits as data), ADR-0025 decisions 1 and 8, ADR-0022 (*Who sees people*); follow-ups M2-F64, M2-F93, M3-F121, M4-F190, M4-F195.

## Goal

An agent is an account of its own that is not a person. It has one responsible person and can never do more than that person can, narrowed by its work and its limits. It holds no role, is in no group and is never *Assigned to* an issue. When its person loses access, the agent loses it at the next request. Administrators see every agent and can stop any of them. Every change in Bugaboo says which person, which agent and which app made it.

## Owns

The actor kinds of `bb-policy` (`evaluate.rs`, `narrowing.rs`, the new permission in `catalogue.rs`), the `Actor` of `bb-app` (`access/mod.rs`), the event's `Actor` in `bb-core/src/event.rs`, `crates/bb-app/src/agents/**`, `crates/bb-store/src/agents*`, `crates/bb-api/src/api/agents*`, agent keys in `bb-auth/src/access_key.rs`, `bb-app/src/auth/access_keys.rs` and `bb-api/src/api/access_keys.rs`, the port `AgentWorkStore` (shape and an empty implementation), its migrations (numbers from the lead).

## What is decided (ADR-0029, M7-Q1 as recommended)

- **Three actor kinds:** a *person* (a session, or an access key that narrows them), a *person via app* (an OAuth grant or a key used by an AI app: the person, narrowed by the scope, with the app's name), an *agent* (its responsible person's evaluation, intersected with its work and its limits). One type in `bb-policy` carries the kind, so a use case cannot judge an agent as its person by mistake.
- **An agent's rights** at a place are the intersection of: what its person may do there; what its work allows there (a hand-over's issues and ticks, a standing rule's search and fields, read through the port `AgentWorkStore`, which WP-08 fills; until then an agent has no work and may do nothing but read its own record); and never a permission of kind *Manage* or *Take over text someone else is editing* (`NOT_FOR_AGENTS`, next to `NOT_FOR_KEYS`).
- **An agent record:** a name (unique per person), its responsible person, a state (*Active*, *Paused*, *Stopped*), its limits (changes a minute, issues at once, attempts on one issue; defaults and the organization's maximum as settings), made by, made at, a version. Handing an agent to another person needs that person's acceptance.
- ***Create agents that work for you*** (M7-Q1): a project permission of kind *Change*, held at the start by *Administrator*, *Project lead* and *Team member*. A person who holds it in at least one project may create agents; an agent's work counts only in projects where its person holds it.
- **Agent keys:** access keys that belong to an agent, always run out (at most the organization's longest key life), are listed with the agent and can be renewed or taken back there. The secret is shown once. A key of an agent can never start a change to more than 1,000 issues (M3-F121: the agent's limits, not a background job, decide).
- **Never a member:** no row of *Who has which role* names an agent, no group holds one, and *Assigned to*, *Followers* and *Reported by* never hold one (the store refuses it). The issue shows *worked on by* (WP-08).
- **Shown as** *"Fixer, agent of Ada Brandt"*: the agent's name always, its person through *Who sees people* for the viewer.
- **Stopping:** the person, an administrator (*Stop*), *Stop all my agents* (WP-12 draws it; the route is here), the person losing access, being switched off or leaving: the agent's next request is refused, and its open work becomes *Stopped* (WP-08 reacts to the event).
- **Agents off:** an organization setting and a project setting (follows the one above), like AI itself.
- **The event's actor** names a person, an agent (with its person) or the system, and its `Channel` names the app when there is one; the history writes *"Ada Brandt via Claude Code"* and *"Fixer, agent of Ada Brandt"*.

## Tasks

1. **`bb-policy`:** the actor kinds, the intersection for an agent, `NOT_FOR_AGENTS`, the new permission with its sentence, kind and starting values; property tests that an agent's rights are a subset of its person's at every place.
2. **Agents:** tables, use cases and routes (`/api/v1/agents`: create, list one's own, read, rename, pause, resume, stop, hand to another person and accept; `/api/v1/organization/agents`: every agent for administrators, *Stop*; `/api/v1/me/agents/stop-all`). Each with its permission, version and event; the security log for stops by an administrator.
3. **Agent keys:** made, renewed and taken back on the agent; the request that comes with one becomes an agent actor. While here: the start of a key written once (M4-F190), the earliest day of a key on a 23-hour day (M4-F195), `KeyProjectRef` renamed to `ProjectLabelBody` (M2-F93).
4. **The actor in `bb-app`:** the kinds, and the person apart from the actor for an agent, so ARCH-AUTHZ-ONLY can tell them (M2-F64).
5. **The event:** the agent and the app in `Actor` and `Channel`, the history's sentences in the shared wording.
6. **The port `AgentWorkStore`:** the work of an agent at a place (issues, ticks, fields of a standing rule), with an implementation that has none, so WP-08 only fills it.

## Acceptance tests (write first)

- `ai_30_an_agent_needs_a_responsible_person`.
- `ai_30_an_agent_never_does_more_than_its_person` (property test over places and permissions).
- `ai_30_an_agent_stops_when_its_person_loses_access` (the next request is refused).
- `ai_30_an_agent_cannot_be_assigned` and `ai_30_an_agent_has_no_role`.
- `ai_30_an_agent_never_holds_a_manage_permission`.
- `ai_30_an_administrator_stops_any_agent`.
- `ai_30_an_agent_key_always_runs_out_and_is_shown_once`.
- `ai_30_without_work_an_agent_reads_only_its_own_record`.
- `ai_30_agents_off_for_a_project_refuses_its_agents_there`.
- `ai_04_the_history_says_person_via_app_and_agent_of_person`.
- `ai_30_an_agent_is_shown_with_its_person_as_who_sees_people_says`.
- SEC-MATRIX: the actor kinds *person via app* and *agent* on every route (an agent with no work is refused everywhere but its own record), the routes of agents, both databases.

## Checks

SEC-MATRIX, ARCH-PERMS, ARCH-AUTHZ-ONLY, ARCH-PERSON-REF, ARCH-EVENTS, ARCH-VERSIONED, ARCH-API-FREEZE, DATA-MARKS, PERF-N1, ARCH-DB-PARITY.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one type of actor, one intersection, `NOT_FOR_AGENTS` written once next to `NOT_FOR_KEYS`, agent keys through the one code of access keys. A security reviewer reads every way an agent could get more than its person, and every place an agent could appear as a person.

## Depends on

Nothing of M7. Runs beside WP-02 and WP-03; merges last in wave 1.
