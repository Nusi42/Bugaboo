import { part, partOf, useLayout } from "@bugaboo/ui";
import { useState } from "react";

import type { ListView } from "../../shared/list-view.ts";
import { PhoneTimeline } from "./phone/PhoneTimeline.tsx";
import { TimelineContext, type TimelineContextValue } from "./timeline-context.ts";
import { TimelineLegend } from "./TimelineLegend.tsx";
import { TimelineMain, type TimelineMainProps } from "./TimelineMain.tsx";
import { TimelineSide, type Asked, type TimelineSideProps } from "./TimelineSide.tsx";
import { TimelineTools } from "./TimelineTools.tsx";
import { useTimelineModel, type TimelineInput } from "./use-timeline.ts";

export interface TimelineResultsProps extends Omit<TimelineInput, "phone"> {
  readonly onView: (view: ListView) => void;
}

/** What the body reads of the model of the timeline (`useTimelineModel`). */
interface BodyModel {
  readonly lines: TimelineMainProps["lines"];
  readonly arrows: TimelineMainProps["arrows"];
  readonly chart: Omit<TimelineMainProps["chart"], "zoom">;
  readonly groupBy: string | null;
  readonly own: TimelineMainProps["own"];
  readonly rows: TimelineMainProps["pages"];
  readonly onToggle: (group: string) => void;
  readonly side: Omit<TimelineSideProps, "asked" | "onAsk" | "phone">;
  readonly context: Pick<TimelineContextValue, "daysOf" | "select">;
}

interface BodyProps {
  readonly props: TimelineResultsProps;
  readonly model: BodyModel;
  readonly phone: boolean;
  readonly onCloseWarnings: () => void;
}

/** The chart, or the list of a phone, with the side: the warnings, the card, the tray and the windows. */
function TimelineBody({ props, model, phone, onCloseWarnings }: BodyProps) {
  const [asked, setAsked] = useState<Asked | null>(null);
  const shared = {
    search: props,
    groupBy: model.groupBy,
    own: model.own,
    names: props.fields.names,
    lines: model.lines,
    pages: model.rows,
    onToggle: model.onToggle,
  };
  const deselect = (): void => {
    model.context.select();
  };
  return (
    <div {...partOf(part.timelineBody)}>
      {phone ? (
        <PhoneTimeline
          {...shared}
          daysOf={model.context.daysOf}
          span={model.chart.span}
          today={model.chart.today}
          onOpen={(row) => {
            model.context.select(row.issue.id ?? undefined);
          }}
        />
      ) : (
        <TimelineMain
          {...shared}
          arrows={model.arrows}
          chart={{ ...model.chart, zoom: props.view.timeline.zoom }}
        />
      )}
      <TimelineSide
        {...model.side}
        asked={asked}
        onAsk={setAsked}
        phone={phone ? { onCloseCard: deselect, onCloseWarnings } : undefined}
      />
    </div>
  );
}

/**
 * *Show as › Timeline* (UX §4.7, PLAN-01 … PLAN-04, PLAN-10 … PLAN-13): the tools, the Legend, the
 * chart of the rows that have dates, and at its side the list of plan warnings, the card of the
 * selected bar and *Not scheduled yet*. On a phone the chart is a list with short bars, and the card
 * and the warnings open as sheets (M8-Q12). What it draws is the plan the server answers for the
 * reader (ADR-0030, amendment M8); restricted issues are not in it at all (PLAN-08).
 */
export function TimelineResults(props: TimelineResultsProps) {
  const phone = useLayout() === "phone";
  const [warningsOpen, setWarningsOpen] = useState(false);
  const model = useTimelineModel({ ...props, phone }, props.fields.names, warningsOpen);
  return (
    <TimelineContext.Provider value={model.context}>
      <div {...partOf(part.timeline)}>
        <TimelineTools
          view={props.view}
          onView={props.onView}
          fields={props.fields}
          own={model.own}
          onStep={model.chart.onStep}
          onToday={model.chart.onToday}
          phone={phone}
          warnings={{
            count: model.openWarnings,
            open: warningsOpen,
            onToggle: () => {
              setWarningsOpen((before) => !before);
            },
          }}
        />
        {phone ? null : <TimelineLegend />}
        <TimelineBody
          props={props}
          model={model}
          phone={phone}
          onCloseWarnings={() => {
            setWarningsOpen(false);
          }}
        />
      </div>
    </TimelineContext.Provider>
  );
}
