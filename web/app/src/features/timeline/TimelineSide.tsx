import { part, partOf, type PagedList } from "@bugaboo/ui";

import type { KeptDates } from "./date-fields.ts";
import type { Pair } from "./dependencies.ts";
import { NotScheduledTray } from "./NotScheduledTray.tsx";
import { CardAndWarnings, type Asked, type PhoneSheets, type SelectedBar } from "./SideSheets.tsx";

export type { Asked, SelectedBar } from "./SideSheets.tsx";
import type { TimelineRow } from "./timeline-data.ts";
import { TimelineWindows } from "./TimelineWindows.tsx";
import type { WarningsListProps } from "./warnings/WarningsList.tsx";

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
  /** The list of plan warnings, while it is open; *Give it to someone else…* is asked here. */
  readonly warnings: Omit<WarningsListProps, "onGive"> | undefined;
  readonly asked: Asked | null;
  readonly onAsk: (asked: Asked | null) => void;
  /** On a phone: how its sheets close; none on a computer. */
  readonly phone: PhoneSheets | undefined;
}

/**
 * The side of the chart: the list of plan warnings, the card of the selected bar, *Not scheduled yet*,
 * and the windows they open. On a phone the card and the list are sheets, and *Not scheduled yet* is
 * the last group of the phone list.
 */
export function TimelineSide(props: TimelineSideProps) {
  const { selected, asked, dateNames, phone } = props;
  const kept = asked === null ? undefined : props.keptOf(asked.row);
  const windows = asked === null || kept === undefined || dateNames === undefined;
  // On a phone a sheet closes before the window it opens, so the window is the one layer on top.
  const onAsk = (next: Asked): void => {
    phone?.onCloseCard();
    phone?.onCloseWarnings();
    props.onAsk(next);
  };
  return (
    <div {...partOf(part.timelineSide)}>
      <CardAndWarnings phone={phone} warnings={props.warnings} selected={selected} onAsk={onAsk} />
      {phone === undefined ? (
        <NotScheduledTray
          {...props.tray}
          onSetDates={(row) => {
            onAsk({ what: "dates", row });
          }}
        />
      ) : null}
      {windows ? null : (
        <TimelineWindows
          asked={asked}
          kept={kept}
          dateNames={dateNames}
          onSet={props.onSet}
          onPair={props.onPair}
          onClose={() => {
            props.onAsk(null);
          }}
        />
      )}
    </div>
  );
}
