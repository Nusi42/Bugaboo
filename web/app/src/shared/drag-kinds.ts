/**
 * What the screens drag, as the drag and drop of the screen names it (`drag-and-drop.tsx`), each
 * with the typed way that does the same (TEST-A11Y, UX §4.6, §4.7): on the timeline a dependency
 * from the dot of a bar (*Add dependency…*) and an issue of *Not scheduled yet* onto the chart
 * (*Set dates…*); on the board a card onto a column (*Move to …*). A bar itself is moved by its keys
 * (`features/timeline/bar-keys.ts`), a card picked up by *Space* is moved by the keys of the board.
 */
export const dragKinds = {
  dependency: { kind: "timeline-dependency", typed: "timeline.add-dependency" },
  unscheduled: { kind: "timeline-unscheduled", typed: "timeline.set-dates" },
  card: { kind: "board-card", typed: "board.move-to" },
  calendarIssue: { kind: "calendar-issue", typed: "calendar.change-due-date" },
} as const;
