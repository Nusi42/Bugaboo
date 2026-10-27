import { useLayout, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

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
  const context = useListContextValue({
    ...props,
    onClear: () => {
      props.onText("", historyStep.push);
    },
    report: shown.report,
  });
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
