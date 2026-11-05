import { t } from "@bugaboo/messages";
import { iconName, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { overlays, useOverlayOpen } from "./overlay-state.ts";
import { useUnreadCountText } from "../../shared/unread-notifications.ts";
import { OverlayTab, PlaceTab } from "./BottomBarTabs.tsx";
import { timePlace } from "./time-parts.ts";
import { TimeSlot } from "./TimeSlot.tsx";

/**
 * The bar at the bottom of a phone, with words under the icons (UX §3.3): *Home*, *Search*, *New*
 * and *More*. *Notifications* with the number not read yet; *More* holds the rest.
 */
export function BottomBar() {
  const moreOpen = useOverlayOpen("more");
  const unread = useUnreadCountText();
  return (
    <nav {...partOf(part.bottomBar)} aria-label={t("shell.menu")}>
      <TimeSlot place={timePlace.bar} />
      <ul {...partOf(part.bottomBarTabs)}>
        <PlaceTab path={paths.home} />
        <OverlayTab
          icon={iconName.search}
          label={t("shell.search")}
          opens={overlays.search}
          place={paths.search}
        />
        <OverlayTab icon={iconName.plus} label={t("issues.new")} opens={overlays.newIssue} />
        <PlaceTab path={paths.notifications} count={unread} />
        <OverlayTab
          icon={iconName.more}
          label={t("shell.more")}
          opens={overlays.more}
          expanded={moreOpen}
        />
      </ul>
    </nav>
  );
}
