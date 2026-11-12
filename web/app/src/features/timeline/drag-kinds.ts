/**
 * What is dragged on the timeline, as the drag and drop of the screen names it
 * (`shared/drag-and-drop.tsx`): a dependency from the dot of a bar, and an issue of *Not scheduled
 * yet* onto the chart.
 */
export const dragKinds = {
  dependency: "timeline-dependency",
  unscheduled: "timeline-unscheduled",
} as const;
