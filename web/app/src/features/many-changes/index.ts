// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/many-changes";
import { lazyRouteComponent } from "@tanstack/react-router";

export { registerManyChanges } from "./register.ts";

/** How far a change to many issues is, or how it ended; loaded when it is first shown. */
export const RunProgress = lazyRouteComponent(
  () => import("./ManyChangeResultView.tsx"),
  "RunProgress",
);
