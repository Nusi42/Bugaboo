import { t } from "@bugaboo/messages";
import { Button, cx, Icon, iconName, type LayoutMode } from "@bugaboo/ui";
import { useCanGoBack, useLocation, useNavigate, useRouter } from "@tanstack/react-router";
import { Link } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import { openOverlay } from "./overlay-state.ts";
import { placeAt, trailTo } from "./places.ts";
import styles from "./TopBar.module.css";

/** Words of the ARIA standard, not text for people. */
const currentPage = "page";

/** *Back* on a phone: where the person came from, or else the screen this one belongs under. */
function useGoBack(): () => void {
  const router = useRouter();
  const canGoBack = useCanGoBack();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  return () => {
    const parent = placeAt(pathname)?.parent;
    if (canGoBack) {
      router.history.back();
    } else {
      void navigate({ to: parent ?? paths.home });
    }
  };
}

/** Where you are, as links up to the start page: `My settings › Appearance`. */
function Trail({ pathname }: { readonly pathname: string }) {
  const trail = trailTo(pathname);
  // The start page is where every trail begins, so it is only named when it is the page itself.
  const shown = trail.length > 1 ? trail.slice(1) : trail;
  return (
    <nav aria-label={t("shell.trail")} className={styles.trail}>
      <ol className={styles.crumbs}>
        {shown.map((place, index) => {
          const last = index === shown.length - 1;
          return (
            <li key={place.path} className={styles.crumb}>
              {last ? (
                <span aria-current={currentPage} className={styles.here}>
                  {t(place.name)}
                </span>
              ) : (
                <>
                  <Link href={place.path} className={cx(styles.up)}>
                    {t(place.name)}
                  </Link>
                  <Icon name={iconName.next} small />
                </>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

export interface TopBarProps {
  readonly layout: LayoutMode;
}

/**
 * The bar at the top of the content (UX §3): where you are, and what a tablet or a phone needs
 * instead of a sidebar: the *Menu* button, *Search*, or *Back*.
 */
export function TopBar({ layout }: TopBarProps) {
  const { pathname } = useLocation();
  const goBack = useGoBack();
  const place = placeAt(pathname);
  const phone = layout === "phone";
  return (
    <header className={styles.bar}>
      {layout === "tablet" ? (
        <Button
          onPress={() => {
            openOverlay("menu");
          }}
        >
          <Icon name={iconName.menu} />
          {t("shell.menu-button")}
        </Button>
      ) : null}
      {phone && place?.parent !== null && place !== undefined ? (
        <Button quiet onPress={goBack}>
          <Icon name={iconName.back} />
          {t("shell.back")}
        </Button>
      ) : null}
      {phone ? (
        <span className={styles.title}>{place === undefined ? t("app.name") : t(place.name)}</span>
      ) : (
        <Trail pathname={pathname} />
      )}
      {layout === "tablet" ? (
        <Button
          onPress={() => {
            openOverlay("search");
          }}
        >
          <Icon name={iconName.search} />
          {t("shell.search")}
        </Button>
      ) : null}
    </header>
  );
}
