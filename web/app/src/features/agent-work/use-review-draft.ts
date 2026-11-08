import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { acceptChoices, type AcceptChoice } from "./accept-status.ts";
import type { Verdict } from "./agent-work.ts";

/** The status left as it is when the person picks none. */
export const KEEP_STATUS = "";

/** What the person has filled in at the review so far. */
export interface ReviewDraft {
  readonly answer: Verdict;
  readonly setAnswer: (answer: Verdict) => void;
  readonly note: string;
  readonly setNote: (note: string) => void;
  readonly status: string;
  readonly setStatus: (status: string) => void;
  /** The statuses to move the issue to, the workflow's proposal first. */
  readonly choices: readonly AcceptChoice[];
  /** *Send back* needs a note. */
  readonly needsNote: boolean;
  /** What the review sends besides the version of the work. */
  readonly body: () => { answer: Verdict; note?: string; status?: string };
}

/** The draft of a review: the answer, the note of a send-back and the status of an accept. */
export function useReviewDraft(
  statusChanges: readonly Schema<"StatusChangeRef">[] | null | undefined,
): ReviewDraft {
  const [answer, setAnswer] = useState<Verdict>("accept");
  const [note, setNote] = useState("");
  const choices = acceptChoices(statusChanges);
  const [status, setStatus] = useState(choices[0]?.value ?? KEEP_STATUS);
  return {
    answer,
    setAnswer,
    note,
    setNote,
    status,
    setStatus,
    choices,
    needsNote: answer === "send-back" && note.trim() === "",
    body: () => ({
      answer,
      ...(answer === "send-back" ? { note } : {}),
      ...(answer === "accept" && status !== KEEP_STATUS ? { status } : {}),
    }),
  };
}
