# WP-11 — Agent work on screen: the hand-over window, the issue page, the review, list rows

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** AI-31, AI-33, AI-34, AI-35 (on screen), AI-32 (proposals on the issue). UX design §4.17, §4.4 (the issue page), §4.2 (list rows), §6.3 (windows), the pictures WP-01 draws; ADR-0029 decisions 3 to 6.

## Goal

From one issue or several ticked ones, a person hands work to one of their agents in one window that shows what the brief is missing. On the issue, agent work is a block of its own under the description, apart from the comments: who handed it over, the brief, each attempt with what it changed, its questions answered right there, and the review. A list row shows that an agent works on the issue without losing its status color.

## Owns

`web/app/src/features/agent-work/**` (the *Hand over to an agent* window, the agent-work block, the question and its answer, *Approve plan*, the review window, the proposals of standing rules on the issue), the place of the block and *worked on by* on the issue page in `features/issues`, the agent mark on list rows, *Hand over to an agent* among the actions for ticked issues, the theme rules of these parts.

## What is decided (ADR-0029, UX §4.17)

- ***Hand over to an agent*** is a window (§6.3) from the issue's *More* and from the ticked issues: the agent (only the person's own; *Create an agent* links to WP-12's page), the brief taken from the issue with what is missing named (*"No steps to reproduce"*, *"No Done when"*) and a note, the ticks of what it may do (always-allowed shown as text, the others as boxes), the budget, *Plan needs approval*, the end (14 days). *Hand over* stays possible with something missing.
- **The agent-work block** under the description: the state in words and as a mark, the agent as *"Fixer, agent of Ada Brandt"* (by *Who sees people*), the brief, each attempt (plan, steps, results as links, checks, the time and cost *as reported by the agent*, the link to its run), questions with an answer box right there, *Approve plan* with a note, the proposals of a standing rule with the assistant's proposal card (WP-10's, taken from `main` after WP-10 is merged).
- **The review window** at *Ready for review*: *Accept* with the status to move to (the workflow's proposal first), *Send back* with a required note, *Throw away* with the preview of *Undo the run* (what goes back, what is skipped) and the sentence that a branch or anything else outside Bugaboo stays as it is.
- ***worked on by Fixer*** next to *Assigned to* in the Details; the person stays.
- **List rows:** a small agent mark with the state in its accessible name; the row keeps its status tint and priority edge (UX §7.3). The column *Agent* and its state can be chosen in *Customize*.
- **The phone:** the block and the review work on a phone (the demo answers a question from the phone).

## Tasks

1. **The window** of a hand-over, for one issue and for ticked issues.
2. **The block** on the issue page, the answer to a question, *Approve plan*, the proposals.
3. **The review window**, with the long preview of *Undo the run* (the one component of the long preview).
4. **List rows and the column** of the agent state.
5. **Pictures** of every changed screen in four themes on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `ai_31_hand_over_names_what_the_brief_is_missing_and_still_hands_over`.
- `ai_31_hand_over_from_ticked_issues_gives_each_issue_to_the_agent`.
- `ai_33_agent_work_shows_under_the_description_apart_from_the_comments`.
- `ai_33_the_issue_shows_worked_on_by_and_keeps_its_person`.
- `ai_33_a_list_row_shows_the_agent_mark_and_keeps_its_status_color`.
- `ai_34_a_question_is_answered_on_the_phone_and_the_answer_joins_the_brief`.
- `ai_34_approve_plan_lets_the_attempt_start`.
- `ai_35_send_back_needs_a_note_and_starts_a_new_attempt`.
- `ai_35_throw_away_shows_what_goes_back_and_undoes_it`.
- `ai_35_accept_moves_the_issue_to_the_chosen_status`.
- TEST-A11Y on every changed screen; the keyboard alone hands over, answers and reviews.

## Checks

TEST-A11Y, THEME-ONLY, STY-PARTS, STY-BOLD, STY-TOKENS, TXT-TERMS, TXT-DATES, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the long preview and the window from their one component, the agent shown through the one person part, the state words from the API's words, the proposal card from WP-10. A UX reviewer compares every picture with UX §4.17, §4.4, §1.1 and §7. A words reviewer reads every new message.

## Depends on

Wave 2 merged (WP-07, WP-08). Builds beside WP-10 and takes its proposal card from `main` before it pushes; merges third in wave 3, after WP-09 and WP-10; WP-12 takes the agent mark from `main` after this merge.
