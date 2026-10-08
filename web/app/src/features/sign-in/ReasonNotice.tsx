import { t, type MessageKey } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";

import { type SignInReason } from "./reasons.ts";

const reasonMessages: Readonly<Record<SignInReason, MessageKey>> = {
  "signed-out": "sign-in.signed-out",
  "session-ended": "error.auth.not-signed-in",
  "wrong-token": "error.request.wrong-token",
};

/** Says why the person is on the sign-in page: they signed out, or the sign-in ended or does not fit. */
export function ReasonNotice({ reason }: { readonly reason: SignInReason }) {
  return (
    <Alert tone={reason === "signed-out" ? alertTone.success : alertTone.warning}>
      {t(reasonMessages[reason])}
    </Alert>
  );
}
