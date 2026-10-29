import { t } from "@bugaboo/messages";

import { trailTo } from "../../shared/places.ts";
import type { TrailStep } from "./page-trail.ts";

/** The steps of the fixed screens: from the start page down to this screen. */
export function stepsOfScreens(pathname: string): readonly TrailStep[] {
  const trail = trailTo(pathname);
  // The start page is where every trail begins, so it is only named when it is the page itself.
  const shown = trail.length > 1 ? trail.slice(1) : trail;
  return shown.map((place) => ({ label: t(place.name), href: place.path }));
}
