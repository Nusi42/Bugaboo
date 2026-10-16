import { addIssueBlock, issueSlots } from "../issues/index.ts";
import { OwnFieldRows } from "./blocks.tsx";
import { OwnFieldSections } from "./long-texts.tsx";

let registered = false;

/**
 * Puts the own fields into the issue page: the fields in the *Details* panel and the long texts
 * as sections under the description (the slots of WP-06). Called once when the app starts.
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
  addIssueBlock(issueSlots.ownFieldSections, {
    id: "own-field-sections",
    order: 100,
    component: OwnFieldSections,
  });
}
