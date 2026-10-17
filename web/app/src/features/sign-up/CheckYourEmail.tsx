import { secondsPerMinute, signUpLinkHours, signUpResendMinutes } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, Link, partOf, part } from "@bugaboo/ui";
import { useEffect } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { useCooldown } from "../../shared/use-cooldown.ts";
import { usePageTitle } from "../../shared/use-page.ts";

export interface CheckYourEmailProps {
  /** The address the link was sent to. */
  readonly email: string;
  /** *Use another address*: back to the form. */
  readonly onOtherAddress: () => void;
}

/**
 * What a person sees after asking for a link (AUTH-06, M4-F62): look at your e-mail, with a way to
 * ask again. The server sends one e-mail to an address in a few minutes, so the button waits for
 * that time; the sign-up route has a limit of its own, which an answer of the server tells.
 */
export function CheckYourEmail({ email, onOtherAddress }: CheckYourEmailProps) {
  usePageTitle(t("sign-up.check-title"));
  const wait = useCooldown(signUpResendMinutes * secondsPerMinute);
  const { start } = wait;
  // The link that was just sent starts the wait for the next one.
  useEffect(() => {
    start();
  }, [start]);
  const again = api.query.useMutation("post", "/api/v1/auth/sign-up", { onSuccess: start });
  const minutes = Math.ceil(wait.secondsLeft / secondsPerMinute);
  return (
    <Card heading={t("sign-up.check-title")} level={1}>
      <p {...partOf(part.text)}>{t("sign-up.check-text", { email, hours: signUpLinkHours })}</p>
      <p {...partOf(part.muted)}>{t("sign-up.check-nothing")}</p>
      {again.isSuccess ? (
        <Alert tone={alertTone.success}>{t("sign-up.resent", { email })}</Alert>
      ) : null}
      {again.isError ? <Alert tone={alertTone.danger}>{messageOf(again.error)}</Alert> : null}
      <div {...partOf(part.row)}>
        <Button
          isPending={again.isPending}
          isDisabled={wait.secondsLeft > 0}
          onPress={() => {
            again.mutate({ body: { email } });
          }}
        >
          {t("sign-up.resend")}
        </Button>
        <Button onPress={onOtherAddress}>{t("sign-up.other-address")}</Button>
      </div>
      {wait.secondsLeft > 0 ? (
        <p {...partOf(part.muted)}>{t("sign-up.resend-in", { minutes })}</p>
      ) : null}
      <Link href={paths.signIn}>{t("sign-up.have-account")}</Link>
    </Card>
  );
}
