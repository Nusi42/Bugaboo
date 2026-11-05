import { Fragment } from "react";

import { useTimeParts, type TimePlace } from "./time-parts.ts";

/**
 * The parts of time at the place they stand: the sidebar of a computer, the start page of a phone
 * and the bar of a phone show the same parts, so the timer and the work day are one tap away
 * wherever a person is (UX §3.1, §3.3). Nothing is shown until the feature of time adds its part.
 */
export function TimeSlot({
  place,
  narrow = false,
}: {
  readonly place: TimePlace;
  readonly narrow?: boolean;
}) {
  return useTimeParts().map((host) => (
    <Fragment key={host.id}>{host.render({ place, narrow })}</Fragment>
  ));
}
