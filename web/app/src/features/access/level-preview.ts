import { listText } from "../../shared/list-text.ts";
import type { LevelTable } from "./levels.ts";

/** One line of the preview: a level and the sentence that names what the role has at that level. */
interface PreviewLine<Level extends string> {
  readonly level: Level;
  readonly text: string;
}

export interface LevelPreviewProps<Subject extends string, Level extends string> {
  readonly table: LevelTable<Subject, Level>;
  /** The ID of the role whose view is shown. */
  readonly role: string;
  /** What a field or kind is called. */
  readonly subjectName: (subject: Subject, name: string | null | undefined) => string;
  /** The sentence of a level, which names the fields or kinds the role has at that level. */
  readonly sentence: (level: Level, subjects: string) => string;
}

/** The lines of the preview, the highest level first; a level the role has nowhere has no line. */
export function previewLines<Subject extends string, Level extends string>({
  table,
  role,
  subjectName,
  sentence,
}: LevelPreviewProps<Subject, Level>): readonly PreviewLine<Level>[] {
  return table.levels.toReversed().flatMap((level): readonly PreviewLine<Level>[] => {
    const subjects = table.rows
      .filter((row) => row.cells.some((cell) => cell.role === role && cell.level === level))
      .map((row) => subjectName(row.subject, row.name));
    return subjects.length === 0 ? [] : [{ level, text: sentence(level, listText(subjects)) }];
  });
}
