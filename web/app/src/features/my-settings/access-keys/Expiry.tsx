import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { TextField } from "@bugaboo/ui";

import { formatCalendarDay } from "../../../shared/format.ts";
import { useRegion } from "../../../shared/region.ts";
import { useTypedDay } from "../../../shared/typed-day.ts";
import { useNow } from "../../../shared/use-now.ts";
import { defaultDaysOf, earliestDay, latestDay, presetDay } from "./expiry-day.ts";
import type { PartProps } from "./draft.ts";

export interface ExpiryProps extends PartProps {
  readonly rules: Schema<"AccessKeyRulesBody">;
  /** The day that was filled in is no day: the field says so. */
  readonly wrongDay: boolean;
}

/**
 * The day the key runs out: the default, or a day of the person's own, up to the longest time. The day
 * is written and read in the person's date format, as in every box for a day (TXT-DATES); a day
 * before the earliest or after the latest is refused at the box.
 */
export function Expiry({ draft, rules, wrongDay }: ExpiryProps) {
  // The days below are counted from `now`, which moves with the clock: the form follows it, so one left open past midnight
  // shows the day the key would run out on now, which is the day the server counts from.
  const now = useNow();
  const region = useRegion();
  const [first, last] = [earliestDay(region, now), latestDay(rules, region, now)];
  // An untouched box follows the day the key would run out on now; a typed one keeps what is typed.
  const day = useTypedDay(presetDay(rules, region, now));
  const outside = day.day !== undefined && day.day !== "" && (day.day < first || day.day > last);
  return (
    <TextField
      label={t("access-keys.expires")}
      description={t("access-keys.expires-help", {
        days: defaultDaysOf(rules),
        latest: formatCalendarDay(last),
      })}
      requiredMessage={t("error.field.required")}
      errorMessage={wrongDay || outside ? t("error.field.wrong-value") : day.problem}
      value={day.typed}
      onChange={(text) => {
        day.setTyped(text);
        draft.setDay(day.read(text) ?? text);
      }}
    />
  );
}
