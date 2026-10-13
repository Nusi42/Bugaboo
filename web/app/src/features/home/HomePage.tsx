import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { useMe, usePermissions } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { MentionsBlock } from "../mentions/index.ts";
import { mayOpen } from "../shell/index.ts";

/** The start page of somebody who is signed in. The rest of Bugaboo arrives with later versions. */
export function HomePage() {
  const me = useMe();
  const permissions = usePermissions();
  const title = t("home.title", { name: me?.display_name ?? "" });
  usePageTitle(title);
  return (
    <>
      <Card heading={title} level={1}>
        <p {...partOf(part.text)}>{t("home.intro")}</p>
        {mayOpen(paths.organizationSettings, permissions) ? (
          <p {...partOf(part.text)}>{t("home.admin")}</p>
        ) : null}
      </Card>
      <MentionsBlock />
    </>
  );
}
