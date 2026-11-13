import { t } from "@bugaboo/messages";
import { part, partOf, ShowMoreOf, type PagedList } from "@bugaboo/ui";
import { useState } from "react";

import { idOf, type TimelineRow } from "../timeline-data.ts";
import type { Span } from "../scale.ts";
import { PhoneGroup } from "./PhoneGroup.tsx";
import { PhoneIssue } from "./PhoneRows.tsx";

export interface PhoneTrayProps {
  readonly rows: readonly TimelineRow[];
  /** How many issues of the search have no dates. */
  readonly count: number;
  readonly pages: PagedList;
  readonly span: Span;
  /** An issue was tapped: its card opens as a sheet, with *Set dates…*. */
  readonly onOpen: (row: TimelineRow) => void;
}

/**
 * *Not scheduled yet* on a phone (UX §4.7, M8-Q12): the last group of the list, folded and opened
 * like the others, each issue tapped for its card and *Set dates…*. Nothing is dragged.
 */
export function PhoneTray({ rows, count, pages, span, onOpen }: PhoneTrayProps) {
  const [open, setOpen] = useState(true);
  return (
    <>
      <PhoneGroup
        open={open}
        count={count}
        onToggle={() => {
          setOpen((before) => !before);
        }}
      >
        <span {...partOf(part.timelineRowTitle)}>{t("timeline.not-scheduled")}</span>
      </PhoneGroup>
      {open
        ? rows.map((row) => (
            <PhoneIssue
              key={idOf(row)}
              row={row}
              days={undefined}
              span={span}
              today={undefined}
              onOpen={() => {
                onOpen(row);
              }}
            />
          ))
        : null}
      {open ? (
        <li {...partOf(part.timelinePhoneMore)}>
          <ShowMoreOf label={t("timeline.more-issues")} list={pages} />
        </li>
      ) : null}
    </>
  );
}
