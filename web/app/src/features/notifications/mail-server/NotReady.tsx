import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";

import { useMailServerStatus } from "./use-mail-server.ts";

/** Says why e-mail cannot go while it is switched on: the server's own judgement, shown as it is. */
export function NotReady({ enabled }: { readonly enabled: boolean }) {
  const readiness = useMailServerStatus().data?.readiness;
  if (!enabled || readiness === undefined || readiness === "ready") {
    return null;
  }
  return (
    <Alert tone={alertTone.warning}>
      {readiness === "incomplete" ? t("error.mail.off") : t("error.mail.insecure")}
    </Alert>
  );
}
