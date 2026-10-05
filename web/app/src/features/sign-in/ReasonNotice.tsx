import { t, type MessageKey } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";

import { type SignInReason } from "./reasons.ts";

const reasonMessages: Readonly<Record<SignInReason, MessageKey>> = {
  "signed-out": "sign-in.signed-out",
  "session-ended": "error.auth.not-signed-in",
  "wrong-token": "error.request.wrong-token",
};

/** Says why the person is on the sign-in page: they signed out, or the sign-in ended or does not fit. */
export function ReasonNotice({ reason }: { readonly reason: SignInReason }) {
  const signedOut = reason === "signed-out";
  return (
    <Alert success={signedOut} warning={!signedOut}>
      {t(reasonMessages[reason])}
    </Alert>
  );
}
