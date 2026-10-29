import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Icon, iconName, part, partOf } from "@bugaboo/ui";

import { codeOf, messageOf } from "../../shared/errors.ts";
import { showSearchSheet } from "../../shared/search-sheet.ts";
import type { ListView } from "../../shared/list-view.ts";
import { PlainList } from "./ListBody.tsx";
import { GroupedList } from "./ListGroups.tsx";
import { useListContext } from "./list-context.ts";

/** The code of the answer for a text that has a mistake in the search language (a search that takes too long has none). */
const searchInvalid = "search.invalid";

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
        {phone && codeOf(failure) === searchInvalid ? (
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
      {view.groupBy === null ? (
        <PlainList spacing={view.spacing} />
      ) : (
        <GroupedList spacing={view.spacing} />
      )}
    </div>
  );
}
