import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { OrganizationTopic } from "./OrganizationTopic.tsx";

/** The settings that belong to *Themes* (UX §5.1): the organization's defaults of how Bugaboo looks. */
const TOPICS = "appearance.";

/** *Organization settings › Look › Themes* (THM-02, CFG-06): the organization's defaults, which an administrator can lock. */
export function ThemesPage() {
  usePageTitle(t("nav.themes"));
  return (
    <Card heading={t("nav.themes")} level={1}>
      <p {...partOf(part.muted)}>{t("setting.page.themes-intro")}</p>
      <OrganizationTopic topics={TOPICS} />
    </Card>
  );
}
