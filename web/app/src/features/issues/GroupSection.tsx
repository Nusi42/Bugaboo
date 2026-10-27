import { t } from "@bugaboo/messages";
import { useState } from "react";
import type { FoundLine } from "./found-issues.ts";
import type { GroupOfKey } from "./group-keys.ts";
import { GroupHeader } from "./GroupHeader.tsx";
import { RowsOf } from "./ListRows.tsx";
import { useGroupRows } from "./use-group-rows.ts";
import { useReportShown } from "./use-report-shown.ts";

export interface GroupSectionProps {
  readonly group: GroupOfKey;
  readonly count: number;
}

/** The mark that tells the rows of one group from those of another: the same issue can stand in two. */
function prefixOf(group: GroupOfKey): string {
  return `${group.id}/`;
}

const none: readonly FoundLine[] = [];

/** A group and its rows, which can be folded (UX section 4.2). */
export function GroupSection({ group, count }: GroupSectionProps) {
  const [open, setOpen] = useState(true);
  const rows = useGroupRows(group, open);
  // A folded group draws no rows, so none of them is shown, counted or ticked with the others.
  useReportShown(group.id, open ? rows.lines : none);
  return (
    <>
      <GroupHeader
        group={group}
        count={count}
        open={open}
        note={rows.listable ? rows.failed : t("issues.group-unlisted")}
        onToggle={() => {
          setOpen(!open);
        }}
      />
      {open && rows.listable ? (
        <RowsOf
          group={prefixOf(group)}
          lines={rows.lines}
          hasMore={rows.hasMore}
          isLoading={rows.isLoading}
          onMore={rows.onMore}
        />
      ) : null}
    </>
  );
}
