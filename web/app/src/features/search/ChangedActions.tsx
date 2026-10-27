import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { SaveSearchButton } from "./SaveSearchButton.tsx";
import { useChangeSearch, type SavedSearch } from "./saved-searches.ts";

/** The buttons at the name of a saved search that was changed: *Save* for its maker, *Save a copy* for anybody else. */
export function ChangedActions({
  search,
  text,
  hasMistake,
  onSaved,
}: {
  readonly search: SavedSearch;
  readonly text: string;
  /** The text in the box has a mistake: what would be kept is not what the person sees. */
  readonly hasMistake: boolean;
  /** The changes were saved: the address of the search holds no unsaved text any more. */
  readonly onSaved: () => void;
}) {
  const change = useChangeSearch(search.id);
  return (
    <>
      {search.mine ? (
        <Button
          look={buttonLook.primary}
          isPending={change.isPending}
          isDisabled={hasMistake}
          onPress={() => {
            change.mutate({ text, version: search.version }, { onSuccess: onSaved });
          }}
        >
          {t("search.save-changes")}
        </Button>
      ) : (
        <SaveSearchButton
          text={text}
          presentation={search.presentation}
          copy
          hasMistake={hasMistake}
          startName={t("search.copy-name", { name: search.name })}
        />
      )}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </>
  );
}
