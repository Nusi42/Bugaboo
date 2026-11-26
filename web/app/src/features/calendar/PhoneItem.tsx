import { buttonType, IssueId, StatusIcon, groupTint, iconSize, part, partOf } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { issueAddress } from "../../shared/paths.ts";
import { known } from "../../shared/present.ts";
import type { CalendarEntry } from "./calendar-data.ts";
import { groupOf } from "./item-group.ts";
import { ItemMenu } from "./ItemMenu.tsx";

/** One issue of the list of a phone: it opens on a tap, and *Change due date …* stands beside it. */
export function PhoneItem({ entry }: { readonly entry: CalendarEntry }) {
  const navigate = useNavigate();
  const key = known(entry.issue.readable_id) ?? "";
  return (
    <li {...partOf(part.calendarItem, { states: [groupTint(groupOf(entry))] })}>
      <button
        type={buttonType.button}
        {...partOf(part.calendarItemPress)}
        onClick={() => {
          void navigate({ href: issueAddress(key) });
        }}
      >
        <StatusIcon group={groupOf(entry)} size={iconSize.small} />
        <IssueId id={key} />
        <span {...partOf(part.calendarItemTitle)}>{known(entry.issue.title) ?? ""}</span>
      </button>
      {entry.may_move ? <ItemMenu entry={entry} issueKey={key} /> : null}
    </li>
  );
}
