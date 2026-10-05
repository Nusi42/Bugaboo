import { isMessageKey, type MessageKey } from "@bugaboo/messages";

import type { Schema } from "./readonly.ts";

/** The body of every error answer (`application/problem+json`), as the API describes it. */
export type Problem = Schema<"Problem">;

/** What can go wrong, as the API names it. */
export type ErrorCode = Schema<"ErrorCode">;

/** The content type of an error answer. */
export const PROBLEM_CONTENT_TYPE = "application/problem+json";

/** What people read when the server did not send a message this app knows. */
const FALLBACK_MESSAGE: MessageKey = "error.server.unavailable";

/**
 * Whether a value looks like the problem body of the API. The body comes from outside (a proxy
 * may answer with something else), so it is checked before it is believed.
 */
export function isProblem(value: unknown): value is Problem {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof value.code === "string" &&
    "message_key" in value &&
    typeof value.message_key === "string" &&
    "status" in value &&
    typeof value.status === "number" &&
    "title" in value &&
    typeof value.title === "string"
  );
}

/** The message to show for a problem: the key the server named, if this app has it. */
export function messageKeyOf(problem: Pick<Problem, "message_key"> | undefined): MessageKey {
  if (problem !== undefined && isMessageKey(problem.message_key)) {
    return problem.message_key;
  }
  return FALLBACK_MESSAGE;
}

/** A request that did not succeed: the server refused it, or no sensible answer came. */
export class ApiError extends Error {
  /** The HTTP status; `0` when no answer came at all. */
  readonly status: number;
  /** The problem the server sent, if it sent one. */
  readonly problem: Readonly<Problem> | undefined;

  constructor(status: number, problem: Readonly<Problem> | undefined) {
    super(problem?.code ?? `http ${String(status)}`);
    this.name = "ApiError";
    this.status = status;
    this.problem = problem;
  }

  /** The key of the message to show to the person. */
  get messageKey(): MessageKey {
    return messageKeyOf(this.problem);
  }
}
