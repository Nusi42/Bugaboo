import { InPaneHead, paneHeadSlot, part, partOf, ReadingHold, useLayout } from "@bugaboo/ui";
import { useId, useState } from "react";

import type { ListView } from "../../shared/list-view.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";
import type { CalendarEntry } from "./calendar-data.ts";
import { CalendarBody } from "./CalendarBody.tsx";
import { CalendarContext } from "./calendar-context.ts";
import { CalendarTools } from "./CalendarTools.tsx";
import { CalendarWindows } from "./CalendarWindows.tsx";
import { useCalendarModel, type CalendarInput } from "./use-calendar.ts";

export interface CalendarResultsProps extends Omit<CalendarInput, "phone" | "fields"> {
  readonly onView: (view: ListView) => void;
  readonly fields: SearchFieldsAnswer;
}

/**
 * *Show as › Calendar* (UX §4.16, PLAN-09): the issues of a search on a month or a week by the date field
 * the view stands on, with goals and releases as marks. A drag, a pick up with the spacebar and the arrow
 * keys, and *Change due date …* all change that date field, with *Undo*. On a phone it is a list of the
 * days to come. What it draws is what the server answers for the reader (ADR-0030): no date rule of the
 * plan is worked out here.
 */
export function CalendarResults(props: CalendarResultsProps) {
  const phone = useLayout() === "phone";
  const model = useCalendarModel({ ...props, phone });
  const [changing, setChanging] = useState<CalendarEntry | undefined>();
  const [openDay, setOpenDay] = useState<string | undefined>();
  const [keys, locked] = [useId(), useId()];
  const described = { keys, locked };
  return (
    <CalendarContext value={{ model, onChangeDay: setChanging, onOpenDay: setOpenDay, described }}>
      <ReadingHold>
        <div {...partOf(part.calendar)}>
          <InPaneHead slot={paneHeadSlot.view}>
            <CalendarTools
              view={props.view}
              onView={props.onView}
              fields={props.fields}
              model={model}
              phone={phone}
            />
          </InPaneHead>
          <CalendarBody model={model} view={props.view} phone={phone} described={described} />
        </div>
      </ReadingHold>
      <CalendarWindows
        changing={changing}
        onCloseChange={() => {
          setChanging(undefined);
        }}
        openDay={openDay}
        onCloseDay={() => {
          setOpenDay(undefined);
        }}
      />
    </CalendarContext>
  );
}
