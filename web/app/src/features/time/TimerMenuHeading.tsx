import { IdsIn } from "./IdsIn.tsx";
import { timerMenuTitle } from "./timer-actions.ts";

/** The heading of the sheet of the timer on a phone: its title, with only the ID and the clock in the code font. */
export function TimerMenuHeading({
  issue,
  clock,
}: {
  readonly issue: string | undefined;
  readonly clock: string | undefined;
}) {
  const title = timerMenuTitle(issue, clock);
  return issue === undefined || clock === undefined ? (
    title
  ) : (
    <IdsIn sentence={title} ids={[issue]} lengths={[clock]} />
  );
}
