import { listWord } from "../../shared/list-columns.ts";
import type { OwnColumns } from "../issues/index.ts";

/** The date fields the bars can take their days from: *Start date*, *Due date* and the own date fields. */
export function dateWords(own: OwnColumns): readonly string[] {
  return [
    listWord.startDate,
    listWord.dueDate,
    ...Object.keys(own).filter((word) => own[word]?.kind === "date"),
  ];
}
