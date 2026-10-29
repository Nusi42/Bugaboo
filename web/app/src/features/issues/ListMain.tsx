import { ListFrame, part, partOf, type ListColumnSpec } from "@bugaboo/ui";

import { cellOf } from "../../shared/list-columns.ts";
import { useListContext } from "./list-context.ts";
import { ListHeader } from "./ListHeader.tsx";
import { ListInfo } from "./ListInfo.tsx";
import { ListOptions } from "./ListOptions.tsx";
import { ListResults } from "./ListResults.tsx";
import { Selected } from "./Selected.tsx";
import type { LoadedProps } from "./results-props.ts";
import { useRowKeys } from "./use-row-keys.ts";

export interface ListMainProps {
  readonly props: LoadedProps;
  readonly customizing: boolean;
  readonly dragged: Readonly<Record<string, number>>;
  /** The ids of the issues drawn now. */
  readonly shown: readonly string[];
  /** The characters of the longest readable ID drawn: the ID column is as wide. */
  readonly keyChars: number;
  readonly onCustomize: () => void;
  readonly onDragged: (dragged: Readonly<Record<string, number>>) => void;
}

/** The box to tick the rows of the head is the first column, where rows can be ticked. */
const selectSpec: ListColumnSpec = { column: "select" };

/** The options, the count line and the list with its head, in one column beside the panel. */
export function ListMain({
  props,
  customizing,
  dragged,
  shown,
  keyChars,
  onCustomize,
  onDragged,
}: ListMainProps) {
  const { view, fields, onView, onText } = props;
  const { words, ticks, phone, project, selection, search } = useListContext();
  const rows = useRowKeys();
  const compact = view.spacing === "compact";
  const specs: readonly ListColumnSpec[] = [
    ...(ticks && !phone ? [selectSpec] : []),
    ...words.map((word) => ({ column: cellOf(word), width: dragged[word] ?? view.widths[word] })),
  ];
  return (
    <div ref={rows} {...partOf(part.listMain)}>
      <ListOptions
        view={view}
        fields={fields}
        customizing={customizing}
        onView={onView}
        onCustomize={onCustomize}
      />
      <ListInfo scopeName={props.scopeName} shown={shown} stale={props.stale} />
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
        <ListResults view={view} stale={props.stale} />
      </ListFrame>
      {project === undefined ? null : (
        <Selected selection={selection} project={project} search={search} shown={shown} />
      )}
    </div>
  );
}
