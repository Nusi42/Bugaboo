import { t } from "@bugaboo/messages";
import type { Wording } from "@bugaboo/wasm";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { weekdayAndDate } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";
import type { TimeEntry } from "./edits.ts";
import { entryItemId } from "./entry-ids.ts";
import { IdsIn } from "./IdsIn.tsx";
import { gridLength } from "./sheet-model.ts";

export interface EntryListProps {
  /** The entries of the issue in the week, in the order of their days. */
  readonly entries: readonly TimeEntry[];
  readonly wording: Wording;
  readonly onPick: (entry: TimeEntry) => void;
}

/** The entries to choose from: each with its day, its length and a button that opens its details. */
export function EntryList({ entries, wording, onPick }: EntryListProps) {
  const region = useRegion();
  if (entries.length === 0) {
    return <p {...partOf(part.muted)}>{t("time.details.none")}</p>;
  }
  return (
    <ul {...partOf(part.sheetHistory)}>
      {entries.map((entry) => {
        const day = weekdayAndDate(entry.date, "long", region);
        return (
          <li key={entry.id} id={entryItemId(entry.id)} {...partOf(part.sheetHistoryEntry)}>
            <span>
              <IdsIn
                sentence={t("time.details.entry", {
                  day,
                  length: gridLength(wording, entry.minutes),
                })}
                lengths={[gridLength(wording, entry.minutes)]}
              />
            </span>
            {entry.note === "" ? null : <span {...partOf(part.muted)}>{entry.note}</span>}
            <span {...partOf(part.sheetAdd)}>
              <Button
                look={buttonLook.quiet}
                aria-label={t("time.details.change-of", { day })}
                onPress={() => {
                  onPick(entry);
                }}
              >
                {t("time.details.change")}
              </Button>
            </span>
          </li>
        );
      })}
    </ul>
  );
}
