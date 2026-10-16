import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, issueSlots } from "../issues/index.ts";

// Loaded when an issue is first opened, so the first page stays small (coding-standards section 6).
const IssueSortingBlock = lazyRouteComponent(
  () => import("./IssueSortingBlock.tsx"),
  "IssueSortingBlock",
);

/** Puts the tags and labels of an issue into the Details of the issue page (WP-08). */
export function registerLabelBlocks(): void {
  addIssueBlock(issueSlots.details, {
    id: "labels-and-tags",
    order: 200,
    component: IssueSortingBlock,
  });
}
