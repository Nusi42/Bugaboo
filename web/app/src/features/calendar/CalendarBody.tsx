import { t } from "@bugaboo/messages";
import { Announce, DelayedSkeleton, part, partOf, useHoldWhileReading } from "@bugaboo/ui";

import { dayInSentence } from "../../shared/clock.ts";
import type { ListView } from "../../shared/list-view.ts";
import { known } from "../../shared/present.ts";
import type { CalendarModel } from "./calendar-model.ts";
import { DayGrid } from "./DayGrid.tsx";
import { PhoneDays } from "./PhoneDays.tsx";

export interface CalendarBodyProps {
  readonly model: CalendarModel;
  readonly view: ListView;
  readonly phone: boolean;
  /** The IDs of the words that say how the keys move an issue, and why one cannot be moved. */
  readonly described: { readonly keys: string; readonly locked: string };
}

/** What a screen reader hears while an issue is carried: where it is, and the keys that move it. */
function carriedWords(model: CalendarModel): string {
  const { picked } = model.pick;
  return picked === undefined
    ? ""
    : t("calendar.carried", {
        key: known(picked.entry.issue.readable_id) ?? "",
        day: dayInSentence(picked.day),
      });
}

/**
 * The grid of days, or the list of a phone, once the first read has come; the words that describe the
 * keys of an issue for screen readers, and what is said while one is carried.
 */
export function CalendarBody({ model, view, phone, described }: CalendarBodyProps) {
  useHoldWhileReading(model.isPending);
  if (model.isPending) {
    return <DelayedSkeleton lines={5} />;
  }
  return (
    <div {...partOf(part.calendarScroll)}>
      {phone ? <PhoneDays /> : <DayGrid window={model.window} span={view.calendar.span} />}
      {model.more ? <p {...partOf(part.muted)}>{t("calendar.too-many")}</p> : null}
      <span id={described.keys} {...partOf(part.visuallyHidden)}>
        {t("calendar.keys")}
      </span>
      <span id={described.locked} {...partOf(part.visuallyHidden)}>
        {t("calendar.locked-note", { field: model.words.noun })}
      </span>
      <Announce>{carriedWords(model)}</Announce>
    </div>
  );
}
