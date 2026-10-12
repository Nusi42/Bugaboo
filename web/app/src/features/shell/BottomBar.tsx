import { t } from "@bugaboo/messages";
import { Icon, iconName, type IconName, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Button, Link } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import { openOverlay, overlays, useOverlayOpen, type OverlayName } from "./overlay-state.ts";
import { placeAt } from "../../shared/places.ts";

/** Words of the ARIA standard, not text for people. */
const currentPage = "page";

interface TabProps {
  readonly icon: IconName;
  readonly label: string;
}

/** The icon with its words under it: a button of the bar. */
function Tab({ icon, label }: TabProps) {
  return (
    <>
      <Icon name={icon} />
      <span {...partOf(part.bottomBarLabel)}>{label}</span>
    </>
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
 * and *More*. *Notifications* joins when that screen exists (M4); *More* holds the rest.
 */
export function BottomBar() {
  const { pathname } = useLocation();
  const home = placeAt(paths.home);
  const moreOpen = useOverlayOpen("more");
  return (
    <nav {...partOf(part.bottomBar)} aria-label={t("shell.menu")}>
      <ul {...partOf(part.bottomBarTabs)}>
        {home === undefined ? null : (
          <li {...partOf(part.bottomBarTab)}>
            <Link
              {...partOf(part.bottomBarItem)}
              href={home.path}
              {...(pathname === home.path ? { "aria-current": currentPage } : {})}
            >
              <Tab icon={home.icon} label={t(home.name)} />
            </Link>
          </li>
        )}
        <OverlayTab icon={iconName.search} label={t("shell.search")} opens={overlays.search} />
        <OverlayTab icon={iconName.plus} label={t("issues.new")} opens={overlays.newIssue} />
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
