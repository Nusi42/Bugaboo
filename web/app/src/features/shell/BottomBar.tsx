import { t } from "@bugaboo/messages";
import { Icon, iconName, type IconName, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Button, Link } from "react-aria-components";

import { paths, type Path } from "../../shared/paths.ts";
import { openOverlay, overlays, useOverlayOpen, type OverlayName } from "./overlay-state.ts";
import { useUnreadCountText } from "../../shared/unread-notifications.ts";
import { isCurrentPlace, placeAt } from "../../shared/places.ts";

/** Words of the ARIA standard, not text for people. */
const currentPage = "page";

interface TabProps {
  readonly icon: IconName;
  readonly label: string;
  /** A number after the words, such as the notifications not read yet. */
  readonly count?: string | undefined;
}

/** The icon with its words under it: a button of the bar. The words come first in the page, so a
 * screen reader says *Notifications 3*; the icon is drawn above them. */
function Tab({ icon, label, count }: TabProps) {
  return (
    <>
      <span {...partOf(part.bottomBarLabel)}>{label}</span>
      <span {...partOf(part.bottomBarIcon)}>
        <Icon name={icon} />
        {count === undefined ? null : <span {...partOf(part.bottomBarCount)}>{count}</span>}
      </span>
    </>
  );
}

/** A link of the bar to a page of the app. */
function PlaceTab({ path, count }: { readonly path: Path; readonly count?: string | undefined }) {
  const { pathname } = useLocation();
  const place = placeAt(path);
  if (place === undefined) {
    return null;
  }
  return (
    <li {...partOf(part.bottomBarTab)}>
      <Link
        {...partOf(part.bottomBarItem)}
        href={place.path}
        {...(isCurrentPlace(place.path, pathname) ? { "aria-current": currentPage } : {})}
      >
        <Tab icon={place.icon} label={t(place.name)} count={count} />
      </Link>
    </li>
  );
}

interface OverlayTabProps extends TabProps {
  /** The layer the button opens. */
  readonly opens: OverlayName;
  /** The layer is open now (for screen readers). */
  readonly expanded?: boolean;
}

/** A button of the bar that opens a layer over the page. */
function OverlayTab({ icon, label, opens, expanded }: OverlayTabProps) {
  return (
    <li {...partOf(part.bottomBarTab)}>
      <Button
        {...partOf(part.bottomBarItem)}
        {...(expanded === undefined ? {} : { "aria-expanded": expanded })}
        onPress={() => {
          openOverlay(opens);
        }}
      >
        <Tab icon={icon} label={label} />
      </Button>
    </li>
  );
}

/**
 * The bar at the bottom of a phone, with words under the icons (UX §3.3): *Home*, *Search*, *New*
 * and *More*. *Notifications* with the number not read yet; *More* holds the rest.
 */
export function BottomBar() {
  const moreOpen = useOverlayOpen("more");
  const unread = useUnreadCountText();
  return (
    <nav {...partOf(part.bottomBar)} aria-label={t("shell.menu")}>
      <ul {...partOf(part.bottomBarTabs)}>
        <PlaceTab path={paths.home} />
        <OverlayTab icon={iconName.search} label={t("shell.search")} opens={overlays.search} />
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
