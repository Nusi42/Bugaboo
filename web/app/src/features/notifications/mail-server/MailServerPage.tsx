import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { usePageTitle } from "../../../shared/use-page.ts";
import { MailServerPanel } from "./MailServerPanel.tsx";

/**
 * *Organization settings › Connections › E-mail & notifications* (NTF-02, PRIV-08): the mail server,
 * the test e-mail, and whether e-mail is on.
 */
export function MailServerPage() {
  usePageTitle(t("mail-server.title"));
  return (
    <Card heading={t("mail-server.title")} level={1} intro={t("mail-server.intro")}>
      <MailServerPanel />
    </Card>
  );
}
