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
 * The line under the name of a saved search: who made it, or, once it is changed, that the change is
 * not saved, with *Undo changes* and *Save* at its end. It is always as high as the buttons, so
 * nothing under the name moves when the state changes (S29), and it is one line, never a band.
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
  return (
    <div {...partOf(part.searchMadeBy, { states: [!unchanged && "changed"] })}>
      <p {...partOf(part.muted)}>
        {unchanged
          ? t("search.made-by", { person: personName(search.maker) })
          : t("search.changed-not-saved")}
      </p>
      <span
        {...partOf(part.searchHeldActions, { states: [unchanged && "held"] })}
        inert={unchanged}
      >
        <ChangedActions
          search={search}
          text={text}
          view={view}
          hasMistake={hasMistake}
          onSaved={onSaved}
          onUndo={onUndo}
        />
      </span>
    </div>
  );
}
