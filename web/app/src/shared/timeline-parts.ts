import { listWord } from "./list-columns.ts";
import type { TimelineOptions } from "./list-view.ts";
import type { SearchState } from "./search-address.ts";

/** The parts of the address that say how a timeline differs from its defaults. */
export function timelineParts(
  timeline: TimelineOptions,
  usual: TimelineOptions,
): Pick<SearchState, "zoom" | "effort" | "drivers" | "dates"> {
  const { start_field: start, end_field: end } = timeline;
  const sameDates = start === usual.start_field && end === usual.end_field;
  const effort = timeline.plan_by_effort ?? false;
  const drivers = timeline.deadline_drivers ?? false;
  // Back to *Start date* and *Due date* from a saved view of own date fields is written too.
  const dates = [start ?? listWord.startDate, end ?? listWord.dueDate] as const;
  return {
    ...(timeline.zoom === usual.zoom ? {} : { zoom: timeline.zoom }),
    ...(effort === (usual.plan_by_effort ?? false) ? {} : { effort }),
    ...(drivers === (usual.deadline_drivers ?? false) ? {} : { drivers }),
    ...(sameDates ? {} : { dates }),
  };
}
