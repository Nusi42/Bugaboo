import { part, partOf, type PagedList } from "@bugaboo/ui";

import { BarCard } from "./BarCard.tsx";
import type { KeptDates } from "./date-fields.ts";
import type { Pair } from "./dependencies.ts";
import type { BarDays } from "./lines.ts";
import { NotScheduledTray } from "./NotScheduledTray.tsx";
import type { TimelineRow } from "./timeline-data.ts";
import { TimelineWindows } from "./TimelineWindows.tsx";

/** A typed way that is open: *Set dates…* or *Add dependency…*, for the issue of a row. */
export interface Asked {
  readonly what: "dates" | "dependency";
  readonly row: TimelineRow;
}

/** The selected bar: its row, its days, and whether its dates may change. */
export interface SelectedBar {
  readonly row: TimelineRow;
  readonly days: BarDays | undefined;
  readonly mayMove: boolean;
}

export interface TimelineSideProps {
  readonly selected: SelectedBar | undefined;
  readonly tray: {
    readonly rows: readonly TimelineRow[];
    readonly count: number;
    readonly pages: PagedList;
  };
  /** The names of the two date fields of the view; none while they are not known. */
  readonly dateNames: { readonly start: string; readonly end: string } | undefined;
  readonly keptOf: (row: TimelineRow) => KeptDates | undefined;
  readonly onSet: (row: TimelineRow, dates: KeptDates) => void;
  readonly onPair: (pair: Pair) => void;
  readonly asked: Asked | null;
  readonly onAsk: (asked: Asked | null) => void;
}

/** The side of the chart: the card of the selected bar, *Not scheduled yet*, and the windows they open. */
export function TimelineSide(props: TimelineSideProps) {
  const { selected, asked, onAsk, dateNames } = props;
  const kept = asked === null ? undefined : props.keptOf(asked.row);
  return (
    <div {...partOf(part.timelineSide)}>
      {selected === undefined ? null : (
        <BarCard
          {...selected}
          onSetDates={() => {
            onAsk({ what: "dates", row: selected.row });
          }}
          onAddDependency={() => {
            onAsk({ what: "dependency", row: selected.row });
          }}
        />
      )}
      <NotScheduledTray
        {...props.tray}
        onSetDates={(row) => {
          onAsk({ what: "dates", row });
        }}
      />
      {asked === null || kept === undefined || dateNames === undefined ? null : (
        <TimelineWindows
          asked={asked}
          kept={kept}
          dateNames={dateNames}
          onSet={props.onSet}
          onPair={props.onPair}
          onClose={() => {
            onAsk(null);
          }}
        />
      )}
    </div>
  );
}
