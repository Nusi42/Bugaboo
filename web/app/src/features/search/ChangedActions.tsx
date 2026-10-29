import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { presentationOf, type ListView } from "../../shared/list-view.ts";
import { SaveSearchButton } from "./SaveSearchButton.tsx";
import { useChangeSearch, type SavedSearch } from "./saved-searches.ts";

/** Gives the changes up when asked. */
function UndoChanges({ onUndo }: { readonly onUndo: () => void }) {
  return (
    <Button look={buttonLook.quiet} onPress={onUndo}>
      {t("search.undo-changes")}
    </Button>
  );
}

/** What stands at the name of a saved search that was changed: that it is not saved, *Undo changes*, and *Save* for its maker or *Save a copy* for anybody else. */
export function ChangedActions({
  search,
  text,
  view,
  hasMistake,
  onSaved,
  onUndo,
}: {
  readonly search: SavedSearch;
  readonly text: string;
  /** How the results are shown now: kept with the text. */
  readonly view: ListView;
  /** The text in the box has a mistake: what would be kept is not what the person sees. */
  readonly hasMistake: boolean;
  /** The changes were saved: the address of the search holds no unsaved text any more. */
  readonly onSaved: () => void;
  /** The person gives up the changes: the address goes back to the saved search. */
  readonly onUndo: () => void;
}) {
  const change = useChangeSearch(search.id);
  return (
    <>
      <UndoChanges onUndo={onUndo} />
      {search.mine ? (
        <Button
          look={buttonLook.primary}
          isPending={change.isPending}
          isDisabled={hasMistake}
          onPress={() => {
            change.mutate(
              { text, presentation: presentationOf(view), version: search.version },
              { onSuccess: onSaved },
            );
          }}
        >
          {t("search.save-changes")}
        </Button>
      ) : (
        <SaveSearchButton
          text={text}
          presentation={presentationOf(view)}
          copy
          hasMistake={hasMistake}
          startName={t("search.copy-name", { name: search.name })}
        />
      )}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </>
  );
}
