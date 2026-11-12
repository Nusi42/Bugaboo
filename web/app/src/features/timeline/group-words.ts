import { listWord } from "../../shared/list-columns.ts";
import { timelineStartGroup } from "../../shared/list-view.ts";
import type { OwnColumns } from "../issues/index.ts";

/**
 * What a timeline can be grouped by (UX §4.7): the project, the goal, the person, the status, the
 * priority, or an own field of choices. The server answers the same set (`groups_of` of the plan)
 * and refuses any other; a release joins it in M10.
 */
const builtIn: ReadonlySet<string> = new Set([
  listWord.project,
  timelineStartGroup,
  listWord.assignedTo,
  listWord.status,
  listWord.priority,
]);

/** Whether the timeline can be grouped by the field of this word. */
export function timelineGroupable(own: OwnColumns): (word: string) => boolean {
  return (word) => {
    const kind = own[word]?.kind;
    return builtIn.has(word) || kind === "pick-one" || kind === "pick-several";
  };
}

/** The date fields the bars can take their days from: *Start date*, *Due date* and the own date fields. */
export function dateWords(own: OwnColumns): readonly string[] {
  return [
    listWord.startDate,
    listWord.dueDate,
    ...Object.keys(own).filter((word) => own[word]?.kind === "date"),
  ];
}
