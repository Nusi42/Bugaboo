/**
 * What is dragged on the timeline, as the drag and drop of the screen names it
 * (`shared/drag-and-drop.tsx`), each with the typed way that does the same (TEST-A11Y, UX §4.7):
 * a dependency from the dot of a bar (*Add dependency…*), and an issue of *Not scheduled yet* onto
 * the chart (*Set dates…*). A bar itself is moved by its keys (`bar-keys.ts`).
 */
export const dragKinds = {
  dependency: { kind: "timeline-dependency", typed: "timeline.add-dependency" },
  unscheduled: { kind: "timeline-unscheduled", typed: "timeline.set-dates" },
} as const;
