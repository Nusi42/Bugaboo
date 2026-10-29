import { useLayout, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { ListContext } from "./list-context.ts";
import { ListMain } from "./ListMain.tsx";
import { ListPanel } from "./ListPanel.tsx";
import type { LoadedProps } from "./results-props.ts";
import { useClearFilters } from "./use-clear-filters.ts";
import { useListContextValue } from "./use-list-context-value.ts";
import { useListShown } from "./use-list-shown.ts";

/** The results of a search once the search language and the fields the person may read are there. */
export function LoadedResults(props: LoadedProps) {
  const { view, fields, defaults, onView } = props;
  const phone = useLayout() === "phone";
  const [customizing, setCustomizing] = useState(false);
  const [room, setRoom] = useState<number>();
  const [dragged, setDragged] = useState<Readonly<Record<string, number>>>({});
  const shown = useListShown();
  const onClear = useClearFilters(props.onText);
  const context = useListContextValue({
    ...props,
    onClear,
    report: shown.report,
    room,
    dragged,
    keyChars: shown.keyChars,
  });
  return (
    <ListContext value={context}>
      <div {...partOf(part.listEditor)}>
        <ListMain
          props={props}
          customizing={customizing}
          dragged={dragged}
          shown={shown.ids}
          keyChars={shown.keyChars}
          onCustomize={() => {
            setCustomizing(!customizing);
          }}
          onDragged={setDragged}
          onRoom={setRoom}
          aside={
            customizing && !phone ? (
              <ListPanel
                view={view}
                fields={fields}
                defaults={defaults}
                onView={onView}
                onClose={() => {
                  setCustomizing(false);
                }}
              />
            ) : null
          }
        />
      </div>
    </ListContext>
  );
}
