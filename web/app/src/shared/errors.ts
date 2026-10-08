import { ApiError, messageKeyOf, type ErrorCode } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { CancelledError } from "@tanstack/react-query";

/** The code of an error answer of the server, if the error is one. */
export function codeOf(error: unknown): ErrorCode | undefined {
  return error instanceof ApiError ? error.problem?.code : undefined;
}

/**
 * What to tell the person about an error. The server names the message; one that this version
 * of the app does not know, or no answer at all, becomes "not available right now".
 */
export function messageOf(error: unknown): string {
  return t(error instanceof ApiError ? error.messageKey : messageKeyOf());
}

/** Whether the answer says that the person has to set up a second step before anything else. */
export function isSecondStepRequired(error: unknown): boolean {
  return codeOf(error) === "auth.second_step_required";
}

/** Whether a password was refused for what it is: too short, or one of the passwords that are tried first. */
export function isPasswordProblem(error: unknown): boolean {
  const code = codeOf(error);
  return code === "auth.password_too_short" || code === "auth.password_breached";
}

/** Whether the answer says that nobody is signed in, or that the sign-in belongs to another page. */
export function sessionLostReason(error: unknown): SessionLostReason | undefined {
  const code = codeOf(error);
  return code === undefined ? undefined : lostReasons[code];
}

const lostReasons: Readonly<Partial<Record<ErrorCode, SessionLostReason>>> = {
  "auth.not_signed_in": "session-ended",
  "request.wrong_token": "wrong-token",
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
