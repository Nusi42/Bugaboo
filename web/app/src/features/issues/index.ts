// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/issues";

export { IssueFocusPage, type IssueFocusPageProps } from "./IssueFocusPage.tsx";
export { ProjectIssuesPage, type ProjectIssuesPageProps } from "./ProjectIssuesPage.tsx";
export { IssueLongText, type IssueLongTextProps } from "./IssueLongText.tsx";
export { registerIssues } from "./register.tsx";
export { addSelectionBar, type SelectionBarProps } from "./selection-bars.ts";
export type { IssueSelection } from "./use-selection.ts";
export { peopleOf, useAssignablePeople } from "./use-assignable-people.ts";
export { issueCells } from "./issue-cells.tsx";
export { priorityChoices } from "./issue-choices.ts";
export { useProjectStatusChanges } from "./use-project-status-changes.ts";
export { choiceIdOf, statusKeyOfChoice, type ProjectChange } from "./status-choices.ts";
export { versionCounted } from "./issue-requests.ts";
export type { SaveLongText } from "./use-description.ts";
export {
  addIssueBlock,
  issueBlocks,
  issueSlots,
  maskOf,
  type IssueBlock,
  type IssueBlockEntry,
  type IssueBlockProps,
  type IssueChange,
  type IssueDetail,
  type IssueFieldMask,
  type IssueSlot,
  type IssueView,
} from "./slots.ts";
