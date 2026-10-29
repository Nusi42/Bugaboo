import { ListFrame, part, partOf, type ListColumnSpec } from "@bugaboo/ui";
import { useCallback, useRef } from "react";

import { cellOf } from "../../shared/list-columns.ts";
import { useListContext } from "./list-context.ts";
import { ListHeader } from "./ListHeader.tsx";
import { ListResults } from "./ListResults.tsx";
import type { LoadedProps } from "./results-props.ts";
import { Selected } from "./Selected.tsx";
import { useRoom } from "./use-row-keys.ts";

export interface ListColumnProps {
  readonly props: LoadedProps;
  readonly dragged: Readonly<Record<string, number>>;
  /** The ids of the issues drawn now. */
  readonly shown: readonly string[];
  /** The characters of the longest readable ID drawn: the ID column is as wide. */
  readonly keyChars: number;
  readonly onDragged: (dragged: Readonly<Record<string, number>>) => void;
  /** Tells how wide the list is, so the columns that do not fit can give way. */
  readonly onRoom: (width: number) => void;
}

/** The box to tick the rows of the head is the first column, where rows can be ticked. */
const selectSpec: ListColumnSpec = { column: "select" };

/** The list with its head, and under it the bar of the ticked issues: the part that has the columns. */
export function ListColumn({
  props,
  dragged,
  shown,
  keyChars,
  onDragged,
  onRoom,
}: ListColumnProps) {
  const { view, fields, onView, onText } = props;
  const { words, ticks, phone, project, selection, search } = useListContext();
  const column = useRef<HTMLDivElement>(null);
  const getBox = useCallback(() => column.current, []);
  useRoom(getBox, onRoom);
  const compact = view.spacing === "compact";
  const specs: readonly ListColumnSpec[] = [
    ...(ticks && !phone ? [selectSpec] : []),
    ...words.map((word) => ({ column: cellOf(word), width: dragged[word] ?? view.widths[word] })),
  ];
  return (
    <div ref={column} {...partOf(part.listColumn)}>
      <ListFrame columns={specs} keyChars={keyChars} compact={compact}>
        {phone ? null : (
          <ListHeader
            view={view}
            fields={fields}
            shown={shown}
            onView={onView}
            onText={onText}
            onDragged={onDragged}
          />
        )}
        <ListResults view={view} />
      </ListFrame>
      {project === undefined ? null : (
        <Selected selection={selection} project={project} search={search} shown={shown} />
      )}
    </div>
  );
}
