import { t } from "@bugaboo/messages";
import { cx, Icon, iconName, type IconName } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Button, Link } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import styles from "./BottomBar.module.css";
import { openOverlay, useOverlayOpen } from "./overlay-state.ts";
import { placeAt } from "./places.ts";

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
      <span className={styles.label}>{label}</span>
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
    <nav className={styles.bar} aria-label={t("shell.menu")}>
      <ul className={styles.tabs}>
        {home === undefined ? null : (
          <li className={styles.tab}>
            <Link
              className={cx(styles.item)}
              href={home.path}
              {...(pathname === home.path ? { "aria-current": currentPage } : {})}
            >
              <Tab icon={home.icon} label={t(home.name)} />
            </Link>
          </li>
        )}
        <li className={styles.tab}>
          <Button
            className={cx(styles.item)}
            onPress={() => {
              openOverlay("search");
            }}
          >
            <Tab icon={iconName.search} label={t("shell.search")} />
          </Button>
        </li>
        <li className={styles.tab}>
          <Button
            className={cx(styles.item)}
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
