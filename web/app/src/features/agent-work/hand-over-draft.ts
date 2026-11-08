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

/** The members of an issue an agent can be given to change, by their codes; the status is a power of its own. */
export const nameableFields = [
  "title",
  "description",
  "priority",
  "estimated-time",
  "time-left",
  "issue-type",
  "labels",
  "tags",
] as const;

/** A whole number typed in a box, or nothing where the box is empty or holds something else. */
export function wholeNumber(text: string): number | undefined {
  const trimmed = text.trim();
  return /^\d+$/u.test(trimmed) ? Number(trimmed) : undefined;
}

/** Whether every box that holds something holds a whole number. */
export function numbersAreRight(draft: HandOverDraft): boolean {
  return [draft.attempts, draft.minutes, draft.cost, draft.days].every(
    (text) => text.trim() === "" || wholeNumber(text) !== undefined,
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
    wholeNumber(draft.cost),
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
