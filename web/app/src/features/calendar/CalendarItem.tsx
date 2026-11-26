import { t } from "@bugaboo/messages";
import { groupTint, part, partOf } from "@bugaboo/ui";

import { DragItem } from "../../shared/drag-and-drop.tsx";
import { dragKinds } from "../../shared/drag-kinds.ts";
import { known } from "../../shared/present.ts";
import { idOf, type CalendarEntry } from "./calendar-data.ts";
import { useCalendar } from "./calendar-context.ts";
import { ItemMenu } from "./ItemMenu.tsx";
import { groupOf } from "./item-group.ts";
import { ItemPress } from "./ItemPress.tsx";

export interface CalendarItemProps {
  readonly entry: CalendarEntry;
  /** Whether a pointer drags it: not in the list of a day that opens over the grid. */
  readonly draggable?: boolean;
}

/**
 * An issue on its day (UX §4.16): its status icon, Issue ID and title, tinted by its status group. A
 * press opens it. A person who may change its date drags it to another day, picks it up with the
 * spacebar and carries it with the arrow keys, or uses *Change due date …* in its menu; one who may
 * not sees why on pointing at it, and it cannot be dragged.
 */
export function CalendarItem({ entry, draggable = true }: CalendarItemProps) {
  const { model } = useCalendar();
  const key = known(entry.issue.readable_id) ?? "";
  const mayMove = entry.may_move;
  const carried = model.pick.carries(entry);
  return (
    <DragItem
      kind={dragKinds.calendarIssue.kind}
      id={idOf(entry)}
      canDrag={mayMove && draggable}
      title={mayMove ? undefined : t("calendar.locked", { key, field: model.words.noun })}
      look={(dragging) =>
        partOf(part.calendarItem, {
          states: [
            groupTint(groupOf(entry)),
            !mayMove && "locked",
            dragging && "dragging",
            carried && "picked",
          ],
        })
      }
    >
      <ItemPress entry={entry} />
      {mayMove ? (
        <span {...partOf(part.calendarItemMenu)}>
          <ItemMenu entry={entry} issueKey={key} />
        </span>
      ) : null}
    </DragItem>
  );
}
