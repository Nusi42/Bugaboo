# WP-14 — Standing rules that hand over, and rules with no end

**Wave 5 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** AI-32 (changed), AI-31, AI-35, AI-37. ADR-0029, decisions 3 and 6 with the amendment "Rules that hand over" (the maintainer's decision of 2026-11-09: Bugaboo must be able to run agents with no person in the loop). Builds on WP-08, WP-12 and WP-13.

## Goal

A standing rule can do real work without a person starting it. It finds issues with its saved search and **hands each one over** to its agent, with the powers of the rule and the rules of every hand-over; the agent takes it and works in attempts with runs, so *Throw away* and *Undo the run* stay complete. A rule that only reads and proposes stays as it is. The old way "change only these fields" is gone: an agent never changes an issue it was not handed. A rule can be allowed, by an administrator, to **never run out**, so a team can run agents for good. With *Accept by itself when its checks pass* the loop has no person in it by design; *My agents* lists what was accepted that way.

## Owns

`crates/bb-app/src/agent_work/rules.rs` and the new `rules_hand_over.rs` beside it, the rule members of `crates/bb-core/src/agent_work.rs` and `agent_records.rs`, the setting `agents.rules-never-run-out` (`crates/bb-core/src/settings/agents.rs`) and its words, the rule columns of `bb-store` and migration 48, the standing-rule window and the rule cards in `web/app/src/features/agents/**`, `userDocs/agents/**`, the rule of TEST-SAMPLE-DATA (`AiPlan::judge`) and the sample's rule, the picture set `.work/shots/M7/rules-hand-over/`.

## What is decided (ADR-0029, amendment "Rules that hand over")

- **A rule has a way of working:** *Propose* (as before: the agent reads what the rule finds and proposes; nothing changes until a person applies it) or *Hand over what it finds* (new). The way is chosen in the rule window and cannot be changed later; a person makes a new rule.
- **What a rule that hands over makes:** each issue its saved search finds (when the rule is made and whenever one comes under it later), inside the rule's project and the person's rights, becomes **one hand-over** of that issue to the rule's agent: the powers of the rule (the ticks and named fields of the hand-over window), the limit of attempts of the agent as the budget, no plan approval, and *Accept by itself* left to the project (it counts only where the project allows it). It has the end of every hand-over (14 days unless set), one taker, its event, its notifications when the agent asks or is ready, and the review. The person must hold, now, what a hand-over needs: *Change issues* (or *Change the issues they reported*) on the issue and *Create agents that work for you* in its project, with agents on there; an issue they cannot hand over now is left out and looked at again at the next change.
- **Handed over once:** the match record of the rule is the memory. An issue is handed over by a rule once, when it first comes under it; that rule never hands it over again, not when the hand-over ended, not when the issue changed and matches again. An issue that already has open work of the agent (from a hand-over by a person or by another rule) is only written down as found. To have it worked again, a person hands it over or sends it back. (The alternative "again after it changes" would let an agent trigger itself through its own changes.)
- **The limit caps the flood:** a rule hands over only while the agent has fewer open pieces of work than its limit *Issues at once*. The rest are not written down yet and are looked at again when room is made: the notification round that sees a hand-over end (run-out, a person ending it, its last work closed), work stopped or given up, or a review that ends work queues the look for the projects of the agent's rules that hand over, and at the next change of an issue in the project. An agent never has more open hand-overs than it may work at once.
- **No change without an attempt, for rules too:** on an issue that only a rule found (way *Propose*) the agent reads and proposes, nothing else (M7-F120). The option *Change only these fields* is removed from the window, the API, the docs and the glossary. A rule made with it before is migrated to *Propose* (its named fields are dropped: it never changed anything without an attempt, so nothing it did is lost).
- **Rules with no end:** a rule keeps 30 days by default and 90 days at most. The organization setting **Allow rules that never run out** (`agents.rules-never-run-out`, administrators only, off at the start) lets a person choose *Never runs out* in the rule window. A rule that never runs out can be stopped at any time and the limits of its agent still apply; its hand-overs end like every hand-over. Switching the setting off later gives each rule that never runs out an end at the most days (90) from that moment, with the usual warning before it runs out (the job `agents.rules-get-an-end`, queued in the change of the setting); no new rule can then be created without an end.
- **Nobody waits:** nothing in the loop waits for a person by design. A question or a plan still waits when the agent asks or the hand-over says so, as before; with *Accept by itself* an attempt whose checks passed is accepted by the system, and it is listed under *Accepted by itself* in *My agents* until a person has looked at it.

## Tasks

1. **Write first** (this file, README, ADR amendment, glossary, follow-ups), then **tests first** on both databases (below).
2. **Core:** the word `RuleWay` (`propose`, `hand-over`); a rule holds its way, the powers and named fields of its hand-overs, and an end that may be none; `RulePower`, `Allowance::of_rule` and the field list of the old option are removed; `rule_end` and its bounds follow the setting; the new setting and its words in `messages/en.json`.
3. **Store:** migration 48 (the way and the powers; existing rules become *Propose* with no named fields); `ends_at` of a rule that never runs out is stored as the one named value `NEVER_ENDS`; a count of the open work of an agent.
4. **Use cases:** `create_standing_rule` and `change_standing_rule` judge the end by the setting; `run_rule_matches` hands over what a rule of the way *Hand over* finds, through the one `plan_hand_over` (with `HandOverPlan::record_change`) that the *Hand over to an agent* use case also calls (SSOT), one transaction per issue (the match, the hand-over, its work and the job of its end, under the lock of the work of the agent).
5. **API and client:** the body and view of a rule carry `way`, `powers`, `fields` (now the named members of a hand-over) and an optional end; regenerate the OpenAPI document, the client and the messages with the xtask gen commands.
6. **The rule window:** the way as two choices (*Propose changes*, *Hand over what it finds*), the powers of a hand-over under the second, *Never runs out* where the setting allows it; the rule cards say *Hands over* and *Never runs out*. Words from messages only; theme parts only from the theme files (UX §4.17, §6.3).
7. **The settings page** of agents shows the new setting; *My agents* › *Accepted by itself* lists the attempts accepted by themselves for hand-overs that rules made; SEC-MATRIX and ARCH-SETTINGS rows.
8. **Sample and user docs:** one rule of the sample hands over; `TEST-SAMPLE-DATA` (`AiPlan::judge`) follows the setting for the longest end; `userDocs/agents/*` explain both ways, the limit and *Never runs out*.
9. **Pictures:** the rule window and the rule cards, computer and phone, four themes, in `.work/shots/M7/rules-hand-over/`, looked at one by one at full size.

## Acceptance tests (write first)

- `m7_wp14_a_rule_hands_over_what_its_search_finds`, now and later, as the rule's person, with the powers of the rule and its event.
- `m7_wp14_one_hand_over_for_each_found_issue_and_never_again`, also after it ended and when a person had handed it over already.
- `m7_wp14_the_limit_of_the_agent_caps_the_open_hand_overs_of_a_rule`, and the rest are handed over when room is made.
- `m7_wp14_an_agent_takes_a_hand_over_of_a_rule_and_throw_away_undoes_the_attempt`.
- `m7_wp14_a_rule_that_proposes_hands_over_nothing_and_its_agent_changes_nothing`.
- `m7_wp14_a_rule_that_never_runs_out_needs_the_setting` and `m7_wp14_the_setting_is_for_administrators_only` (SEC-MATRIX, ARCH-SETTINGS).
- `m7_wp14_a_migrated_rule_proposes`, on both databases (migration test).
- `m7_wp14_an_accept_by_itself_of_a_hand_over_of_a_rule_is_listed_in_my_agents`.
- Browser (computer and phone): the rule window with both ways and *Never runs out* off and on (`web/app/e2e/rules-hand-over.spec.ts`); unit tests for the draft of the window.

## Checks

SEC-MATRIX, ARCH-SETTINGS, ARCH-EVENTS, ARCH-PERSON-REF, DATA-MARKS, ARCH-DB-PARITY, TEST-SAMPLE-DATA, TEST-A11Y, TXT-TERMS (glossary and user docs), LINT-LINKS, the generated files in step.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the hand-over of a rule goes through the one `plan_hand_over`; the days and the setting are written once; the old option has no leftover (no `RulePower`, no `of_rule`, no list of fields). A security reviewer reads: a rule can hand over only what its person may hand over now, never an issue of another project, never past the limit; an unattended agent has no way to do more than a hand-over allows (COMP-10: no people fields, never assigned). A UX reviewer looks at the pictures.

## Depends on

WP-13 (the refusal of changes on issues only a rule found, M7-F120; the sample; the user docs) and WP-12 (the rule window) merged. Migration 48.
