import type { Schema } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { refreshRoutes } from "../../shared/query-keys.ts";

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

/**
 * The most turns a question carries and the most characters of a question and of a turn. The
 * server's own limits (`bb_core::ai`): a longer one is refused, so the panel keeps within them.
 */
export const limits = { turns: 20, question: 4000, turn: 8000 } as const;

/**
 * Whether the assistant is there at the place, and where its answers come from. The place is a
 * project, or none for a page of the organization. While the answer is not known, and when the
 * server refuses (a project the person may not see, or one kept from AI), nothing of the assistant
 * is shown.
 */
export function useAssistantStatus(project: string | undefined): AssistantStatus | undefined {
  const query = useQuery({
    ...api.query.queryOptions("get", "/api/v1/assistant", {
      params: { query: project === undefined ? {} : { project } },
    }),
    retry: false,
  });
  return query.data;
}

/** AI was switched on or off, or its setup changed: the places ask again whether the assistant is there. */
export function refreshAssistant(): void {
  refreshRoutes(["/api/v1/assistant"]);
}

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
