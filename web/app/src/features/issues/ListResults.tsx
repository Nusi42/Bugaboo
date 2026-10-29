import { Alert, alertTone, Icon, iconName, part, partOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import type { ListView } from "../../shared/list-view.ts";
import { PlainList } from "./ListBody.tsx";
import { GroupedList } from "./ListGroups.tsx";
import { useListContext } from "./list-context.ts";

export interface ListResultsProps {
  readonly view: ListView;
  /** The text of the search has a mistake: the results of the last fixed search stay, edged and noted. */
  readonly stale: boolean;
}

/**
 * The rows of the list, in groups or not, or why there are none. While the text of the search has
 * a mistake the rows of the last fixed search stay, with a dashed edge round the list and a note
 * above it (the count line); a search that was refused has no earlier list, so its error says it all.
 */
export function ListResults({ view, stale }: ListResultsProps) {
  const { failure } = useListContext();
  if (failure !== null) {
    return (
      <Alert tone={alertTone.danger}>
        <span {...partOf(part.row, { variants: ["inline"] })}>
          <Icon name={iconName.danger} />
          {messageOf(failure)}
        </span>
      </Alert>
    );
  }
  return (
    <div {...partOf(part.searchResults, { states: [stale && "stale"] })}>
      {view.groupBy === null ? (
        <PlainList spacing={view.spacing} />
      ) : (
        <GroupedList spacing={view.spacing} />
      )}
    </div>
  );
}
