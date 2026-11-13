import { listWord } from "../../shared/list-columns.ts";
import type { OwnColumns } from "../issues/index.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";

/**
 * What a timeline groups by (UX §4.7) is the server's to say: the readable fields mark each field it
 * can group by (`timeline_group`, from `plan::timeline_groups_by`), and a page refuses any other.
 */
export function timelineGroupable(fields: Pick<SearchFieldsAnswer, "timelineGroups">) {
  return (word: string): boolean => fields.timelineGroups.includes(word);
}

/** The date fields the bars can take their days from: *Start date*, *Due date* and the own date fields. */
export function dateWords(own: OwnColumns): readonly string[] {
  return [
    listWord.startDate,
    listWord.dueDate,
    ...Object.keys(own).filter((word) => own[word]?.kind === "date"),
  ];
}
