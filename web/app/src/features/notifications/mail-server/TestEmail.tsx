import { testEmailEverySeconds } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { codeOf, messageOf } from "../../../shared/errors.ts";
import { useCooldown } from "../../../shared/use-cooldown.ts";
import { useMe } from "../../../shared/use-me.ts";
import { useSendTestEmail } from "./use-mail-server.ts";

/**
 * The answer of the mail server to the test e-mail, in plain words: it went; it was sent too soon
 * after the one before; or why it failed, with what to check.
 */
function Answer({
  sent,
  error,
  email,
}: {
  readonly sent: boolean;
  readonly error: unknown;
  readonly email: string;
}) {
  if (sent) {
    return <Alert tone={alertTone.success}>{t("mail-server.test-done", { email })}</Alert>;
  }
  if (error === null || error === undefined) {
    return null;
  }
  if (codeOf(error) === "rate_limit.exceeded") {
    return (
      <Alert tone={alertTone.warning}>
        {t("mail-server.test-wait", { seconds: testEmailEverySeconds })}
      </Alert>
    );
  }
  return <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>;
}

/**
 * *Send me a test e-mail*: Bugaboo sends one e-mail to the person who asks, through the mail server
 * as it is saved, and tells what the mail server answered. A person can send one every few seconds.
 */
export function TestEmail() {
  const email = useMe()?.email ?? "";
  const send = useSendTestEmail();
  const cooldown = useCooldown(testEmailEverySeconds);
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.text)}>{t("mail-server.test-intro", { email })}</p>
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.primary}
          isPending={send.isPending}
          isDisabled={cooldown.secondsLeft > 0}
          onPress={() => {
            cooldown.start();
            send.mutate({});
          }}
        >
          {t("mail-server.test-send")}
        </Button>
        {cooldown.secondsLeft > 0 ? (
          <span {...partOf(part.muted)}>
            {t("mail-server.test-wait", { seconds: cooldown.secondsLeft })}
          </span>
        ) : null}
      </div>
      <Answer sent={send.isSuccess} error={send.error} email={email} />
    </div>
  );
}
