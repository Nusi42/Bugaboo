import { t } from "@bugaboo/messages";

import { IdsIn } from "./IdsIn.tsx";

/** The heading of the sheet of the timer on a phone: *Timer on WEB-5 · 0:42*, the ID and the clock in the code font. */
export function TimerMenuHeading({
  issue,
  clock,
}: {
  readonly issue: string | undefined;
  readonly clock: string | undefined;
}) {
  return issue === undefined || clock === undefined ? (
    t("time.timer.menu")
  ) : (
    <IdsIn sentence={t("time.timer.menu-of", { issue, clock })} ids={[issue]} lengths={[clock]} />
  );
}
