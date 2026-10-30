import { part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { ListColumn } from "./ListColumn.tsx";
import { ListInfo } from "./ListInfo.tsx";
import { ListOptions } from "./ListOptions.tsx";
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
  /** Tells how wide the list is, so the columns that do not fit can give way. */
  readonly onRoom: (width: number) => void;
  /** The panel of Customize, beside the list level with its head, or under it. */
  readonly aside: ReactNode;
}

/** The options, the count line, and the list with the panel of Customize beside or under it. */
export function ListMain({
  props,
  customizing,
  dragged,
  shown,
  keyChars,
  onCustomize,
  onDragged,
  onRoom,
  aside,
}: ListMainProps) {
  const { view, fields, onView } = props;
  const rows = useRowKeys();
  return (
    <div ref={rows} {...partOf(part.listMain)}>
      <ListOptions
        view={view}
        fields={fields}
        customizing={customizing}
        onView={onView}
        onCustomize={onCustomize}
      />
      <ListInfo
        scopeName={props.scopeName}
        shown={shown}
        stale={props.stale}
        chosen={view.columns.length}
      />
      <div {...partOf(part.listBody)}>
        <ListColumn
          props={props}
          dragged={dragged}
          shown={shown}
          keyChars={keyChars}
          onDragged={onDragged}
          onRoom={onRoom}
        />
        {aside}
      </div>
    </div>
  );
}
