import { t } from "@bugaboo/messages";
import { Card, Link } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";

/** The head of *Set up the assistant*: the title of the page, what it is for, and the way to the guide to AI (COMP-11). */
export function AiIntro() {
  usePageTitle(t("ai-setup.title"));
  return (
    <Card heading={t("ai-setup.title")} level={1} intro={t("ai-setup.intro")}>
      <Link href={paths.aiGuide}>{t("ai-setup.guide-link")}</Link>
    </Card>
  );
}
