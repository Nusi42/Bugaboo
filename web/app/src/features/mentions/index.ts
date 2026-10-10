import { lazyRouteComponent } from "@tanstack/react-router";

/** *Mentions* of the start page, loaded when the start page is first shown. */
export const MentionsBlock = lazyRouteComponent(
  () => import("./MentionsBlock.tsx"),
  "MentionsBlock",
);
