import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, issueSlots } from "../issues/index.ts";

const FollowersBlock = lazyRouteComponent(() => import("./FollowersBlock.tsx"), "FollowersBlock");

/** Puts *Followers · Follow* into the fields panel of the issue page, after the other fields (WP-07). */
export function registerFollowersBlock(): void {
  addIssueBlock(issueSlots.fieldsPanel, { id: "followers", order: 100, component: FollowersBlock });
}
