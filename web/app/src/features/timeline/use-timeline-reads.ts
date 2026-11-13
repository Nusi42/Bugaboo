import type { SearchLanguage } from "@bugaboo/wasm-search";

import type { ListView } from "../../shared/list-view.ts";
import { useOwnColumns } from "../issues/index.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";
import { timelineGroupable } from "./group-words.ts";
import { readOf, useNotScheduled, useTimelineRows } from "./timeline-data.ts";
import { useChartSpan } from "./use-chart-span.ts";

/** What a timeline is of: a search, where it looks, and the view. */
export interface TimelineInput {
  readonly text: string;
  readonly within: string | undefined;
  readonly view: ListView;
  readonly language: SearchLanguage;
  /** The fields the person may read, with their names and the ones a timeline groups by. */
  readonly fields: SearchFieldsAnswer;
  /** Whether the screen is a phone: the list with its short bars, not the chart. */
  readonly phone: boolean;
}

/** The rows and the tray of a timeline, read from the server for the days the chart shows. */
export function useTimelineReads({ text, within, view, language, fields, phone }: TimelineInput) {
  const own = useOwnColumns(language, true);
  const groupBy =
    view.groupBy !== null && timelineGroupable(fields)(view.groupBy) ? view.groupBy : null;
  const chart = useChartSpan(view.timeline.zoom, phone);
  const ask = { text, within, options: view.timeline, groupBy, span: chart.span };
  const rows = useTimelineRows(ask);
  const tray = useNotScheduled(ask);
  return {
    own,
    groupBy,
    chart,
    rows,
    tray,
    read: readOf(rows.data?.pages),
    waiting: readOf(tray.data?.pages),
  };
}
