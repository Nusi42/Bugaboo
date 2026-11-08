import { assistantLimits, type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";

/** Whether the assistant is there at a place, and where its answers come from. */
export type AssistantStatus = Schema<"AssistantStatusBody">;
/** What the assistant answers: the text, the same written safely, and the changes it proposes. */
export type AssistantAnswer = Schema<"AssistantAnswerBody">;
/** A change the assistant proposes; nothing is saved until the person applies it. */
export type Proposal = Schema<"AssistantProposalBody">;
/** What *Apply* saved: the fields that were changed, with the value before and after. */
export type AppliedProposal = Schema<"AppliedProposalBody">;
/** The fields a proposal would change, or a change has changed. */
export type ProposedChange = Schema<"ProposedChangeBody">;

/** The server's limits of a question, so the panel keeps within them: a longer one is refused. */
export const limits = assistantLimits;

/** Asks the assistant a question about what the panel looks at. */
export function useAsk() {
  return api.query.useMutation("post", "/api/v1/assistant/questions");
}

/** Saves a change the assistant proposed, as the person. */
export function useApply() {
  return api.query.useMutation("post", "/api/v1/assistant/apply");
}

/** Which built-in fields and projects are kept from AI; every person who is signed in may read it. */
export function useNeverSent() {
  return api.query.useQuery("get", "/api/v1/never-sent-to-ai");
}
