import { t } from "@bugaboo/messages";
import { Alert, alertTone, Announce, part, partOf, usePlace } from "@bugaboo/ui";
import { useRef } from "react";

import type { Box, Dashboard } from "./dashboards.ts";
import { gridPlaceOf, readingOrder, type GridPlace } from "./grid-layout.ts";
import { GridBox } from "./GridBox.tsx";
import type { Placing } from "./use-grid-drag.ts";
import { useGridPlacing } from "./use-grid-placing.ts";

/** Where the box that is placed will land: drawn on its place, for the eye only. */
function Shadow({ place }: { readonly place: GridPlace }) {
  const ref = useRef<HTMLLIElement>(null);
  usePlace(ref, gridPlaceOf(place));
  return <li ref={ref} {...partOf(part.dashboardShadow)} aria-hidden />;
}

/** The place the box that is placed will land on, if a box is placed. */
function landing(placing: Placing | null): GridPlace | undefined {
  return placing === null ? undefined : placing.places[placing.at];
}

/** The words of where a box is placed, for screen readers while its keys move it. */
function placeWords(place: GridPlace | undefined): string | null {
  return place === undefined
    ? null
    : t("dashboards.place-now", {
        column: place.column + 1,
        row: place.row + 1,
        width: place.width,
        height: place.height,
      });
}

export interface BoxGridProps {
  readonly dashboard: Dashboard;
  /** *Change* (on *Home* *Customize*) is on: the boxes can be placed, changed and removed. */
  readonly changing: boolean;
  readonly phone: boolean;
  /** *Change box*: the form of the box opens. */
  readonly onChangeBox: (box: Box) => void;
}

/**
 * The boxes of a dashboard on their grid (UX §4.10): twelve columns on a computer, one column in the reading order on a
 * phone. In *Change* a box is dragged by its title and resized by its corner, or placed by the keys of its menu; the
 * boxes stand still while it is placed, its shadow shows where it lands, and the others make room once (UX §6.4).
 */
export function BoxGrid({ dashboard, changing, phone, onChangeBox }: BoxGridProps) {
  const gridRef = useRef<HTMLUListElement>(null);
  const grid = useGridPlacing(dashboard, () => gridRef.current, { changing, phone });
  const { boxes } = dashboard;
  const order = phone ? readingOrder(boxes) : boxes.map((_, at) => at);
  const shadow = landing(grid.placing);
  const words = placeWords(landing(grid.keyed));
  return (
    <>
      {grid.refused === null ? null : <Alert tone={alertTone.danger}>{grid.refused}</Alert>}
      <ul ref={gridRef} {...partOf(part.dashboardGrid, { variants: [phone && "phone"] })}>
        {order.map((at) => {
          const box = boxes[at];
          return box === undefined ? null : (
            <GridBox
              key={box.id}
              dashboard={dashboard}
              box={box}
              at={at}
              changing={changing}
              grid={grid.controls}
              onChangeBox={onChangeBox}
            />
          );
        })}
        {shadow === undefined ? null : <Shadow place={shadow} />}
      </ul>
      {words === null ? null : <Announce>{words}</Announce>}
    </>
  );
}
