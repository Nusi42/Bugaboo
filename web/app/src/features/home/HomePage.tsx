import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";
import { lazyRouteComponent } from "@tanstack/react-router";

import { paths } from "../../shared/paths.ts";
import { useMe, usePermissions } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { mayOpen } from "../shell/index.ts";

/** *Notifications* of the start page, loaded when the start page is first shown, so the first page stays small. */
const NotificationsBlock = lazyRouteComponent(
  () => import("../notifications/index.ts"),
  "NotificationsBlock",
);

/** The start page of somebody who is signed in (UX §4.1): a greeting, a hint to look after the server for
 * those who may, and the newest unread notifications. */
export function HomePage() {
  const me = useMe();
  const permissions = usePermissions();
  const title = t("home.title", { name: me?.name ?? "" });
  usePageTitle(title);
  return (
    <>
      <Card heading={title} level={1}>
        {mayOpen(paths.organizationSettings, permissions) ? (
          <p {...partOf(part.text)}>{t("home.admin")}</p>
        ) : null}
      </Card>
      <NotificationsBlock />
    </>
  );
}
