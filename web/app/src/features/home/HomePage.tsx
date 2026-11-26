import { t } from "@bugaboo/messages";
import { Card, HoldWhileLoading, partOf, part, ReadingHold } from "@bugaboo/ui";
import { lazyRouteComponent } from "@tanstack/react-router";
import { Suspense } from "react";

import { paths } from "../../shared/paths.ts";
import { useMe, usePermissions } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { mayOpen } from "../shell/index.ts";

/** The boxes of the person's own dashboard, loaded with the dashboards when the start page is first shown. */
const HomeBoxes = lazyRouteComponent(() => import("../dashboards/index.ts"), "HomeBoxes");

/** *Notifications* of the start page, loaded when the start page is first shown, so the first page stays small. */
const NotificationsBlock = lazyRouteComponent(
  () => import("../notifications/index.ts"),
  "NotificationsBlock",
);

/** The start page of somebody who is signed in (UX §4.1): a greeting, a hint to look after the server for
 * those who may, the newest unread notifications as a fixed block, and below them the person's own dashboard, whose
 * *Customize* is its *Change* (ADR-0050, decision 6). */
export function HomePage() {
  const me = useMe();
  const permissions = usePermissions();
  const title = t("home.title", { name: me?.name ?? "" });
  usePageTitle(title);
  // The greeting is never held. The notifications and the boxes stand under it, each held until its own reads are in,
  // so nothing pushes what stands under it (UX §6.4). The timer of a phone is the box *My timer* (ADR-0050, decision 6).
  return (
    <>
      <Card heading={title} level={1}>
        {mayOpen(paths.organizationSettings, permissions) ? (
          <p {...partOf(part.text)}>{t("home.admin")}</p>
        ) : null}
      </Card>
      <ReadingHold>
        <Suspense fallback={<HoldWhileLoading />}>
          <NotificationsBlock />
        </Suspense>
      </ReadingHold>
      <ReadingHold>
        <Suspense fallback={<HoldWhileLoading />}>
          <HomeBoxes />
        </Suspense>
      </ReadingHold>
    </>
  );
}
