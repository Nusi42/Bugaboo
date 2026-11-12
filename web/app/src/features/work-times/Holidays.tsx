import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useEffect } from "react";

import { refreshWorkTimes } from "../../shared/work-times-reads.ts";
import { usePersonalForm } from "../my-settings/index.ts";
import { SettingsForm } from "../settings/index.ts";
import type { WorkTimes } from "./work-times.ts";
import { nextHolidaysText } from "./work-times-words.ts";

/**
 * *Holidays* on one's own page (UX §4.13): the generic control of the setting, with *Reset to
 * default* back to the organization's choice, and under it the next three holidays that count, as
 * the server works them out, so the choice is easy to check. When a new choice is saved, the work
 * times are read again for the holidays of the new choice.
 */
function OwnHolidays({ workTimes }: { readonly workTimes: WorkTimes }) {
  const form = usePersonalForm([settingKeys.workTimesHolidays]);
  const chosen = form.settings[0]?.value;
  const saved = form.saveState === "saved";
  // Once the new choice is saved, the server answers its holidays: read them again until it does.
  useEffect(() => {
    if (saved && typeof chosen === "string" && chosen !== workTimes.holidays) {
      refreshWorkTimes();
    }
  }, [saved, chosen, workTimes.holidays]);
  return (
    <SettingsForm
      {...form}
      searchable={false}
      helps={{ [settingKeys.workTimesHolidays]: () => nextHolidaysText(workTimes) }}
    />
  );
}

/** Another person's *Holidays*: read only, with the next three that count for them. */
function OthersHolidays({ workTimes }: { readonly workTimes: WorkTimes }) {
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.text)}>
        {t("work-times.holidays-of", { choice: t(`holidays.region.${workTimes.holidays}`) })}
      </p>
      <p {...partOf(part.muted)}>{nextHolidaysText(workTimes)}</p>
    </div>
  );
}

/** *Holidays*: which public holidays count for the person (ADR-0030, amendment M8). */
export function Holidays({ workTimes }: { readonly workTimes: WorkTimes }) {
  return workTimes.own ? (
    <OwnHolidays workTimes={workTimes} />
  ) : (
    <OthersHolidays workTimes={workTimes} />
  );
}
