import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { OrganizationTopic } from "./OrganizationTopic.tsx";

/** The settings that belong to *Themes* (UX §5.1): the organization's defaults of how Bugaboo looks. */
const TOPICS = "appearance.";

/** *Organization settings › Look › Themes* (THM-02, CFG-06): the organization's defaults, which an administrator can lock. */
export function ThemesPage() {
  usePageTitle(t("nav.themes"));
  return (
    <Card heading={t("nav.themes")} level={1} intro={t("setting.page.themes-intro")}>
      <OrganizationTopic topics={TOPICS} />
    </Card>
  );
}
