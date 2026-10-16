# ADR-0029: AI agents: apps that act for a person, and agents that work on their own

Status: **Accepted** · 2026-10-16 · refines [ADR-0013](0013-ai.md) and [ADR-0025](0025-roles-groups-and-assignments.md)

## Context

M8 plans the MCP server (AI-20…AI-23): an outside AI app signs in as a person and does what that person may do (AI-04). Decision: this stays, and agents should also be able to work **on their own**, with a person's permission, on chosen issues. Bugaboo does not run agents, now or in the foreseeable future: they run outside (coding agents in a developer's tools, a company's own agents) and connect through MCP.

An agent works on an issue differently from a person:

| | Person | Agent |
|---|---|---|
| The issue | A reminder and a conversation; much of the context is in people's heads | The whole job: it knows nothing that is not written there or linked from it |
| Memory | Remembers yesterday | Starts fresh each time; what it learned must be stored |
| How work goes | Steady, with a comment now and then | In attempts: plan, work, result or failure, try again; several at once |
| When stuck | Asks someone | Asks through the issue and waits, or guesses |
| Done | "Looks fine to me" | Checkable: tests pass, what *Done when* says is true |
| What it leaves | A comment, a new status | A branch, a pull request, a test report, a log, its time and cost |
| Trust | Earned over years | Every result is reviewed by a person |
| Pace | One issue at a time | Hundreds of reads and changes in minutes, often unattended |

One person may also drive several agents at the same time, each on its own issues.

## Decision

### 1. Two ways in, one policy engine

- **An AI app acting for a person** (AI-04, AI-22, AI-23, unchanged): the app signs in as the person through OAuth or an access key, does at most what the person may do, narrowed by the key's choice, and the history says *"Ada Brandt via Claude Code"*. This is the way for "give all my open issues in Website to Lea".
- **An agent working on its own** (AI-30): an account of its own that is not a person. It can do nothing until a person gives it work (decision 3).
- Both go through `bb-policy` and the one tool registry of ADR-0013; there is no second rule set. Enforced by: ARCH-AI-TOOLS and SEC-MATRIX (rows for the actor kinds *person via app* and *agent*).

### 2. Agents

- **Every agent has one responsible person**, normally the person who made it; handing an agent to another person needs that person's acceptance. Making agents takes the new permission *Make agents that work for you* (organization, kind *Change*; at the start Administrator, Project lead and Team member have it). Administrators see every agent of the organization in *Organization settings › Agents* and can stop any of them; an administrator can switch agents off for the organization or a project, like AI itself (AI-02). Enforced by: the tests `ai_30_an_agent_needs_a_responsible_person` and `ai_30_an_administrator_stops_any_agent` (M8).
- **What an agent can do at a place is at most what its responsible person can do there**, narrowed by its work (decision 3) and its limits (decision 9). When the person loses that access, is switched off or leaves, the agent loses it at the next request, and its open work becomes *Stopped*. Enforced by: the tests `ai_30_an_agent_never_does_more_than_its_person` and `ai_30_an_agent_stops_when_its_person_loses_access` (M8).
- **An agent is not a member:** it has no role, is in no group, never appears in *Who has which role*, and is never *Assigned to* an issue. The issue keeps its person; while an agent works on it the issue shows *"worked on by Fixer"*. This keeps a person answerable for every issue and keeps COMP-10 simple. Enforced by: the tests `ai_30_an_agent_cannot_be_assigned` and `ai_30_an_agent_has_no_role` (M8).
- **Shown as** its name with an agent mark and its person: *"Fixer, agent of Ada Brandt"*. Its own name is always shown (it is not a person); its person follows *Who sees people* for the viewer (*"agent of a Team member"*).
- An agent signs in with its own **agent key** (an access key that belongs to the agent and always runs out) or with OAuth for its app. Agent keys are listed with the agent and can be renewed or taken back there.
- **One person, several agents:** a person may have several agents, and each agent may work on several issues at once, within its limits. *My agents* (decision 8) shows all of them together, with what waits for the person first.

### 3. Giving an agent work: hand over, or a standing rule

There are two ways, and no third one (a project-wide permission would be a standing rule without a filter).

- **Hand over** (AI-31): a person hands one or several issues, by their IDs, to one of their agents in the *Hand over to an agent* window. They choose:
  - the agent;
  - **the brief**: taken from the issue (description, *Steps to reproduce*, *Done when*), plus a note; the window says what is missing (*"No steps to reproduce"*) but does not stop the hand-over;
  - **what the agent may do in Bugaboo**: always read the issue and what it links to, comment, report its work, log its time, attach results and link related issues; ticked on request: change named fields, change the status, make sub-issues;
  - **a budget**: attempts, time, and the cost the agent reports;
  - **whether its plan needs approval** before it starts.

  The hand-over ends at *Accept*, *Throw away*, *Stop*, or when it runs out (14 days unless set otherwise). The person handing over needs *Change issues* (or *Change the issues they reported*) on each issue. Enforced by: the tests `ai_31_a_handed_over_agent_sees_only_its_issues` and `ai_31_a_hand_over_ends_at_accept` (M8).
- **Standing rule** (AI-32): a person lets one of their agents work on everything a saved search finds, for example *"every new bug in Website"*. Because anybody who changes an issue so that it matches gives the agent that issue, a standing rule is narrower than a hand-over:
  - it may **propose** (default: nothing changes until a person clicks *Apply*, as with the assistant), or **change only the fields named in the rule** from a short list (*Labels, Tags, Type, Duplicate of, Found in release*); it may create issues in one named project when the rule says so (for an agent that reports failing tests);
  - it never changes a field that holds people (*Assigned to*, *Followers*, *Reported by*) and never the status; Enforced by: the test `ai_32_a_standing_rule_never_changes_people_fields` (M8), and COMP-10 in review;
  - it **runs out** after 30 days unless set otherwise (at most 90) and is renewed with one click;
  - every issue that newly comes under the rule is written down, so the person sees what the agent saw.
- **Never given to an agent:** the permissions of kind *Manage* (people, roles, settings, fields) and *Take over text someone else is editing*. Enforced by: the test `ai_30_an_agent_never_holds_a_manage_permission` (M8).

### 4. Agent work: the record of what the agent does on an issue

- **Agent work** (AI-33) is a record of its own on the issue, apart from the comments, so the conversation stays readable for people. It holds the brief, the **attempts** (each with the agent's plan, its steps, its results as links such as a branch, a pull request or a test report, and the time and cost the agent reports, marked as reported by the agent), the questions and their answers, and the review.
- **States**, shown on the issue page, as a mark on list rows (the row keeps its status color) and searchable (`agent:Fixer`, `agent-state:waiting-for-you`): *Waiting for the agent*, *Agent working*, *Waiting for you* (a question or a plan to approve), *Ready for review*, *Agent gave up*, *Stopped*.
- When an attempt is ready, one short comment marked *Written by an agent* says so and links to the agent work.
- The issue's own status is not an agent state. It moves only through the review (decision 6).

### 5. Questions back, instead of guessing

- An agent that is stuck asks (MCP tool `ask`). Its work waits as *Waiting for you*; the responsible person and the person who handed it over get a notification (NTF-01). The answer is stored in the brief, so the next attempt and any other agent read it. Enforced by: the test `ai_34_a_question_waits_for_an_answer_and_the_answer_joins_the_brief` (M8).
- With *Plan needs approval*, the first step of each attempt is the plan, and the agent waits for *Approve plan* or a note.

### 6. Review

- At *Ready for review* a person chooses **Accept** (the hand-over ends; the status moves to the one the reviewer picks, proposed by the workflow), **Send back** with a note (a new attempt with the note in the brief) or **Throw away** (everything the attempts changed in Bugaboo is undone in one step; results outside Bugaboo, such as a branch, are the agent's business, and the window says so).
- A project can allow **Accept by itself when its checks pass** for handed-over issues (off at the start). Issues accepted that way are listed under *Accepted by itself* in *My agents* until a person has looked at them. Enforced by: the test `ai_35_accept_by_itself_is_off_at_the_start` (M8).

### 7. Runs: everything an app or an agent did, undone together

- Every session of an AI app acting for a person, and every attempt of an agent, is **one run** (AI-38), built on the store of changes to many issues (`bulk_run`, its events and its *Stopped* state). A run is shown as the list of its changes and can be **undone in one step**; a change that somebody changed again afterwards is skipped and named, as in *Change many issues*. Enforced by: the test `ai_38_a_run_is_undone_in_one_step_and_skips_later_changes` (M8).
- Text written through MCP is marked *Written by an AI app* or *Written by an agent*, stored with the text like *Written with the assistant* (COMP-08, COMP-09).

### 8. My agents: one person, several agents

- **My agents** (AI-37) lists the person's agents and all their work at once. On top is what waits for the person (questions, plans to approve, results to review, standing rules about to run out), then the work in progress per agent, then the standing rules. Each agent can be paused or stopped, and *Stop all my agents* stops every one in one step.
- On a phone, *My agents* shows *Waiting for you* first, so questions and reviews can be answered on the way.

### 9. Built for agents: tools, limits, safety

- **MCP tools for agents** (AI-36), in the one tool registry: *take* an issue that is handed over (all or nothing, with the issue's version, so two agents never start the same work), *report* a step, *ask*, *attach* a result, *give up* with a reason, and *changes since* (what changed in the issues it may see since a mark it got before). Every write tool takes `preview` and then answers what would happen, from the same one evaluation as the preview of *Change many issues*. Each tool says whether it only reads or changes something. Enforced by: ARCH-AI-TOOLS (every tool declares read or change and has a preview test).
- **Long texts:** agents never hold an edit lock for longer than one save and never take over a person's text; a save against an older version is refused with the newer text, as for people (*Compare and save*). Enforced by: the test `ai_36_an_agent_never_takes_over_a_lock` (M8).
- **Limits per agent and per key:** changes per minute, issues at once, attempts per issue. Reaching a limit pauses the agent with a sentence; it never drops a change halfway.
- **Untrusted text:** tool answers mark every text with who wrote it (*someone in the project*, *a Guest*, *someone outside*, *an agent*), so the agent's own safeguards can treat it as data, not as instructions (AI-05). The prompt-injection test set covers agents and standing rules.
- **COMP-10:** no tool ranks, evaluates or picks people. An app acting for a person may assign issues because the person told it to; an agent working on its own never changes a field that holds people (decision 3).

### 10. Agents of the organization (V1)

- *AUTH-08* (V1, M12) becomes **agents of the organization**: the same model, with the organization as the responsible party and a role instead of a person's rights, for webhooks (NTF-03) and imports. It is built on this decision, not beside it.

## Consequences

- One new kind of actor in `bb-policy` (*agent*), evaluated as the intersection of its person's rights, its work and its limits. The access-key narrowing of ADR-0025, decision 8, is reused for agent keys.
- New records: agents, hand-overs, standing rules, agent work with attempts, questions and reviews. Runs reuse `bulk_run`.
- New words in the glossary (*Agent*, *Hand over*, *Brief*, *Done when*, *Standing rule*, *Agent work*, *Attempt*, the agent states, *Accept / Send back / Throw away*, *My agents*, *Agent key*, *Written by an AI app / an agent*); *Bot account* gives way to *Agent*, and the assistant's entry no longer forbids the word *agent*.
- M8 grows by AI-30…AI-38 and the new permission; its demo shows a hand-over, a question, a review and *Throw away*.
- The UX design gains the hand-over window, agent work on the issue page, *My agents* and its phone form (UX concept).

## Alternatives considered

- **Agents as ordinary members with roles:** an agent could hold more than the person who set it up, and nobody would clearly answer for it. Rejected.
- **Assigning issues to agents:** blurs who answers for the issue, and makes AI part of who-does-what decisions about people. Rejected; *worked on by* shows the agent instead.
- **Access by saved search only:** anybody who edits an issue so that it matches gives the agent that issue, Guests included. Kept only for standing rules with narrow powers; real work goes through a hand-over by ID.
- **Agent progress as comments:** buries the conversation between people under logs. Rejected; agent work is its own record with one summary comment.
- **Bugaboo running agents:** needs heavy machines and access to code, and is out of scope (decision).

## Enforced by

ARCH-AI-TOOLS, ARCH-PERMS, SEC-MATRIX (see `tools/checks/catalog.toml`), and the tests named above, which the M8 work packages add.

<!-- COMP-10 checked: no tool or rule ranks, evaluates or assigns people; agents are never assigned; standing rules never change fields that hold people. -->
