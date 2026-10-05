/** Why the person is on the sign-in page, when it is not their own doing. */
export type SignInReason = "signed-out" | "session-ended" | "wrong-token";

const reasons: ReadonlySet<unknown> = new Set(["signed-out", "session-ended", "wrong-token"]);

/** Whether a value from the address is one of the reasons. */
export function isSignInReason(value: unknown): value is SignInReason {
  return reasons.has(value);
}
