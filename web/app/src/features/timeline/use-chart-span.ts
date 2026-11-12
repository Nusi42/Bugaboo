import { useState } from "react";

import { personToday } from "../../shared/clock.ts";
import type { ZoomWord } from "../../shared/list-view.ts";
import { useRegion } from "../../shared/region.ts";
import { useNow } from "../../shared/use-now.ts";
import { firstAround, placeOf, spanFrom, stepped, type Span } from "./scale.ts";

/**
 * The days the chart shows (UX §4.7): around today when it opens or its zoom changes, a step
 * earlier or later with *Earlier* and *Later*, and back with *Today*. Where the chart stands is the
 * screen's, not the view's: a saved search opens around the day it is opened.
 */
export function useChartSpan(zoom: ZoomWord) {
  const region = useRegion();
  const today = personToday(region, useNow());
  const around = (): { readonly zoom: ZoomWord; readonly first: string } => ({
    zoom,
    first: firstAround(today, zoom, region.firstDay),
  });
  const [anchor, setAnchor] = useState(around);
  const first = anchor.zoom === zoom ? anchor.first : around().first;
  const span: Span = spanFrom(first, zoom);
  const day = placeOf(span, today);
  return {
    span,
    firstDay: region.firstDay,
    /** Where today stands on the chart; none when it is not on it. */
    today: day >= 0 && day < span.days ? day : undefined,
    onStep: (direction: -1 | 1): void => {
      setAnchor({ zoom, first: stepped(first, zoom, direction) });
    },
    onToday: (): void => {
      setAnchor(around());
    },
  };
}
