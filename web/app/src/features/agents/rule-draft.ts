import { agentRuleDaysMost, ruleFieldWords, type Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";

/** A field a standing rule may be told to change: the short list of the server, never people, never the status. */
export type RuleFieldWord = Schema<"RuleField">;

/** What a standing rule may do: only propose, or change the fields it names. */
export type RulePower = "propose" | "change";

/** What the person filled in the window of a standing rule. */
export interface RuleDraft {
  readonly search: string;
  readonly agent: string;
  readonly project: string;
  readonly power: RulePower;
  readonly fields: readonly RuleFieldWord[];
  readonly createIssues: boolean;
  readonly createsIn: string;
  /** The days the rule lasts, as typed; empty is the usual number of the organization. */
  readonly days: string;
}

export interface DraftProps {
  readonly draft: RuleDraft;
  readonly onChange: (change: Partial<RuleDraft>) => void;
}

/** The fields the window offers, in the order of the server's list. */
export const offeredFields: readonly RuleFieldWord[] = ruleFieldWords;

/** The window before anything is filled in; a rule only proposes at the start. */
export function emptyRuleDraft(start: Partial<RuleDraft> = {}): RuleDraft {
  return {
    search: "",
    agent: "",
    project: "",
    power: "propose",
    fields: [],
    createIssues: false,
    createsIn: "",
    days: "",
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

/** Whether the window may be confirmed: a search, an agent, a project, the power and the days are right. */
export function ruleIsComplete(draft: RuleDraft): boolean {
  return (
    draft.search !== "" &&
    draft.agent !== "" &&
    draft.project !== "" &&
    (draft.power === "propose" || draft.fields.length > 0) &&
    (!draft.createIssues || draft.createsIn !== "") &&
    daysAreRight(draft.days)
  );
}

/** What the window sends. Only the fields of the short list can be in it: the list is closed. */
export function ruleBody(draft: RuleDraft) {
  const days = daysOf(draft.days);
  const fields =
    draft.power === "change" ? draft.fields.filter((field) => offeredFields.includes(field)) : [];
  return {
    agent: draft.agent,
    search: draft.search,
    project: draft.project,
    fields: [...fields],
    ...(draft.createIssues ? { creates_in: draft.createsIn } : {}),
    ...(days === null ? {} : { days }),
  };
}

/** Ticks or unticks one field, keeping the order of the server's list. */
export function withField(
  fields: readonly RuleFieldWord[],
  field: RuleFieldWord,
  on: boolean,
): RuleFieldWord[] {
  const keptSet = new Set(fields.filter((one) => one !== field));
  return offeredFields.filter((one) => (one === field ? on : keptSet.has(one)));
}

/** The names of the fields of the short list: the names the issue page uses, and *Duplicate of* for the relation. */
const FIELD_NAMES = {
  labels: "issue-field.labels",
  tags: "issue-field.tags",
  type: "issue-field.issue-type",
  "duplicate-of": "agents.rule-field.duplicate-of",
  "found-in-release": "issue-field.found-in-release",
} as const satisfies Record<RuleFieldWord, MessageKey>;

/** The name of a field of the short list, in words. */
export function ruleFieldText(field: RuleFieldWord): string {
  return t(FIELD_NAMES[field]);
}
