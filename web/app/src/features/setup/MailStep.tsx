import { t } from "@bugaboo/messages";
import { Button, buttonLook, Card, partOf, part } from "@bugaboo/ui";

import { MailServerPanel } from "../notifications/index.ts";

export interface MailStepProps {
  readonly onFinish: () => void;
}

/**
 * The fourth step: the mail server, which can also be entered later (PRIV-08). E-mail stays off
 * until the administrator has entered the server and switched it on.
 */
export function MailStep({ onFinish }: MailStepProps) {
  return (
    <>
      <Card heading={t("first-run.mail.title")} level={1}>
        <p {...partOf(part.text)}>{t("first-run.mail.intro")}</p>
      </Card>
      <MailServerPanel saveLook={buttonLook.plain} />
      <Button look={buttonLook.primary} wide onPress={onFinish}>
        {t("common.continue")}
      </Button>
    </>
  );
}
