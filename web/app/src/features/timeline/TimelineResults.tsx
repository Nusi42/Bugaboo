import { part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import type { ListView } from "../../shared/list-view.ts";
import { TimelineContext } from "./timeline-context.ts";
import { TimelineLegend } from "./TimelineLegend.tsx";
import { TimelineMain } from "./TimelineMain.tsx";
import { TimelineSide, type Asked } from "./TimelineSide.tsx";
import { TimelineTools } from "./TimelineTools.tsx";
import { useTimelineModel, type TimelineInput } from "./use-timeline.ts";

export interface TimelineResultsProps extends TimelineInput {
  readonly onView: (view: ListView) => void;
}

/**
 * *Show as › Timeline* (UX §4.7, PLAN-01 … PLAN-04): the tools, the Legend, the chart of the rows
 * that have dates, and at its side the card of the selected bar and *Not scheduled yet*. What it
 * draws is the plan the server answers for the reader (ADR-0030, amendment M8); restricted issues
 * are not in it at all (PLAN-08).
 */
export function TimelineResults(props: TimelineResultsProps) {
  const model = useTimelineModel(props, props.fields.names);
  const [asked, setAsked] = useState<Asked | null>(null);
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
        />
        <TimelineLegend />
        <div {...partOf(part.timelineBody)}>
          <TimelineMain
            search={props}
            groupBy={model.groupBy}
            own={model.own}
            names={props.fields.names}
            lines={model.lines}
            arrows={model.arrows}
            chart={{ ...model.chart, zoom: props.view.timeline.zoom }}
            pages={model.rows}
            onToggle={model.onToggle}
          />
          <TimelineSide {...model.side} asked={asked} onAsk={setAsked} />
        </div>
      </div>
    </TimelineContext.Provider>
  );
}
