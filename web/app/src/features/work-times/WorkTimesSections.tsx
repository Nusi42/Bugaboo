import { AwayTimes } from "./AwayTimes.tsx";
import { SharesLine } from "./SharesLine.tsx";
import { UsualDays } from "./UsualDays.tsx";
import type { WorkTimes, WorkTimesRead, Whose } from "./work-times.ts";

/** The parts of a page of work times under its head: the same for one's own and for somebody else's. */
export function Sections({
  whose,
  read,
  workTimes,
  otherName,
}: {
  readonly whose: Whose;
  readonly read: WorkTimesRead;
  readonly workTimes: WorkTimes;
  readonly otherName: string | undefined;
}) {
  return (
    <>
      <UsualDays
        whose={whose}
        read={read}
        usual={workTimes.usual}
        mayChange={workTimes.may_change}
      />
      <AwayTimes whose={whose} coming={workTimes} past={read.past} otherName={otherName} />
      {workTimes.shares === null ? null : <SharesLine shares={workTimes.shares} />}
    </>
  );
}
