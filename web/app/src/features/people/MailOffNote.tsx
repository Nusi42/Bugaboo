import { t } from "@bugaboo/messages";
import { Alert, alertTone, Link, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { paths } from "../../shared/paths.ts";
import { statusOptions } from "../../shared/session.ts";
import { usePermissions } from "../../shared/use-me.ts";
import { mayOpen } from "../shell/index.ts";

/**
 * Signing up without an invitation needs e-mail: while it is off the page for signing up says it is
 * closed, so the switch says so too, with the way to set e-mail up for whoever may.
 */
export function MailOffNote({ enabled }: { readonly enabled: boolean }) {
  const open = useQuery(statusOptions()).data?.sign_up_open;
  const permissions = usePermissions();
  if (!enabled || open !== false) {
    return null;
  }
  return (
    <Alert tone={alertTone.warning}>
      <div {...partOf(part.column)}>
        <span>{t("sign-up-policy.mail-off")}</span>
        {mayOpen(paths.mailServer, permissions) ? (
          <Link href={paths.mailServer}>{t("sign-up-policy.mail-link")}</Link>
        ) : null}
      </div>
    </Alert>
  );
}
