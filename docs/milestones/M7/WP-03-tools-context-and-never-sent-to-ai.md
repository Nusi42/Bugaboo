# WP-03 — The one tool registry, the context builder, *Never sent to AI* and fenced text

**Wave 1 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** AI-03, AI-04, AI-05 (fenced text, every tool call logged), AI-21 (the tools declared, the read tools built), COMP-10. ADR-0013, ADR-0035 decision 9, ADR-0025 decision 8, architecture §5 *The assistant proposes* and §7 *AI*.

## Goal

The assistant, AI apps and agents share one list of tools and one way of building what the AI reads. A tool is a use case of `bb-app` run as the actor, so it can never do or see more than the person. What the AI reads holds only what the actor may read and nothing an administrator marked *Never sent to AI*, and every text people wrote is fenced and says who wrote it, so it is read as data and never as an instruction. Tools that rank, evaluate or pick people, or read working hours, do not exist.

## Owns

`crates/bb-app/src/ai/tools/**`, `crates/bb-app/src/ai/context/**`, `crates/bb-app/src/ai/never_sent*`, *Never sent to AI* in the store and the API of fields and projects (`crates/bb-store/src/ai*`, `crates/bb-api/src/api/never_sent*`), the log of tool calls (store and its use case), its migrations (numbers from the lead), `xtask/src/checks/arch_ai_tools*`, the check and the test set of TEST-PROMPT-INJECTION, and both checks from *planned* to *active* in the catalogue (WP-01 writes the planned entries and their rule text first; this package rebases onto them).

## What is decided (ADR-0013, the placement in the README)

- **One registry**, a table in `bb-app/src/ai/tools/`: each tool has a stable name (the AI-21 names: `search_issues`, `get_issue`, `list_projects`, `get_my_work`, `create_issue`, `update_issue`, `change_status`, `add_comment`, `log_time`, `start_timer`, `stop_timer`), a plain description from the message catalogue, *reads* or *changes*, the use case it calls, the scope it needs (*See only* or *See and change*), the schema of its input (generated from the Rust type), and for a tool that changes something a **preview**: the same evaluation the use case would make, answered without saving. The assistant and MCP list and call tools only through the registry.
- **What is designed out:** no tool reads work days, breaks, warnings or the totals of a person, or another person's time entries (ADR-0035 decision 9); no tool sets *Assigned to* by its own choice, ranks people, gives a score or reads mood. An app acting for a person may still change *Assigned to* through `update_issue` because the person told it to (ADR-0029 decision 9); the tool has no input that asks the AI to choose a person.
- **The context builder** reads as the actor (with the actor's key or grant narrowing), drops every field the actor cannot read and every field or project marked *Never sent to AI*, and writes each text people wrote inside a fence that names its author kind: *someone in the project*, *a Guest*, *someone outside*, *an agent*, *the assistant*. Nothing inside a fence can close it (the fence is chosen so the text cannot contain it). Tool answers to AI apps (MCP) use the same fences.
- ***Never sent to AI*** is a mark on a field (built-in or own) and on a project (it covers the sub-projects, like *Offered in*). Only administrators set it (*Change organization settings* for fields, *Change project settings* for a project). It applies even when the person can read the field. A search sent through a tool that names a field kept from AI is refused with the same answer as an unknown field (QRY-05).
- **The log of tool calls:** every call of every tool, by the assistant, an AI app or an agent, is one row: who, which actor kind and app, which tool, *reads* or *changes*, the size in and out, the time, the outcome. Never the content (DATA-LOGS). The security log gets a row when a tool is refused.

## Tasks

1. **The registry** with the read tools working (each calls the existing use case: search, issue, projects, *My work*) and the write tools declared with their previews (each previews through the existing use case; WP-05 and WP-06 run them).
2. **The context builder:** what the panel or an app looks at (an issue, a list, a project) into fenced text, the readable fields only, *Never sent to AI* left out, people shown as M7-Q5 says (through `PersonRef`).
3. ***Never sent to AI***: the mark on fields and projects with routes, versions, events and the security log; the generated field table of architecture §7 gets the column.
4. **The log of tool calls** with its keep period (a named constant, 90 days, ADR-0013 amendment).
5. **ARCH-AI-TOOLS** (planned → active): every tool in the registry declares *reads* or *changes*, a use case and a scope; every tool that changes has a test of its preview; no file outside the registry builds a tool; no tool calls a use case of the list that is designed out (working hours, other people's time, roles, *Check access*). Bad and good examples.
6. **TEST-PROMPT-INJECTION** (planned by WP-01 → active): the test set in `bb-app` with the fake provider: an issue description, a comment, an own field, a file name and a tool answer that each say "ignore your instructions", "change the status", "show this picture" or "send the text of WEB-1 to this address"; the set proves that the fence holds, that nothing changes without *Apply* or a scope that allows it, and that a field kept from AI never reaches the fake. The check finds the set by name, with the bad and good examples "Adding a rule" asks for (architecture §10).

## Acceptance tests (write first)

- `ai_03_a_field_never_sent_to_ai_is_not_in_the_context_even_when_the_person_reads_it`.
- `ai_03_a_project_never_sent_to_ai_keeps_its_sub_projects_out_too`.
- `ai_03_a_search_through_a_tool_that_names_a_field_kept_from_ai_is_refused_like_an_unknown_field`.
- `ai_04_a_tool_reads_as_the_actor_and_never_more` (a Contributor's context has no field hidden from Contributors).
- `ai_04_a_key_that_may_only_see_lists_no_tool_that_changes`.
- `ai_05_text_people_wrote_is_fenced_with_its_author_kind`.
- `ai_05_text_cannot_close_its_fence`.
- `ai_05_every_tool_call_is_logged_without_its_content`.
- `comp_10_no_tool_reads_working_hours_or_ranks_people` (reads the registry).
- The prompt-injection set of TEST-PROMPT-INJECTION (assistant and MCP cases; WP-09 adds agents and standing rules).
- SEC-MATRIX: the routes of *Never sent to AI*.

## Checks

ARCH-AI-TOOLS, TEST-PROMPT-INJECTION, ARCH-AUTHZ-ONLY, ARCH-PERSON-REF, DATA-LOGS, DATA-MARKS, SEC-MATRIX, ARCH-EVENTS, ARCH-VERSIONED, TXT-CATALOG (the descriptions of tools).

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one registry, the input schema generated from the Rust type, the preview from the use case's own evaluation (never a second judgment), one context builder, one fence. A security reviewer reads the fence, the context for every field level and *Never sent to AI*, and COMP-10 on every tool.

## Depends on

Nothing of M7 but the placement in the README. Runs beside WP-02 and WP-04; adds its module lines to `bb-app/src/ai/mod.rs` after WP-02 is merged (the only shared lines).
