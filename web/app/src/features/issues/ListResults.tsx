import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Icon, iconName, part, partOf } from "@bugaboo/ui";

import { isInvalidSearch, messageOf } from "../../shared/errors.ts";
import { showSearchSheet } from "../../shared/search-sheet.ts";
import type { ListView } from "../../shared/list-view.ts";
import { PlainList } from "./ListBody.tsx";
import { GroupedList } from "./ListGroups.tsx";
import { useListContext } from "./list-context.ts";

export interface ListResultsProps {
  readonly view: ListView;
}

/**
 * The rows of the list, in groups or not, or why there are none. While the text of the search has
 * a mistake the rows of the last fixed search stay, with a note above them (the count line); a
 * search that was refused has no earlier list, so its error says it all.
 */
export function ListResults({ view }: ListResultsProps) {
  const { failure, phone } = useListContext();
  if (failure !== null) {
    // On a phone the mistake is in the closed sheet of the bar: the way on is a button in the note.
    return (
      <Alert tone={alertTone.danger}>
        <span {...partOf(part.row, { variants: ["inline"] })}>
          <Icon name={iconName.danger} />
          {messageOf(failure)}
        </span>
        {phone && isInvalidSearch(failure) ? (
          <div>
            <Button look={buttonLook.quiet} onPress={showSearchSheet}>
              {t("search.show-mistake")}
            </Button>
          </div>
        ) : null}
      </Alert>
    );
  }
  return (
    <div {...partOf(part.searchResults)}>
      {view.groupBy === null ? <PlainList /> : <GroupedList />}
    </div>
  );
}
