import { agentRuleDaysMost, type Schema } from "@bugaboo/api-client";
import type { MessageKey } from "@bugaboo/messages";

import { emptyDraft, powersOf, type HandOverDraft } from "../agent-work/index.ts";

/** How a standing rule works on what it finds: it proposes, or it hands each issue over. */
export type RuleWayWord = Schema<"RuleWay">;

/** What the person filled in in the window of a standing rule. */
export interface RuleDraft {
  readonly search: string;
  readonly agent: string;
  readonly project: string;
  readonly way: RuleWayWord;
  /** The powers of the hand-overs of a rule that hands over: the ticks, the fields, accepting by itself. */
  readonly handOver: HandOverDraft;
  readonly createIssues: boolean;
  readonly createsIn: string;
  /** The days the rule lasts, as typed; empty is the usual number of the organization. */
  readonly days: string;
  /** The rule never runs out (only where an administrator allows it). */
  readonly neverEnds: boolean;
}

export interface DraftProps {
  readonly draft: RuleDraft;
  readonly onChange: (change: Partial<RuleDraft>) => void;
}

/** The window before anything is filled in; a rule only proposes at the start. */
export function emptyRuleDraft(start: Partial<RuleDraft> = {}): RuleDraft {
  return {
    search: "",
    agent: "",
    project: "",
    way: "proposals",
    handOver: emptyDraft,
    createIssues: false,
    createsIn: "",
    days: "",
    neverEnds: false,
    ...start,
  };
}

/** The days as a number: none when empty, `NaN` for a text that is no whole number of days. */
function daysOf(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed === "") {
    return null;
  }
  return /^\d+$/u.test(trimmed) ? Number(trimmed) : Number.NaN;
}

/** Whether the days are empty or a whole number from 1 to the most a rule lasts. */
export function daysAreRight(text: string): boolean {
  const days = daysOf(text);
  return days === null || (days >= 1 && days <= agentRuleDaysMost);
}

/** Whether a rule that hands over asks to change fields and names none. */
function namesNoField(draft: RuleDraft): boolean {
  return (
    draft.way === "hand-over" && draft.handOver.changeFields && draft.handOver.fields.length === 0
  );
}

/**
 * Why *Create the rule* waits, as the line above the button says it: the first thing that is
 * missing or wrong, in the order of the window; none when the rule can be created.
 */
export function ruleWaitsFor(draft: RuleDraft): MessageKey | undefined {
  if (draft.search === "" && draft.agent === "") {
    return "agents.rule-need-search-agent";
  }
  if (draft.search === "") {
    return "agents.rule-need-search";
  }
  if (draft.agent === "") {
    return "agents.rule-need-agent";
  }
  if (draft.project === "") {
    return "agents.rule-need-project";
  }
  if (namesNoField(draft)) {
    return "agents.rule-need-fields";
  }
  if (draft.createIssues && draft.createsIn === "") {
    return "agents.rule-need-creates-in";
  }
  return draft.neverEnds || daysAreRight(draft.days) ? undefined : "agents.rule-need-days";
}

/** Whether the window may be confirmed: nothing is missing or wrong. */
export function ruleIsComplete(draft: RuleDraft): boolean {
  return ruleWaitsFor(draft) === undefined;
}

/** What the window sends: nothing of a hand-over for a rule that proposes, and no days for a rule that never runs out. */
export function ruleBody(draft: RuleDraft) {
  const days = daysOf(draft.days);
  const handing = draft.way === "hand-over";
  return {
    agent: draft.agent,
    search: draft.search,
    project: draft.project,
    way: draft.way,
    powers: handing ? powersOf(draft.handOver) : [],
    fields: handing && draft.handOver.changeFields ? [...draft.handOver.fields] : [],
    accept_by_itself: handing && draft.handOver.acceptByItself,
    ...(draft.createIssues ? { creates_in: draft.createsIn } : {}),
    ...(draft.neverEnds ? { never_ends: true } : days === null ? {} : { days }),
  };
}
