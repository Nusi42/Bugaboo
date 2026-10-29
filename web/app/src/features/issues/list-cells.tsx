import { t } from "@bugaboo/messages";
import { IssueTypeIcon, listShowsWord, Priority } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { formatAgo } from "../../shared/format.ts";
import { cellOf, type ColumnCell } from "../../shared/list-columns.ts";
import type { FoundLine } from "./found-issues.ts";
import { issueCells } from "./issue-cells.tsx";
import type { OwnColumns } from "./own-columns.ts";
import { assignedText, labelsOf, valueOf } from "./value-cell.tsx";

/** What a cell needs besides the issue. */
export interface CellContext {
  /** Where the title goes: the focus view or the side panel, as the person chose. */
  readonly href: string;
  /** The clock of the screen, for how long ago it was changed. */
  readonly now: number;
  /** The fields of the organization by their search word, for the columns that show one. */
  readonly own: OwnColumns;
  /** Whether this is the issue open next to the list. */
  readonly current: boolean;
}

type RowStart = ReturnType<typeof issueCells>;

/** What the content of a cell is made from. */
interface CellInput {
  readonly word: string;
  readonly line: FoundLine;
  readonly context: CellContext;
  readonly start: RowStart;
}

/** What stands in the cell of each kind of column (UX section 4.2, R9). */
const contentOf: Readonly<Record<ColumnCell, (input: CellInput) => ReactNode>> = {
  status: ({ start }) => start.status,
  key: ({ start }) => start.key,
  type: ({ line }) =>
    line.issueType === undefined ? undefined : (
      <>
        <IssueTypeIcon name={line.issueType.icon} />
        {line.issueType.name}
      </>
    ),
  title: ({ start }) => start.title,
  labels: ({ line }) => labelsOf(line),
  priority: ({ line }) => (
    <Priority
      priority={line.priority}
      label={t(`priority.${line.priority}`)}
      showWord={listShowsWord(line.priority)}
    />
  ),
  person: ({ line }) => assignedText(line),
  changed: ({ line, context }) =>
    line.changedAt === undefined ? undefined : formatAgo(line.changedAt, context.now),
  project: ({ line }) => line.project?.name,
  value: ({ word, line, context }) => valueOf(word, line, context.own),
};

/** A cell of a row: the column it stands in, the word of its field and what it shows. */
export interface RowCell {
  readonly word: string;
  readonly cell: ColumnCell;
  readonly content: ReactNode;
}

/**
 * The cells of the row of an issue, one for each column in the order of the columns. The same
 * things stand in the same places as on the issue page; a field the list has no cell for shows its
 * value as text.
 */
export function rowCells(
  words: readonly string[],
  line: FoundLine,
  context: CellContext,
): readonly RowCell[] {
  const start = issueCells(
    {
      key: line.key,
      title: line.title,
      group: line.group,
      statusWord: line.status.name,
      type: line.issueType,
    },
    context.href,
    { current: context.current },
  );
  return words.map((word) => {
    const cell = cellOf(word);
    return { word, cell, content: contentOf[cell]({ word, line, context, start }) };
  });
}
