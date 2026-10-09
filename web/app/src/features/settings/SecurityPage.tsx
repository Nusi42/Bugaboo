import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { OrganizationTopic } from "./OrganizationTopic.tsx";

/** The settings that belong to *Security*: what text written by people may show (SEC-13). */
const TOPICS = "content.";

/** *Organization settings › System › Security* (SEC-13, ADR-0018). */
export function SecurityPage() {
  usePageTitle(t("nav.organization-security"));
  return (
    <Card heading={t("nav.organization-security")} level={1}>
      <p {...partOf(part.muted)}>{t("setting.page.security-intro")}</p>
      <OrganizationTopic topics={TOPICS} />
    </Card>
  );
}
