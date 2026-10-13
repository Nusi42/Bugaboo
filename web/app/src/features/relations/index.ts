import "@bugaboo/messages/parts/relations";
import { lazyRouteComponent } from "@tanstack/react-router";

import { addIssueBlock, issueSlots } from "../issues/index.ts";

// Loaded when an issue is first opened, so the first page stays small (coding-standards section 6).
const SubIssuesBlock = lazyRouteComponent(() => import("./SubIssuesBlock.tsx"), "SubIssuesBlock");
const RelatedIssuesBlock = lazyRouteComponent(
  () => import("./RelatedIssuesBlock.tsx"),
  "RelatedIssuesBlock",
);

/** Puts the sub-issues and the related issues into their slots of the issue page (WP-07). */
export function registerRelationBlocks(): void {
  addIssueBlock(issueSlots.subIssues, { id: "sub-issues", order: 0, component: SubIssuesBlock });
  addIssueBlock(issueSlots.relatedIssues, {
    id: "related-issues",
    order: 0,
    component: RelatedIssuesBlock,
  });
}
export { relationChoices, type RelationChoice } from "./relations.ts";
