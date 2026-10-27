import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";

import type { ListView } from "../../shared/list-view.ts";
import { PlainList } from "./ListBody.tsx";
import { GroupedList } from "./ListGroups.tsx";

export interface ListResultsProps {
  readonly view: ListView;
  /** The text of the search has a mistake: the results of the last fixed search stay, dimmed. */
  readonly stale: boolean;
}

/** The rows of the list, in groups or not, dimmed while the text of the search has a mistake. */
export function ListResults({ view, stale }: ListResultsProps) {
  return (
    <div {...partOf(part.searchResults)}>
      {stale ? <output {...partOf(part.text)}>{t("search.found-stale")}</output> : null}
      <div {...partOf(part.searchResultsList, { states: [stale && "stale"] })}>
        {view.groupBy === null ? (
          <PlainList spacing={view.spacing} />
        ) : (
          <GroupedList spacing={view.spacing} />
        )}
      </div>
    </div>
  );
}
