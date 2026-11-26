import { Alert, alertTone, DelayedSkeleton, HoldWhileLoading } from "@bugaboo/ui";
import { lazyRouteComponent } from "@tanstack/react-router";
import { Suspense } from "react";

import { messageOf } from "../../shared/errors.ts";
import { TimeSlot, timePlace } from "../shell/index.ts";
import { boxKind, type Box, type BoxAnswer } from "./dashboards.ts";
import { Results } from "./IssueBoxes.tsx";
import { NumbersChart, TimeNumbers } from "./TotalsChart.tsx";
import { WeekChart } from "./WeekChart.tsx";

/**
 * The boxes that draw with a screen of their own are loaded when such a box is shown: the phone list of the timeline and
 * the sentences of the history. *Home* starts with neither, and they bring much with them that a slow phone would load
 * before *Home* shows. While such a box is loaded the hold of the page waits for it too, so it does not grow after the
 * page is seen (UX §6.4).
 */
const SmallTimeline = lazyRouteComponent(() => import("../timeline/index.ts"), "SmallTimeline");
const Changes = lazyRouteComponent(() => import("./ChangesBox.tsx"), "Changes");

/** What a box shows of the answer of its read, by the kind of the answer and of the box. */
function Answered({ box, answer }: { readonly box: Box; readonly answer: BoxAnswer }) {
  if (answer.answer === "issues") {
    return <Results issues={answer.issues} />;
  }
  if (answer.answer === "totals") {
    return box.kind === boxKind.timeTotals ? (
      <TimeNumbers box={box} totals={answer.totals} />
    ) : (
      <NumbersChart box={box} totals={answer.totals} />
    );
  }
  if (answer.answer === "week") {
    return <WeekChart box={box} entries={answer.entries} />;
  }
  if (answer.answer === "timeline") {
    return (
      <Suspense fallback={<HoldWhileLoading />}>
        <SmallTimeline page={answer.timeline} />
      </Suspense>
    );
  }
  if (answer.answer === "timer") {
    // The timer's own block, which reads the timer itself and is the same wherever it stands.
    return <TimeSlot place={timePlace.home} />;
  }
  return (
    <Suspense fallback={<HoldWhileLoading />}>
      <Changes changes={answer.changes} />
    </Suspense>
  );
}

export interface BoxContentProps {
  readonly box: Box;
  /** What the read of the box answered, while it is read, or why it could not be. */
  readonly read: {
    readonly data?: BoxAnswer | undefined;
    readonly isError: boolean;
    readonly error: unknown;
  };
}

/**
 * What a box shows (ADR-0050, decision 4): each kind drawn by the screen that exists for its data, never a copy of it:
 * the slim issue list, the charts, the timer's block, the phone list of the timeline and the sentences of the history.
 * While the first read is on its way, grey shapes stand in only once it takes longer than a moment (UX §6).
 */
export function BoxContent({ box, read }: BoxContentProps) {
  if (read.data !== undefined) {
    return <Answered box={box} answer={read.data} />;
  }
  if (read.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert>;
  }
  return <DelayedSkeleton lines={3} />;
}
