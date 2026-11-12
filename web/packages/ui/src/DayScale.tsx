import { useRef } from "react";

import { partOf } from "./parts.ts";
import { usePlace } from "./place.ts";
import { part } from "./tokens.ts";

/** One unit of a scale of days: where it starts (days from the first day drawn), how many days it holds, its name. */
export interface ScaleUnit {
  readonly key: string;
  readonly start: number;
  readonly length: number;
  readonly label: string;
}

function ScaleCell({ unit }: { readonly unit: ScaleUnit }) {
  const ref = useRef<HTMLDivElement>(null);
  usePlace(ref, { "timeline-start": unit.start, "timeline-length": unit.length });
  return (
    <div ref={ref} {...partOf(part.dayScaleCell)}>
      {unit.label}
    </div>
  );
}

export interface DayScaleProps {
  /** The units in order: days, weeks, months or quarters. */
  readonly units: readonly ScaleUnit[];
}

/**
 * A scale of days (the head of the timeline, UX §4.7): a name at the start of each unit, each unit
 * as wide as its days. The width of a day is the one of the drawing it stands in (`timeline-day`).
 */
export function DayScale({ units }: DayScaleProps) {
  return (
    <div {...partOf(part.dayScale)} aria-hidden>
      {units.map((unit) => (
        <ScaleCell key={unit.key} unit={unit} />
      ))}
    </div>
  );
}
