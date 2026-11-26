// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/issues";

export { IssueFocusPage, type IssueFocusPageProps } from "./IssueFocusPage.tsx";
export { ProjectIssuesPage, type ProjectIssuesPageProps } from "./ProjectIssuesPage.tsx";
export { LegendButton } from "./LegendButton.tsx";
export { IssueLegend } from "./IssueLegend.tsx";
export { SearchResults } from "./SearchResults.tsx";
export { IssueLongText, type IssueLongTextProps } from "./IssueLongText.tsx";
export { registerIssues } from "./register.tsx";
export { addMoreEntry, type MoreDialogProps, type MoreEntry } from "./more-entries.ts";
export { addSelectionBar, type SelectionBarProps } from "./selection-bars.ts";
export {
  addRowAction,
  type RowActionInput,
  type RowActionState,
  type RowDialogProps,
} from "./row-actions.ts";
export type { IssueSelection } from "./use-selection.ts";
export { peopleOf, useAssignablePeople } from "./use-assignable-people.ts";
export { sortOf } from "./list-sort.ts";
export { TimeLeftLength } from "./TimeFacts.tsx";
export { timeLeftOf } from "./time-words.ts";
export { issueCells, priorityCell } from "./issue-cells.tsx";
export { SlimIssueList } from "./SlimIssueList.tsx";
export { lineOf } from "./issues.ts";
export { AskFor, type AskForProps } from "./AskFor.tsx";
export { needsAsking, type OfferedChange } from "./ask-for.ts";
export { priorityChoices } from "./issue-choices.ts";
export { useProjectStatusChanges } from "./use-project-status-changes.ts";
export { choiceIdOf, choiceLabelOf, type ProjectChange } from "./status-choices.ts";
export { inLayoutOrder } from "./details-order.ts";
export { counting, listRowsChanged, versionCounted } from "./issue-requests.ts";
export { rowStatusOf, type RowChange } from "./row-status.ts";
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
export { sendChange } from "./issue-requests.ts";
export {
  afterSaves,
  changeAfterSaves,
  newestVersion,
  saveOnNewest,
  takeVersion,
} from "./dates-in-order.ts";
export { useFoundGroups } from "./found-issues.ts";
export { groupName, groupOfKey, groupsOfKeys, type GroupOfKey } from "./group-keys.ts";
export {
  dateFieldOfWord,
  dayKeptIn,
  dayValues,
  ownDateWords,
  type DateField,
} from "./date-field.ts";
export { ownColumnText, useOwnColumns, type OwnColumn, type OwnColumns } from "./own-columns.ts";
export type { IssueRequestValues, IssueValues } from "./issue-values.ts";
export { TimeFactorChange } from "./TimeFactorChange.tsx";
export { AssignedPicker } from "./AssignedPicker.tsx";
