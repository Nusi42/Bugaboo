// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/dashboards";

import { lazyRouteComponent } from "@tanstack/react-router";

export type { DashboardPageProps } from "./DashboardPage.tsx";
export { registerDashboards } from "./register.tsx";

/**
 * The page of a dashboard and the boxes of *Home*, with the grid, the charts and the placing they bring, are loaded
 * when they are first shown: the page through its route, which loads it with this entry in one lazy step (its loader
 * before the page opens), the boxes of *Home* as a lazy part. The rest of this entry is what joins the shell before the
 * first page (the section of the sidebar): small, so the first page waits for nothing of the grid (PERF-04).
 */
export function loadDashboardPage(): Promise<typeof import("./DashboardPage.tsx")> {
  return import("./DashboardPage.tsx");
}

export const HomeBoxes = lazyRouteComponent(() => import("./HomeBoxes.tsx"), "HomeBoxes");
