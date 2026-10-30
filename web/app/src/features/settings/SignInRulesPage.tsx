import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { OrganizationTopic } from "./OrganizationTopic.tsx";

/** The settings that belong to *Sign-in rules* (UX §5.1): how long people stay signed in and who must use two-step sign-in. */
const TOPICS = "security.,auth.access-key-";

/** *Organization settings › People › Sign-in rules* (CFG-06, ADR-0018). */
export function SignInRulesPage() {
  usePageTitle(t("nav.sign-in-rules"));
  return (
    <Card heading={t("nav.sign-in-rules")} level={1} intro={t("setting.page.sign-in-rules-intro")}>
      <OrganizationTopic topics={TOPICS} />
    </Card>
  );
}
