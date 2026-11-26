import { ItemActions } from "@bugaboo/ui";
import { itemActionsWords } from "../../shared/item-actions-words.ts";
import type { CalendarEntry } from "./calendar-data.ts";
import { useCalendar } from "./calendar-context.ts";

/** The word that names the one action of the menu of an issue. */
const changeDay = "change-day";

/**
 * The menu of an issue that may be moved, on the days of a month or a week and on the list of a phone: the shared
 * compact menu button, with *Change due date …* (named by the date field the calendar stands on) in it.
 */
export function ItemMenu({
  entry,
  issueKey,
}: {
  readonly entry: CalendarEntry;
  readonly issueKey: string;
}) {
  const { model, onChangeDay } = useCalendar();
  return (
    <ItemActions
      {...itemActionsWords(issueKey)}
      actions={[
        {
          id: changeDay,
          label: model.words.change,
          onAction: () => {
            onChangeDay(entry);
          },
        },
      ]}
    />
  );
}
