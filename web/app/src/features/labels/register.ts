import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, issueSlots } from "../issues/index.ts";

// Loaded when an issue is first opened, so the first page stays small (coding-standards section 6).
const IssueTagsBlock = lazyRouteComponent(
  () => import("./IssueLabelsAndTagsBlock.tsx"),
  "IssueTagsBlock",
);
const IssueLabelsBlock = lazyRouteComponent(
  () => import("./IssueLabelsAndTagsBlock.tsx"),
  "IssueLabelsBlock",
);

/** Puts the rows *Tags* and *Labels* of an issue into the Details of the issue page (WP-08). */
export function registerLabelBlocks(): void {
  addIssueBlock(issueSlots.details, {
    id: "tags",
    order: 200,
    places: ["tags"],
    component: IssueTagsBlock,
  });
  addIssueBlock(issueSlots.details, {
    id: "labels",
    order: 201,
    places: ["labels"],
    component: IssueLabelsBlock,
  });
}
