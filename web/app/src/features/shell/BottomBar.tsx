import { t } from "@bugaboo/messages";
import { Icon, iconName, type IconName, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Button, Link } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import { openOverlay, useOverlayOpen } from "./overlay-state.ts";
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

/**
 * The bar at the bottom of a phone, with words under the icons (UX §3.3): *Home*, *Search* and
 * *More*. *New* and *Notifications* join when those screens exist (M3, M4); *More* holds the rest.
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
        <li {...partOf(part.bottomBarTab)}>
          <Button
            {...partOf(part.bottomBarItem)}
            onPress={() => {
              openOverlay("search");
            }}
          >
            <Tab icon={iconName.search} label={t("shell.search")} />
          </Button>
        </li>
        <li {...partOf(part.bottomBarTab)}>
          <Button
            {...partOf(part.bottomBarItem)}
            aria-expanded={moreOpen}
            onPress={() => {
              openOverlay("more");
            }}
          >
            <Tab icon={iconName.more} label={t("shell.more")} />
          </Button>
        </li>
      </ul>
    </nav>
  );
}
