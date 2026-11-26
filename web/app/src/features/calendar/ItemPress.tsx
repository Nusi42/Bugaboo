import { buttonType, IssueId, StatusIcon, iconSize, part, partOf } from "@bugaboo/ui";
import { t } from "@bugaboo/messages";
import { useNavigate } from "@tanstack/react-router";

import { issueAddress } from "../../shared/paths.ts";
import { known } from "../../shared/present.ts";
import type { CalendarEntry } from "./calendar-data.ts";
import { useCalendar } from "./calendar-context.ts";
import { groupOf } from "./item-group.ts";
import { useFocusAfterMove } from "./use-focus-after-move.ts";
import { itemKeys } from "./use-item-keys.ts";

/**
 * The button that fills an issue of the calendar: its status icon, Issue ID and title. A press opens
 * the issue; *Space* picks it up where the person may move it, and the arrow keys carry it.
 */
export function ItemPress({ entry }: { readonly entry: CalendarEntry }) {
  const { model, described } = useCalendar();
  const navigate = useNavigate();
  const key = known(entry.issue.readable_id) ?? "";
  const mayMove = entry.may_move;
  const title = known(entry.issue.title) ?? "";
  // A locked issue says why on pointing at it, as its outer part does (CalendarItem): the reason comes first.
  const tip = mayMove ? "" : `${t("calendar.locked", { key, field: model.words.noun })} `;
  return (
    <button
      ref={useFocusAfterMove(entry, model.pick)}
      type={buttonType.button}
      title={`${tip}${key} ${title}`.trim()}
      aria-describedby={mayMove ? described.keys : described.locked}
      {...partOf(part.calendarItemPress)}
      onClick={() => {
        void navigate({ href: issueAddress(key) });
      }}
      {...itemKeys(entry, model.pick, mayMove)}
    >
      <StatusIcon group={groupOf(entry)} size={iconSize.small} />
      <IssueId id={key} />
      <span {...partOf(part.calendarItemTitle)}>{title}</span>
    </button>
  );
}
