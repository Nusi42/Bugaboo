import "@bugaboo/messages/parts/follow";
import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, issueSlots } from "../issues/index.ts";

export { useFollowing, type Following } from "./use-following.ts";

const FollowersBlock = lazyRouteComponent(() => import("./FollowersBlock.tsx"), "FollowersBlock");

/** Puts *Followers · Follow* into the Details of the issue page, after the other fields (WP-07). */
export function registerFollowersBlock(): void {
  addIssueBlock(issueSlots.details, {
    id: "followers",
    order: 200,
    places: "end",
    component: FollowersBlock,
  });
}
