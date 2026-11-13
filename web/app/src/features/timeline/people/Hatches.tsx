import { part, partOf, usePlace } from "@bugaboo/ui";
import { useRef } from "react";

import type { HatchedStretch } from "./hatched-words.ts";

/** One stretch of hatched days inside a bar, on its days. */
function Hatch({ stretch }: { readonly stretch: HatchedStretch }) {
  const ref = useRef<HTMLSpanElement>(null);
  usePlace(ref, { "timeline-start": stretch.start, "timeline-length": stretch.length });
  return (
    <span
      ref={ref}
      {...partOf(part.timelineBarHatch, { variants: [stretch.kind] })}
      title={stretch.words}
      aria-hidden
    />
  );
}

/**
 * The days of a bar on which its person does not work (ADR-0030, decision 3): away days and public
 * holidays hatched, days off shaded like the weekend. Inside a bar they are a gap, so the bar grows
 * instead of hiding them. The bar's label says them in words.
 */
export function Hatches({ stretches }: { readonly stretches: readonly HatchedStretch[] }) {
  return stretches.map((stretch) => <Hatch key={stretch.start} stretch={stretch} />);
}
