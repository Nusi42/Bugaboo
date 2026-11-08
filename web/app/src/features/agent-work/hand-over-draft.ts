import { nameableMembers } from "@bugaboo/api-client";
import { centsIn, wholeNumberIn } from "../../shared/number.ts";
import type { OwnField, TypeInfo } from "../fields/index.ts";
import type { Power } from "./agent-work.ts";

/** What the person has filled in in the window of a hand-over. */
export interface HandOverDraft {
  /** The ID of the agent; none until one is chosen. */
  readonly agent: string;
  readonly note: string;
  readonly changeStatus: boolean;
  readonly changeFields: boolean;
  readonly createSubIssues: boolean;
  /** The members of the issue the agent may change, by their codes. */
  readonly fields: readonly string[];
  /** The budget and the end as typed; empty means the usual. */
  readonly attempts: string;
  readonly minutes: string;
  /** The most cost the agent reports, in euros, written the way the person writes numbers. */
  readonly cost: string;
  readonly days: string;
  readonly planNeedsApproval: boolean;
  readonly acceptByItself: boolean;
}

/** A hand-over with nothing filled in: no power ticked, the plan needs no approval. */
export const emptyDraft: HandOverDraft = {
  agent: "",
  note: "",
  changeStatus: false,
  changeFields: false,
  createSubIssues: false,
  fields: [],
  attempts: "",
  minutes: "",
  cost: "",
  days: "",
  planNeedsApproval: false,
  acceptByItself: false,
};

/** A member of an issue the window offers a box for: a code of the catalogue, or an own field by its ID and name. */
export interface NameableField {
  readonly code: string;
  readonly name?: string;
}

/**
 * What an agent can be given to change, one box each: the members the server lists
 * (`nameableMembers`) and the own fields of issues that are not retired and hold no people (the
 * server refuses the others, COMP-10).
 */
export function nameableFields(
  own: readonly Pick<OwnField, "id" | "name" | "kind" | "applies_to">[],
  kinds: readonly Pick<TypeInfo, "kind" | "traits">[],
): readonly NameableField[] {
  // `useFields` leaves retired fields out already; a kind that holds people is never offered.
  const people = new Set(
    kinds.filter((info) => info.traits.includes("holds-people")).map((info) => info.kind),
  );
  return [
    ...nameableMembers.map((code) => ({ code })),
    ...own
      .filter((field) => field.applies_to === "issues" && !people.has(field.kind))
      .map((field) => ({ code: field.id, name: field.name })),
  ];
}

/** A whole number typed in a box, or nothing where the box is empty or holds something else. */
export function wholeNumber(text: string): number | undefined {
  return wholeNumberIn(text) ?? undefined;
}

/** The cost typed in euros as cents, or nothing where the box is empty or holds no amount. */
function costCents(text: string): number | undefined {
  return centsIn(text) ?? undefined;
}

/** Whether every box that holds something holds what it asks: whole numbers, and an amount for the cost. */
export function numbersAreRight(draft: HandOverDraft): boolean {
  const whole = [draft.attempts, draft.minutes, draft.days];
  return (
    whole.every((text) => text.trim() === "" || wholeNumber(text) !== undefined) &&
    (draft.cost.trim() === "" || costCents(draft.cost) !== undefined)
  );
}

/** The powers that are ticked. */
function powersOf(draft: HandOverDraft): Power[] {
  return [
    ...(draft.changeFields ? (["change-fields"] as const) : []),
    ...(draft.changeStatus ? (["change-status"] as const) : []),
    ...(draft.createSubIssues ? (["create-sub-issues"] as const) : []),
  ];
}

/** What the server is asked: the issues, the agent, the note, the ticks and the budget. */
export function handOverBody(draft: HandOverDraft, issues: readonly string[]) {
  const [attempts, minutes, cost, days] = [
    wholeNumber(draft.attempts),
    wholeNumber(draft.minutes),
    costCents(draft.cost),
    wholeNumber(draft.days),
  ];
  return {
    agent: draft.agent,
    issues: [...issues],
    note: draft.note,
    powers: powersOf(draft),
    fields: draft.changeFields ? [...draft.fields] : [],
    plan_needs_approval: draft.planNeedsApproval,
    accept_by_itself: draft.acceptByItself,
    ...(attempts === undefined ? {} : { budget_attempts: attempts }),
    ...(minutes === undefined ? {} : { budget_minutes: minutes }),
    ...(cost === undefined ? {} : { budget_cost: cost }),
    ...(days === undefined ? {} : { days }),
  };
}

/** What a part of the window gets: the draft and the way to change a piece of it. */
export interface DraftProps {
  readonly draft: HandOverDraft;
  readonly onChange: (change: Partial<HandOverDraft>) => void;
}
