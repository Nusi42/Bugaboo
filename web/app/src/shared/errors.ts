import { ApiError, messageKeyOf, type ErrorCode } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { CancelledError } from "@tanstack/react-query";

/** The code of an error answer of the server, if the error is one. */
export function codeOf(error: unknown): ErrorCode | undefined {
  return error instanceof ApiError ? error.problem?.code : undefined;
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
  return t(error instanceof ApiError ? error.messageKey : messageKeyOf());
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
