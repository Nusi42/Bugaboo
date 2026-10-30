import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  Icon,
  iconName,
  iconSize,
  type LayoutMode,
  partOf,
  part,
  ariaCurrent,
} from "@bugaboo/ui";
import { useCanGoBack, useLocation, useNavigate, useRouter } from "@tanstack/react-router";
import { Link } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import { openOverlay } from "./overlay-state.ts";
import { pageTrails, type TrailStep } from "./page-trail.ts";
import { placeAt } from "../../shared/places.ts";
import { stepsOfScreens } from "./steps-of-screens.ts";
import { useRegistry } from "./registry.ts";

/**
 * *Back* on a phone: where the person came from, or else the screen this one belongs under (for a
 * page with its own trail, the step above it).
 */
function useGoBack(trail: readonly TrailStep[] | undefined): () => void {
  const router = useRouter();
  const canGoBack = useCanGoBack();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  return () => {
    const above = trail === undefined ? undefined : trail.at(-2)?.href;
    const parent = above ?? placeAt(pathname)?.parent;
    if (canGoBack) {
      router.history.back();
    } else {
      void navigate({ to: parent ?? paths.home });
    }
  };
}

/** Where you are, as links up: `My settings › Appearance`, or the projects above a project. */
function Trail({ steps }: { readonly steps: readonly TrailStep[] }) {
  return (
    <nav aria-label={t("shell.trail")} {...partOf(part.shellFill)}>
      <ol {...partOf(part.topBarCrumbs)}>
        {steps.map((step, index) => {
          const last = index === steps.length - 1;
          return (
            <li key={step.href} {...partOf(part.topBarCrumb)}>
              {last ? (
                <span aria-current={ariaCurrent.page} {...partOf(part.strong)}>
                  {step.label}
                </span>
              ) : (
                <>
                  <Link href={step.href} {...partOf(part.topBarUp)}>
                    {step.label}
                  </Link>
                  <Icon name={iconName.next} size={iconSize.small} />
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
  const set = useRegistry(pageTrails).at(-1)?.steps;
  const goBack = useGoBack(set);
  const place = placeAt(pathname);
  const phone = layout === "phone";
  const steps = set ?? stepsOfScreens(pathname);
  // A page with a trail of its own has a screen above it, which *Back* leads to.
  const hasUp = set === undefined ? place?.parent !== null && place !== undefined : true;
  return (
    <header {...partOf(part.topBar)}>
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
      {phone && hasUp ? (
        <Button look={buttonLook.quiet} onPress={goBack}>
          <Icon name={iconName.back} />
          {t("shell.back")}
        </Button>
      ) : null}
      {phone ? (
        <span {...partOf(part.topBarTitle)}>{steps.at(-1)?.label ?? t("app.name")}</span>
      ) : (
        <Trail steps={steps} />
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
