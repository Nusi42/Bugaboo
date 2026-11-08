import { useCanGoBack, useLocation, useNavigate, useRouter } from "@tanstack/react-router";

import { paths } from "../../shared/paths.ts";
import { placeAt } from "../../shared/places.ts";
import { pageTrails, type TrailStep } from "./page-trail.ts";
import { useRegistry } from "./registry.ts";
import { stepsOfScreens } from "./steps-of-screens.ts";

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

/** What the top bar shows of where the person is: the steps of the trail, whether a screen is above, and *Back*. */
export function useTopBar(): {
  readonly steps: readonly TrailStep[];
  readonly hasUp: boolean;
  readonly goBack: () => void;
} {
  const { pathname } = useLocation();
  const set = useRegistry(pageTrails).at(-1)?.steps;
  const goBack = useGoBack(set);
  const place = placeAt(pathname);
  // A page with a trail of its own has a screen above it, which *Back* leads to.
  const hasUp = set === undefined ? place?.parent !== null && place !== undefined : true;
  return { steps: set ?? stepsOfScreens(pathname), hasUp, goBack };
}
