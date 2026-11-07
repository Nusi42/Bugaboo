import { t } from "@bugaboo/messages";
import { Card, cardLook, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../../shared/use-page.ts";
import { AppList } from "./AppList.tsx";

/** *Connected apps* under *My settings*: the AI apps the person allowed, and *Take back*. */
export function ConnectedAppsPage() {
  usePageTitle(t("connected-apps.title"));
  return (
    <div {...partOf(part.stack)}>
      <Card
        heading={t("connected-apps.title")}
        level={1}
        look={cardLook.pageHead}
        intro={t("connected-apps.intro")}
      />
      <AppList />
    </div>
  );
}
