import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { agentWorkKeys } from "../../shared/agent-work-keys.ts";
import { api } from "../../shared/api.ts";
import { formatDecimal } from "../../shared/format-decimal.ts";
import { itemsOf, pageSize, usePaged } from "../../shared/paged.ts";
import { refreshHistory, refreshIssues, refreshRoutes } from "../../shared/query-keys.ts";

export type IssueAgentWork = Schema<"IssueAgentWorkBody">;
export type Work = Schema<"WorkBody">;
export type Attempt = Schema<"AttemptBody">;
export type BriefLack = Schema<"BriefPart">;
export type Power = Schema<"HandOverPower">;
export type Verdict = Schema<"ReviewAnswer">;
export type Proposal = Schema<"ProposalBody">;

/** The agent work of an issue: who works on it, the work, and what waits for a person. */
export function useIssueAgentWork(issue: string, enabled = true) {
  return api.query.useQuery(
    "get",
    "/api/v1/issues/{issue}/agent-work",
    { params: { path: { issue } } },
    { enabled },
  );
}

/** Agent work changed: what an issue shows of it, its history and the lists of issues are read again. */
export function refreshAgentWork(): void {
  refreshRoutes(["/api/v1/issues/{issue}/agent-work"]);
  refreshHistory();
  refreshIssues();
}

/** The agents of the person, a page at a time, for the choice in the window of a hand-over. */
export function useMyAgents() {
  const list = usePaged({
    queryKey: agentWorkKeys.myAgents(),
    read: (after) =>
      api.client.GET("/api/v1/agents", {
        params: { query: { limit: pageSize.largest, ...after } },
      }),
  });
  return { ...list, agents: itemsOf(list.data?.pages) };
}

/** Hands issues over to an agent. */
export function useHandOver(onHandedOver?: () => void, onFailed?: (error: unknown) => void) {
  // The window closes when it is confirmed, so what is done afterwards is told to the mutation
  // itself: the callbacks of one call go with the window.
  return api.query.useMutation("post", "/api/v1/hand-overs", {
    onSuccess: () => {
      refreshAgentWork();
      onHandedOver?.();
    },
    onError: (error: unknown) => {
      onFailed?.(error);
    },
  });
}

/** Answers a question of an agent. */
export function useAnswerQuestion() {
  return api.query.useMutation("post", "/api/v1/agent-work/{work}/answer", {
    onSuccess: refreshAgentWork,
  });
}

/** *Approve plan*, or a note instead. */
export function useApprovePlan() {
  return api.query.useMutation("post", "/api/v1/agent-work/{work}/approve-plan", {
    onSuccess: refreshAgentWork,
  });
}

/** The review of a result: accept, send back, throw away. */
export function useReviewWork(onFailed?: (error: unknown) => void) {
  // The window closes when it is confirmed, so a refusal is told to the mutation itself.
  return api.query.useMutation("post", "/api/v1/agent-work/{work}/review", {
    onSuccess: refreshAgentWork,
    onError: (error: unknown) => {
      onFailed?.(error);
    },
  });
}

/** Marks work that was accepted by itself as looked at. */
export function useLookAtWork() {
  return api.query.useMutation("post", "/api/v1/agent-work/{work}/looked-at", {
    onSuccess: refreshAgentWork,
  });
}

/** Applies or discards a proposal of a standing rule. */
export function useDecideProposal() {
  return api.query.useMutation("post", "/api/v1/proposals/{proposal}/decision", {
    onSuccess: refreshAgentWork,
  });
}

/** The parts of a brief an issue lacks, in words: *No steps to reproduce*. */
export function missingText(lack: BriefLack): string {
  return t(`agent-work.missing-${lack}`);
}

/** The work that waits for a person to review it: an attempt is ready. */
export function isReadyForReview(work: Work): boolean {
  return work.state === "ready-for-review" && work.open;
}

/** The newest attempt of a piece of work, if the agent made one. */
export function lastAttempt(work: Work): Attempt | undefined {
  return work.attempts.at(-1);
}

/** Cents as a number with two places, in the person's way of writing numbers. */
export function centsText(cents: number): string {
  return formatDecimal(cents / 100, 2);
}

/** An attempt that has a run: its number and the run, which lists what the attempt changed. */
export interface AttemptRun {
  readonly number: number;
  readonly run: string;
}

/** The runs of the attempts, newest first: what *Throw away* undoes, one step each. */
export function runsOf(work: Work): readonly AttemptRun[] {
  return work.attempts
    .toReversed()
    .flatMap((attempt) =>
      attempt.run === null || attempt.run === undefined
        ? []
        : [{ number: attempt.number, run: attempt.run }],
    );
}

/** Whether a plan waits for a person: approval is asked for, the plan is there, nothing else waits first. */
export function planWaitsForYou(work: Work): boolean {
  const attempt = lastAttempt(work);
  const asking = work.questions.some(
    (question) => question.answer === null || question.answer === undefined,
  );
  return (
    work.open &&
    work.budget.plan_needs_approval &&
    !asking &&
    attempt !== undefined &&
    attempt.plan !== null &&
    attempt.plan !== undefined &&
    !attempt.plan_approved &&
    (attempt.ended_at === null || attempt.ended_at === undefined)
  );
}
