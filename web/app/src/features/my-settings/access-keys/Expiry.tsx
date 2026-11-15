import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { TextField } from "@bugaboo/ui";
import { useState } from "react";

import { dayText, formatCalendarDay } from "../../../shared/format.ts";
import { dayProblem, readDay } from "../../../shared/read-day.ts";
import { useRegion } from "../../../shared/region.ts";
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
  const [typed, setTyped] = useState<string | null>(null);
  const shown = typed ?? dayText(draft.day ?? presetDay(rules, region, now), region);
  const read = readDay(shown, region);
  const outside = read !== undefined && (read < first || read > last);
  return (
    <TextField
      label={t("access-keys.expires")}
      description={t("access-keys.expires-help", {
        days: defaultDaysOf(rules),
        latest: formatCalendarDay(last),
      })}
      requiredMessage={t("error.field.required")}
      errorMessage={wrongDay || outside ? t("error.field.wrong-value") : dayProblem(shown, region)}
      value={shown}
      onChange={(text) => {
        setTyped(text);
        draft.setDay(readDay(text, region) ?? text);
      }}
    />
  );
}
