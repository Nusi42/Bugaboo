import "@bugaboo/messages/parts/follow";
import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, addRowAction, issueSlots } from "../issues/index.ts";
import { useFollowOnRow } from "./use-following.ts";

const FollowersBlock = lazyRouteComponent(() => import("./FollowersBlock.tsx"), "FollowersBlock");

/**
 * Puts *Followers · Follow* into the Details of the issue page, after the other fields (WP-07), and *Follow* on the
 * rows of the issue list (UX §3.3).
 */
export function registerFollowersBlock(): void {
  addRowAction({ id: "follow", order: 0, side: "start", useAction: useFollowOnRow });
  addIssueBlock(issueSlots.details, {
    id: "followers",
    order: 200,
    places: "end",
    component: FollowersBlock,
  });
}
