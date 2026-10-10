import { addIssueBlock, issueSlots } from "../issues/index.ts";
import { RelatedIssuesBlock } from "./RelatedIssuesBlock.tsx";
import { SubIssuesBlock } from "./SubIssuesBlock.tsx";

export { RelatedIssuesBlock } from "./RelatedIssuesBlock.tsx";
export { SubIssuesBlock } from "./SubIssuesBlock.tsx";

/** Puts the sub-issues and the related issues into their slots of the issue page (WP-07). */
export function registerRelationBlocks(): void {
  addIssueBlock(issueSlots.subIssues, { id: "sub-issues", order: 0, component: SubIssuesBlock });
  addIssueBlock(issueSlots.relatedIssues, {
    id: "related-issues",
    order: 0,
    component: RelatedIssuesBlock,
  });
}
