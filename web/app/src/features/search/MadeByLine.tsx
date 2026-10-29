import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";

import { personName } from "../../shared/person.ts";
import type { ListView } from "../../shared/list-view.ts";
import { ChangedActions } from "./ChangedActions.tsx";
import type { SavedSearch } from "./saved-searches.ts";

export interface MadeByLineProps {
  readonly search: SavedSearch;
  readonly text: string;
  readonly view: ListView;
  /** The text and the view are the saved ones. */
  readonly unchanged: boolean;
  readonly hasMistake: boolean;
  readonly onSaved: () => void;
  readonly onUndo: () => void;
}

/**
 * The line under the name of a saved search: who made it, whether it was changed, and what is done
 * with a change. The room of all of it is held while nothing is changed, so nothing under the name
 * moves when *Changed, not saved* appears (S29).
 */
export function MadeByLine({
  search,
  text,
  view,
  unchanged,
  hasMistake,
  onSaved,
  onUndo,
}: MadeByLineProps) {
  const held = { states: [unchanged && "held"] } as const;
  return (
    <div {...partOf(part.searchMadeBy)}>
      <div>
        <p {...partOf(part.muted)}>{t("search.made-by", { person: personName(search.maker) })}</p>
      </div>
      <span {...partOf(part.searchHeldActions, held)} inert={unchanged}>
        <ChangedActions
          search={search}
          text={text}
          view={view}
          hasMistake={hasMistake}
          onSaved={onSaved}
          onUndo={onUndo}
        />
      </span>
      <p {...partOf(part.muted)} aria-hidden={unchanged}>
        <span {...partOf(part.searchHeldActions, held)}>{t("search.changed-not-saved")}</span>
      </p>
    </div>
  );
}
