import { lazyRouteComponent } from "@tanstack/react-router";

/** *Working hours of others* is loaded when it is first opened, with the rest of the timesheet. */
export const OthersScreen = lazyRouteComponent(
  () => import("../features/time/index.ts"),
  "OthersPage",
);
