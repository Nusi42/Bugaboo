import { useLayout, part, partOf } from "@bugaboo/ui";
import { useCallback, useEffect, useRef, useState } from "react";

import { historyStep } from "../../shared/search-address.ts";
import { ListContext } from "./list-context.ts";
import { ListMain } from "./ListMain.tsx";
import { ListPanel } from "./ListPanel.tsx";
import type { LoadedProps } from "./results-props.ts";
import { useListContextValue } from "./use-list-context-value.ts";
import { useListShown } from "./use-list-shown.ts";

/** The results of a search once the search language and the fields the person may read are there. */
export function LoadedResults(props: LoadedProps) {
  const { view, fields, defaults, onView } = props;
  const phone = useLayout() === "phone";
  const [customizing, setCustomizing] = useState(false);
  const [dragged, setDragged] = useState<Readonly<Record<string, number>>>({});
  const shown = useListShown();
  // The latest `onText`, so the function that clears the filters is the same on every draw.
  const onText = useRef(props.onText);
  useEffect(() => {
    onText.current = props.onText;
  });
  const onClear = useCallback(() => {
    onText.current("", historyStep.push);
  }, []);
  const context = useListContextValue({ ...props, onClear, report: shown.report });
  return (
    <ListContext value={context}>
      <div {...partOf(part.listEditor)}>
        <ListMain
          props={props}
          customizing={customizing}
          dragged={dragged}
          shown={shown.ids}
          onCustomize={() => {
            setCustomizing(!customizing);
          }}
          onDragged={setDragged}
        />
        {customizing && !phone ? (
          <ListPanel
            view={view}
            fields={fields}
            defaults={defaults}
            onView={onView}
            onClose={() => {
              setCustomizing(false);
            }}
          />
        ) : null}
      </div>
    </ListContext>
  );
}
