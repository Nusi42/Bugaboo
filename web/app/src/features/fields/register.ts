import { addIssueBlock, issueSlots } from "../issues/index.ts";
import { OwnFieldGroups, OwnFieldRows } from "./blocks.tsx";
import { OwnFieldSections } from "./long-texts.tsx";

let registered = false;

/**
 * Puts the own fields into the issue page: the fields as rows of the *Details* panel (or, for a
 * layout with titled sections or tabs, as lists after it) and the long texts as sections under the
 * description (the slots of WP-06). Called once when the app starts.
 */
export function registerFields(): void {
  if (registered) {
    return;
  }
  registered = true;
  addIssueBlock(issueSlots.details, {
    id: "own-fields",
    order: 100,
    component: OwnFieldRows,
  });
  addIssueBlock(issueSlots.detailSections, {
    id: "own-field-groups",
    order: 100,
    component: OwnFieldGroups,
  });
  addIssueBlock(issueSlots.ownFieldSections, {
    id: "own-field-sections",
    order: 100,
    component: OwnFieldSections,
  });
}
