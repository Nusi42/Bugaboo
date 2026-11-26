import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  ChoiceGroup,
  IconButton,
  iconName,
  live,
  part,
  partOf,
  selectLook,
} from "@bugaboo/ui";

import { monthTitle } from "../../shared/clock.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { spanOf, spanWords, type ListView, type SpanWord } from "../../shared/list-view.ts";
import { ViewChoices, type SearchFieldsAnswer } from "../search/index.ts";
import { DatesFromChoice } from "./DatesFromChoice.tsx";
import type { CalendarModel } from "./calendar-model.ts";

export interface CalendarToolsProps {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
  readonly fields: SearchFieldsAnswer;
  readonly model: Pick<CalendarModel, "window" | "own" | "onStep" | "onToday">;
  /** A phone lists the days from a day on: it steps them, but has no month or week; *Show as* is in the sheet of the filters. */
  readonly phone: boolean;
}

/** The name of each span: *Month · Week*. */
const spanNames = {
  month: "calendar.span-month",
  week: "calendar.span-week",
} as const satisfies Readonly<Record<SpanWord, string>>;

/** What stands between the two steps: the month and its year, or the days of the week. */
function periodTitle(window: CalendarToolsProps["model"]["window"], span: SpanWord): string {
  return span === spanOf.month
    ? monthTitle(window.month ?? window.first)
    : t("calendar.week-range", {
        first: formatCalendarDay(window.first),
        last: formatCalendarDay(window.last),
      });
}

/**
 * *‹ October 2026 ›*, *Today* and *Month · Week* (UX §4.16). A phone has the steps and *Today* over its list,
 * which is named by its first and last day as a week is, and no *Month · Week*.
 */
function Period({ view, onView, model, phone }: Omit<CalendarToolsProps, "fields">) {
  const { span } = view.calendar;
  return (
    <>
      <IconButton
        icon={iconName.back}
        label={t("calendar.earlier")}
        onPress={() => {
          model.onStep(-1);
        }}
      />
      <h2 {...partOf(part.calendarTitle)} aria-live={live.polite}>
        {periodTitle(model.window, phone ? spanOf.week : span)}
      </h2>
      <IconButton
        icon={iconName.next}
        label={t("calendar.later")}
        onPress={() => {
          model.onStep(1);
        }}
      />
      <Button look={buttonLook.quiet} onPress={model.onToday}>
        {t("calendar.today")}
      </Button>
      {phone ? null : (
        <ChoiceGroup
          legend={t("calendar.span")}
          choices={spanWords.map((word) => ({ value: word, label: t(spanNames[word]) }))}
          value={span}
          onChange={(next) => {
            onView({ ...view, calendar: { ...view.calendar, span: next } });
          }}
        />
      )}
    </>
  );
}

/**
 * The tools over the calendar (UX §4.16): *Show as*, *Dates from*, the steps through time with *Today*
 * and *Month · Week*. Every choice is part of the view, kept in the address and saved with the search.
 * A phone has the steps but no month or week, and its *Show as* is in the sheet of the filters.
 */
export function CalendarTools(props: CalendarToolsProps) {
  const { view, onView, fields, model, phone } = props;
  return (
    <div {...partOf(part.calendarTools)}>
      {phone ? null : (
        <ViewChoices view={view} fields={fields} onView={onView} look={selectLook.inline} />
      )}
      <DatesFromChoice view={view} onView={onView} names={fields.names} own={model.own} />
      <Period view={view} onView={onView} model={model} phone={phone} />
    </div>
  );
}
