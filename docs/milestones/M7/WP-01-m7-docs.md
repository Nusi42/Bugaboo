# WP-01 — Decision records, words, security and privacy, UX gaps, the maintainer's answers

**Wave 1 · Folder / branch:** `bugaboo.win1` / `WIN1` (lead)
**Features:** all of M7, the docs they need (AI-01 to AI-05, AI-10, AI-20 to AI-23, AI-30 to AI-38, COMP-08 to COMP-11, PRIV-08); ADR-0013, ADR-0017, ADR-0025, ADR-0029, ADR-0033, the new ADR-0042.

## Goal

Before code names anything new, the words, the decisions and the maintainer's answers are written down once, so the builders of waves 2 and 3 read one source. Wave 1's code packages (WP-02, WP-03, WP-04) need only what is decided already (ADR-0013, ADR-0029 and the placement in the [README](README.md)) and start while the lead writes the rest.

## Tasks

1. **The maintainer's answers** to M7-Q1…Q10 ([README](README.md)), each written where it belongs:
   - **M7-Q1** (who may create agents): ADR-0029 decision 2 and the ADR-0025 amendment M7.
   - **M7-Q2** (what the assistant does): ADR-0013 amendment M7, UX §4.12, and 04-milestones M7 *Demo (exit)* if the wording of the demo changes.
   - **M7-Q3, Q4** (consent, *Connected apps*, which apps may register): ADR-0042, architecture §7 *MCP*, and the feature list (AI-22, AI-24) if part of AI-24 moves into M7.
   - **M7-Q5, Q6** (names of people, conversations, the log of tool calls): ADR-0013 amendment M7, architecture §7 *AI*, and *Personal data in M7*.
   - **M7-Q7, Q8** (an agent's time, *Accept by itself*): ADR-0029 decisions 3, 4 and 6, and ADR-0035 decision 9 (no time entry by an agent).
   - **M7-Q9** (how the maintainer tries M7): the M7 section of architecture §11 and the README's exit demo.
   - **M7-Q10** (what stays out): the follow-up tables and the README.
2. **A new decision record, ADR-0042 *The MCP server and how AI apps sign in*:**
   - the MCP server inside Bugaboo at `/mcp` on the server's own port, streamable HTTP, `rmcp`;
   - the OAuth 2.1 authorization server inside Bugaboo: authorization code with PKCE (S256 only), the metadata documents of the MCP authorization spec, registration as M7-Q4 says, short access tokens (15 minutes), refresh tokens that turn over at each use and stop the whole grant when an old one is used again, everything stored as a hash;
   - the scopes *See only* and *See and change* (the words of access keys), which only narrow the person (ADR-0025, decision 8); what no key holds no OAuth grant holds;
   - an access key as the other way in (`Authorization: Bearer bb_…`);
   - the consent step and *Connected apps* (M7-Q3); *Sign out everywhere* also ends every grant (as it ends access keys);
   - each MCP session is one run (ADR-0029, decision 7); the history says *"Ada Brandt via Claude Code"*;
   - rate limits per grant and per person; the security log of every grant, renewal and take-back;
   - Enforced by: SEC-MATRIX (`mcp` rows), ARCH-API-OUT, ARCH-AI-TOOLS, and the tests named in WP-05.
3. **Amendments:**
   - **ADR-0013 (M7):** the placement of the README (the registry and the context builder in `bb-app`, the adapters in `bb-ai`, MCP in `bb-mcp`); the settings of AI (off for the organization, a switch per project that follows the one above, the provider, the model, the address, the API key as a stored secret, the agreement step, *What goes where*); *Never sent to AI* on fields and projects in M7, restricted issue groups with SEC-05; names of people (M7-Q5); what the log of a tool call holds (M7-Q6); the author marks of untrusted text.
   - **ADR-0029 (M7):** M7-Q1, Q7, Q8; the default limits of an agent (60 changes a minute, 5 issues at once, 5 attempts on one issue; each a setting of the agent within the organization's maximum); proposals of standing rules are records a person applies; what an agent never holds (every permission of kind *Manage* and *Take over text someone else is editing*).
   - **ADR-0025 (M7):** *Create agents that work for you* in the catalogue (topic, where it counts, kind, starting roles); an agent is no member; agent keys narrow like access keys.
   - **ADR-0017 (M7):** the jobs of M7 (`agents.hand-over-runs-out`, `agents.rule-runs-out`, `agents.rule-about-to-run-out`, `agents.rule-matches`, `oauth.clean-old-grants`), each with its schedule.
   - **ADR-0033 (M7):** the notification kinds of M7 (*An agent asks you*, *A plan waits for you*, *Ready for review*, *An agent gave up*, *A standing rule runs out soon*, *An agent was stopped*) with their starting values of *in the app* and *by e-mail*.
   - **ADR-0018:** the setting keys of AI and of agents, under the topics *AI* and *Agents*.
4. **The port of runs.** `RunStore` in `bb-app/src/ports.rs`: open a run for an MCP session or an attempt (with its actor), close it, read its changes (paged), preview *Undo the run* (what goes back, what is skipped and why) and undo it with the version of the preview. The shape (names, inputs, outputs, the rule that an undo never touches a change somebody made later) is written here before wave 2 starts, so WP-05 and WP-08 test against a fake while WP-07 builds it.
5. **Architecture:** §4 (the placement, the module map without `mcp --> ai` and with the edges the crates have, M4-F133), §5 (the sequence of a hand-over and an attempt, next to the assistant's), §7 *AI* and *MCP* (with the answers), §11 (sample part 6, the ten-people cases of agents, the M7 speed scenarios), the scope line of §12 ("M6 built, M7 planned").
6. **Glossary** (*AI* and *People and access*):
   - setting up AI: *Set up the assistant*, *AI provider*, *Data processing agreement*, *What goes where*, *Never sent to AI*, *Send names of people* (if M7-Q5 is taken), *Local model*;
   - the assistant: *Proposed change*, *Apply*, *Discard*, *Looking at*;
   - AI apps: *Allow* (the consent step), *Take back* (of an app's access), the scopes *See only* and *See and change* for apps;
   - agents: *Pause*, *Resume*, *Limits* (of an agent), *Budget* (of a hand-over), *Plan needs approval*, *Accepted by itself*, *Check* (a result an agent attaches), *Undo the run*;
   - internal terms: *Tool registry*, *Context builder*, *Fenced text*, *Author mark*, *Actor kind*, *Grant* (of an app).
   Each with its *Don't say* (for example *LLM*, *prompt*, *bot*, *copilot*, *auto-pilot*, *token* in the sense of a credential, *revoke*, *scope* on screens).
7. **Checks** (planned, in the catalogue, with their rule text, early in the wave so WP-03 rebases onto them): TEST-PROMPT-INJECTION (WP-03 makes it active), written as "Adding a rule" in architecture §10 asks; the files of ARCH-AI-TOOLS after the placement (`crates/bb-app/src/ai/tools/**`, `crates/bb-mcp/**`).
8. **Security chapter:** the section *Assistant, AI apps and agents (M7)* in architecture §12 and the new boundary *Server ↔ AI provider* in its picture, STRIDE rows with what proves each:
   - an AI app or an agent reads or changes more than its person may, or than its scope, its work or its limits allow (elevation of privilege; SEC-MATRIX, TEST-PROMPT-INJECTION);
   - text in an issue tells the AI to change, send or show something (tampering, information disclosure; AI-05; TEST-PROMPT-INJECTION);
   - an answer of the provider carries a picture or a link that sends data out (information disclosure; SEC-RICHTEXT);
   - a field kept from AI, a hidden field or a hidden issue reaches the provider (information disclosure; the context tests of WP-03);
   - a stolen refresh token, a code without its PKCE proof, an app that registers someone else's return address (spoofing; the tests of WP-05);
   - an administrator points the provider address at the internal network and reads it through the errors (information disclosure; the password asked again, errors that never echo the provider, WP-02);
   - an agent runs away with changes (denial of service, tampering; limits, *Stop all my agents*, *Undo the run*);
   - nobody can tell what an AI wrote (repudiation; COMP-09 marks, the actor and the app in every event).
9. **Personal data in M7** (architecture §12 and `docs/privacy/personal-data.md` through DATA-MARKS): what goes to an outside provider and on which basis (the agreement step, PRIV-08); briefs, questions, answers, plans and notes of agent work as people's text; agents and their keys; OAuth grants; the log of tool calls (no content); the conversation of the assistant (not stored, M7-Q6).
10. **UX design** (§4.12, §4.17, §4.13 and the settings chapter), drawn the way §4.9 draws the computer and the phone:
    - *Organization settings › AI*: *Set up the assistant* (provider, model, address, key, the agreement step, *What goes where*), apps that may sign in, the project switch;
    - *Never sent to AI* on a field and on a project;
    - the consent step of an AI app and *Connected apps*;
    - the *Hand over to an agent* window (with what is missing in the brief), the agent-work block on the issue page (attempts, questions answered there, *Approve plan*), the review window (*Accept* with the status, *Send back* with a note, *Throw away* with what is undone and what is not);
    - *My agents* on computer and phone (*Waiting for you* first), an agent's page (keys, limits, *Pause*, *Stop*), the standing-rule window, *Organization settings › Agents*;
    - the marks *Written with the assistant*, *Written by an AI app*, *Written by an agent* beside a text, and the run page.
11. **Follow-up tables:** the rows of earlier milestones whose target is M7 point to their M7 package (README, "Follow-ups handled in M7"; [follow-ups.md](follow-ups.md)).

## Acceptance

- `cargo xtask check staged` green on the docs (LINT-LINKS, PLAN-TRACE, LINT-SPELL, FMT-MD, TXT-TERMS, META-RULE-TRACE for every new "must", DOC-ADR-ENFORCED for ADR-0042).
- Every answer of M7-Q1…Q10 is findable in the place this list names.
- The builders of wave 2 have the ADRs, the words, the port shape and the pictures before they start.

## Review

A reviewer with a fresh context reads the docs against the maintainer's answers and applies the SSOT/DRY/ODR gate of coding-standards §1 to them: a rule written in two docs, or a word that the glossary names otherwise, blocks the merge like code does. A security reviewer reads ADR-0042 and the M7 section of §12. A docs-only fix round is still read before the merge.

## Depends on

The maintainer's answers to M7-Q1…Q10. Nothing else.
