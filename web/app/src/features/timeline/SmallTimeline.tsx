import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { groupTint, part, partOf } from "@bugaboo/ui";
import { Link } from "react-aria-components";

import { personToday } from "../../shared/clock.ts";
import { issueAddress } from "../../shared/paths.ts";
import { useRegion } from "../../shared/region.ts";
import { useNow } from "../../shared/use-now.ts";
import { PhoneIssueBody } from "./phone/PhoneRows.tsx";
import { phoneSpan, placeOf } from "./scale.ts";

/** What a read-only line says beyond its row: no warning sign and no driver, which belong to the whole timeline. */
const noMarks = { warned: false, sets: undefined } as const;

/**
 * The *Small timeline* of a dashboard (CFG-03, ADR-0050, decision 4): the rows of one page of the timeline of a search,
 * drawn as the phone list draws them (status, ID, title, dates, person and a short bar over seven weeks around today),
 * read-only: each line is the link of its issue, and nothing is dragged.
 */
export function SmallTimeline({ page }: { readonly page: Schema<"TimelinePageBody"> }) {
  const region = useRegion();
  const today = personToday(region, useNow());
  const span = phoneSpan(today, region.firstDay);
  const day = placeOf(span, today);
  const rows = page.rows.filter((row) => !row.copy);
  if (rows.length === 0) {
    return <p {...partOf(part.muted)}>{t("timeline.empty")}</p>;
  }
  return (
    <ul {...partOf(part.timelinePhone)} aria-label={t("timeline.phone-list")}>
      {rows.map((row) => (
        <li key={row.issue.id ?? ""}>
          <Link
            href={issueAddress(row.issue.readable_id ?? "")}
            {...partOf(part.timelinePhoneItem, {
              states: [groupTint(row.issue.status?.status_group ?? "not-started")],
            })}
          >
            <PhoneIssueBody
              row={row}
              days={row.bar?.days}
              span={span}
              today={day >= 0 && day < span.days ? day : undefined}
              marks={noMarks}
            />
          </Link>
        </li>
      ))}
    </ul>
  );
}
