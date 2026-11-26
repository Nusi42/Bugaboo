// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/dashboards";

import { lazyRouteComponent } from "@tanstack/react-router";

export type { DashboardPageProps } from "./DashboardPage.tsx";
export { registerDashboards } from "./register.tsx";

/**
 * The page of a dashboard and the boxes of *Home*, with the grid, the charts and the placing they bring, are loaded
 * when they are first shown. The rest of this entry is what joins the shell before the first page (the section of the
 * sidebar): small, so the first page waits for nothing of the grid (PERF-04).
 */
export const DashboardPage = lazyRouteComponent(
  () => import("./DashboardPage.tsx"),
  "DashboardPage",
);
export const HomeBoxes = lazyRouteComponent(() => import("./HomeBoxes.tsx"), "HomeBoxes");
