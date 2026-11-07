import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, useLayout, Window, partOf, part } from "@bugaboo/ui";

import { weekdayAndDate } from "../../shared/clock.ts";
import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { windowLabels } from "../../shared/window-labels.ts";
import type { Region } from "../../shared/region.ts";
import { IdsIn } from "./IdsIn.tsx";
import { breakSpan, cellKey, clockText } from "./sheet-model.ts";

type History = Schema<"WorkDayHistoryBody">;
type Change = Schema<"WorkDayChangeBody">;
type Times = Schema<"TimesBody">;

/** What a set of times says in words, and the times in it, which are drawn in the code face. */
interface TimesLine {
  readonly text: string;
  readonly times: readonly string[];
}

/** What a set of times says in words: *Start 7:40 · End 18:30 · Breaks 12:10–12:40*. */
function timesText(times: Times | null | undefined, zone: string, region: Region): TimesLine {
  const first = times?.periods[0];
  const last = times?.periods.at(-1);
  if (first === undefined || last === undefined) {
    return { text: t("time.sheet.history-no-times"), times: [] };
  }
  const clock = (moment: string | null | undefined): string => clockText(moment, zone, region);
  const spans = (times?.breaks ?? []).map((span) => breakSpan(clock(span.start), clock(span.end)));
  const shown = [clock(first.start), ...(last.end === null ? [] : [clock(last.end)]), ...spans];
  const parts = [
    t("time.sheet.history-start", { time: clock(first.start) }),
    last.end === null
      ? t("time.sheet.history-running")
      : t("time.sheet.history-end", { time: clock(last.end) }),
    ...(spans.length === 0
      ? []
      : [
          t("time.sheet.history-breaks", {
            spans: spans.join(t("time.sheet.list-separator")),
          }),
        ]),
  ];
  return { text: parts.join(t("time.sheet.facts-separator")), times: shown };
}

/** A sentence about a set of times (`make` writes it from their words), the times in the code face. */
function Told({
  make,
  line,
}: {
  readonly make: (times: string) => string;
  readonly line: TimesLine;
}) {
  return (
    <span>
      <IdsIn sentence={make(line.text)} lengths={line.times} />
    </span>
  );
}

function Entry({
  change,
  zone,
  region,
}: {
  readonly change: Change;
  readonly zone: string;
  readonly region: Region;
}) {
  return (
    <li {...partOf(part.sheetHistoryEntry)}>
      <span>
        {t("time.sheet.history-by", {
          person: personName(change.by),
          at: formatDateTime(change.at, region),
        })}
      </span>
      {change.before === null || change.before === undefined ? (
        <Told
          make={(times) => t("time.sheet.history-made", { times })}
          line={timesText(change.after, zone, region)}
        />
      ) : (
        <>
          <Told
            make={(times) => t("time.sheet.history-before", { times })}
            line={timesText(change.before, zone, region)}
          />
          <Told
            make={(times) => t("time.sheet.history-after", { times })}
            line={timesText(change.after, zone, region)}
          />
        </>
      )}
      {change.reason === null || change.reason === undefined ? null : (
        <span {...partOf(part.muted)}>
          {t("time.sheet.history-reason", { reason: change.reason })}
        </span>
      )}
    </li>
  );
}

export interface DayHistoryProps {
  readonly date: string;
  readonly history: History | undefined;
  readonly zone: string;
  readonly region: Region;
  readonly onClose: () => void;
}

/** The history of a work day: who changed it, when, the old and the new times and why (TIME-10). */
export function DayHistory({ date, history, zone, region, onClose }: DayHistoryProps) {
  const phone = useLayout() === "phone";
  const day = weekdayAndDate(date, phone ? "short" : "long", region);
  return (
    <Window
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={t("time.sheet.history-title", { day })}
      labels={windowLabels()}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.close")}
          </Button>
        </div>
      }
    >
      <ol {...partOf(part.sheetHistory)}>
        {(history?.changes ?? []).toReversed().map((change, index) => (
          <Entry key={cellKey(change.at, index)} change={change} zone={zone} region={region} />
        ))}
      </ol>
      {history?.truncated === true ? (
        <p {...partOf(part.muted)}>{t("time.sheet.history-older")}</p>
      ) : null}
    </Window>
  );
}
