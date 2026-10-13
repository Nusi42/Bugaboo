import { lazyRouteComponent } from "@tanstack/react-router";

/** The screen of *My settings › Start page*, loaded when it is first opened. */
export const StartScreen = lazyRouteComponent(
  () => import("../features/my-settings/index.ts"),
  "StartPage",
);
