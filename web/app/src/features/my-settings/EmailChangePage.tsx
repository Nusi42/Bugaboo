import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { useEffect, useRef } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { refreshPerson } from "../../shared/session.ts";
import { useMe } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";

/**
 * Opens the link once the page shows, and says what came of it. The link works once, so it is sent
 * once even when the page is drawn twice (the double run of React's strict mode). A person who is
 * signed in sees their new address from then on.
 */
function Confirm({ secret }: { readonly secret: string }) {
  const confirm = api.query.useMutation("post", "/api/v1/auth/e-mail-change/confirm", {
    onSuccess: () => {
      void refreshPerson();
    },
  });
  const { mutate } = confirm;
  const sent = useRef(false);
  useEffect(() => {
    if (sent.current) {
      return;
    }
    sent.current = true;
    mutate({ body: { secret } });
  }, [mutate, secret]);
  if (confirm.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(confirm.error)}</Alert>;
  }
  return confirm.isSuccess ? (
    <Alert tone={alertTone.success}>{t("profile.confirmed")}</Alert>
  ) : null;
}

/**
 * The page the link to a new e-mail address opens (PRIV-06): the secret is in the part of the
 * address after the hash sign, so it never reaches a log. It works with or without a session; only
 * a person who is not signed in is offered *Sign in*.
 */
export function EmailChangePage() {
  usePageTitle(t("profile.confirm-title"));
  const secret = useLocation().hash.replace(/^#/u, "");
  const signedIn = useMe() !== undefined;
  return (
    <Card heading={t("profile.confirm-title")} level={1}>
      {secret === "" ? (
        <Alert tone={alertTone.warning}>{t("profile.incomplete")}</Alert>
      ) : (
        <Confirm secret={secret} />
      )}
      {signedIn ? null : <Link href={paths.signIn}>{t("profile.sign-in")}</Link>}
    </Card>
  );
}
