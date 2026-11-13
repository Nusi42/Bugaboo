import { t } from "@bugaboo/messages";
import { BottomSheet } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { BarCard } from "./BarCard.tsx";
import type { BarDays } from "./lines.ts";
import type { TimelineRow } from "./timeline-data.ts";
import { WarningsList, type WarningsListProps } from "./warnings/WarningsList.tsx";

/**
 * A typed way that is open for the issue of a row: *Set dates…*, *Add dependency…*, or the person
 * picker of *Give it to someone else…*.
 */
export interface Asked {
  readonly what: "dates" | "dependency" | "give";
  readonly row: TimelineRow;
}

/** The selected bar: its row, its days, and whether its dates may change. */
export interface SelectedBar {
  readonly row: TimelineRow;
  /** Whether the issue has a copy on the page (*Group by person*): planned with the first person's work times. */
  readonly copied: boolean;
  readonly days: BarDays | undefined;
  readonly mayMove: boolean;
}

/** A sheet from the bottom of a phone, open while it has something to show. */
function Sheet({
  label,
  content,
  onClose,
}: {
  readonly label: string;
  readonly content: ReactNode;
  readonly onClose: () => void;
}) {
  return (
    <BottomSheet
      isOpen={content !== null}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={label}
      closeLabel={t("common.close")}
    >
      {content}
    </BottomSheet>
  );
}

/** What the side shows on a phone: the card and the list of warnings as sheets, closed by these. */
export interface PhoneSheets {
  readonly onCloseCard: () => void;
  readonly onCloseWarnings: () => void;
}

export interface CardAndWarningsProps {
  /** The list of plan warnings, while it is open. */
  readonly warnings: Omit<WarningsListProps, "onGive"> | undefined;
  readonly selected: SelectedBar | undefined;
  /** A typed way is asked for an issue: *Set dates…*, *Add dependency…* or the person picker. */
  readonly onAsk: (asked: Asked) => void;
  readonly phone: PhoneSheets | undefined;
}

/** The card and the list of warnings: beside the chart on a computer, as sheets from the bottom on a phone. */
export function CardAndWarnings({ warnings: list, selected, onAsk, phone }: CardAndWarningsProps) {
  const warnings =
    list === undefined ? null : (
      <WarningsList
        {...list}
        onGive={(row) => {
          onAsk({ what: "give", row });
        }}
      />
    );
  const card =
    selected === undefined ? null : (
      <BarCard
        {...selected}
        onSetDates={() => {
          onAsk({ what: "dates", row: selected.row });
        }}
        onAddDependency={() => {
          onAsk({ what: "dependency", row: selected.row });
        }}
      />
    );
  if (phone === undefined) {
    return (
      <>
        {warnings}
        {card}
      </>
    );
  }
  return (
    <>
      <Sheet
        label={t("timeline.warnings-title")}
        content={warnings}
        onClose={phone.onCloseWarnings}
      />
      <Sheet label={t("timeline.phone-sheet")} content={card} onClose={phone.onCloseCard} />
    </>
  );
}
