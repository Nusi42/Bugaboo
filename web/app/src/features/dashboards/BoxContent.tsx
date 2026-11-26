import { Alert, alertTone, Skeleton } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { TimeSlot, timePlace } from "../shell/index.ts";
import { SmallTimeline } from "../timeline/index.ts";
import { boxKind, type Box, type BoxAnswer } from "./dashboards.ts";
import { Changes, Results } from "./IssueBoxes.tsx";
import { NumbersChart, TimeNumbers } from "./TotalsChart.tsx";
import { WeekChart } from "./WeekChart.tsx";

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
    return <SmallTimeline page={answer.timeline} />;
  }
  if (answer.answer === "timer") {
    // The timer's own block, which reads the timer itself and is the same wherever it stands.
    return <TimeSlot place={timePlace.home} />;
  }
  return <Changes changes={answer.changes} />;
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
 */
export function BoxContent({ box, read }: BoxContentProps) {
  if (read.data !== undefined) {
    return <Answered box={box} answer={read.data} />;
  }
  if (read.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert>;
  }
  return <Skeleton lines={3} />;
}
