import { ApiError, messageKeyOf, type ErrorCode, type Problem } from "@bugaboo/api-client";
import { t, tFrom } from "@bugaboo/messages";
import { CancelledError } from "@tanstack/react-query";

import { listText } from "./list-text.ts";

/** The code of an error answer of the server, if the error is one. */
export function codeOf(error: unknown): ErrorCode | undefined {
  return error instanceof ApiError ? error.problem?.code : undefined;
}

/** The server refused the text as a search: it has a mistake in the search language (a search that takes too long is another answer). */
export function isInvalidSearch(error: unknown): boolean {
  return codeOf(error) === "search.invalid";
}

/** The seconds the server asks to wait before trying again, if the error is an answer that says so. */
export function retryAfterSecondsOf(error: unknown): number | undefined {
  return error instanceof ApiError ? error.retryAfterSeconds : undefined;
}

/** The field a refusal is about (the word of a built-in field or the ID of an own field), if it names one. */
export function fieldOf(error: unknown): string | undefined {
  return error instanceof ApiError ? (error.problem?.field ?? undefined) : undefined;
}

/**
 * What to tell the person about an error. The server names the message; one that this version
 * of the app does not know, or no answer at all, becomes "not available right now".
 */
export function messageOf(error: unknown): string {
  if (!(error instanceof ApiError)) {
    return tFrom(messageKeyOf(), {});
  }
  const message = tFrom(error.messageKey, paramsOf(error.problem));
  const named = namedBy(error.problem);
  return named === undefined ? message : `${message} ${named}`;
}

/** The members of an answer that are lists (the loop of dependencies, the kinds that have an effect). */
function listOf(problem: Readonly<Problem> | undefined, member: string): readonly unknown[] {
  const found: unknown = problem === undefined ? undefined : Reflect.get(problem, member);
  return Array.isArray(found) ? found : [];
}

/**
 * What a refusal names besides its message: a loop of dependencies by the issue IDs the person may
 * see (an issue they may not see is *an issue you can't see*), and the kinds of relation that have a
 * date effect already. Nothing for any other answer.
 */
function namedBy(problem: Readonly<Problem> | undefined): string | undefined {
  const loop = listOf(problem, "dependency_loop");
  if (loop.length > 0) {
    const issues = loop.map((each) =>
      typeof each === "string" ? each : t("related-issues.loop-hidden-issue"),
    );
    return t("related-issues.loop-is", { issues: listText(issues) });
  }
  const kinds = listOf(problem, "date_effect_kinds").filter(
    (each): each is string => typeof each === "string",
  );
  return kinds.length === 0
    ? undefined
    : t("relation-kinds.date-effect-kinds", { kinds: listText(kinds) });
}

/** The values the server sent for the placeholders of a message: the limits it names, numbers and words only. */
function paramsOf(
  problem: Readonly<Problem> | undefined,
): Readonly<Record<string, string | number>> {
  const known: Record<string, string | number> = {};
  for (const [name, value] of Object.entries(problem?.params ?? {})) {
    if (typeof value === "string" || typeof value === "number") {
      known[name] = value;
    }
  }
  return known;
}

/** How long the code for support is: the start of the ID of the request is enough to find it. */
const SUPPORT_CODE_LENGTH = 8;

/** The codes of the answers that a person may try again, because nothing they did was wrong. */
const RETRYABLE: ReadonlySet<ErrorCode> = new Set<ErrorCode>([
  "request.timed-out",
  "server.unavailable",
  "server.internal",
]);

/** Whether the failure is one the person can try again: a request that took too long, a server that could not answer, or no answer. */
export function isRetryable(error: unknown): boolean {
  if (!(error instanceof ApiError)) {
    return false;
  }
  return error.problem === undefined || RETRYABLE.has(error.problem.code);
}

/** The short code for support that ends a note about a failure of the server: the start of the ID of the request, none without one. */
export function supportCodeOf(error: unknown): string | undefined {
  const id = error instanceof ApiError ? error.problem?.request_id : undefined;
  return id === undefined || id === null || id === ""
    ? undefined
    : id.slice(0, SUPPORT_CODE_LENGTH);
}

/** Whether the answer says that the person has to set up a second step before anything else. */
export function isSecondStepRequired(error: unknown): boolean {
  return codeOf(error) === "auth.second-step-required";
}

/** Whether a password was refused for what it is: too short, or one of the passwords that are tried first. */
export function isPasswordProblem(error: unknown): boolean {
  const code = codeOf(error);
  return code === "auth.password-too-short" || code === "auth.password-breached";
}

/** Whether the answer says that nobody is signed in, or that the sign-in belongs to another page. */
export function sessionLostReason(error: unknown): SessionLostReason | undefined {
  const code = codeOf(error);
  return code === undefined ? undefined : lostReasons[code];
}

const lostReasons: Readonly<Partial<Record<ErrorCode, SessionLostReason>>> = {
  "auth.not-signed-in": "session-ended",
  "request.wrong-token": "wrong-token",
};

/** Why the person is taken to the sign-in page: the sign-in ended, or does not fit the page. */
export type SessionLostReason = "session-ended" | "wrong-token" | "signed-out";

/**
 * Whether an error is a request that did not give an answer: the server refused or was not there
 * (`ApiError`), or the question was cancelled before it was answered. Anything else is a fault in
 * the program, and is not for a handler that carries on without the answer to hide.
 */
export function isNoAnswer(error: unknown): boolean {
  return error instanceof ApiError || error instanceof CancelledError;
}
