import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { SaveSearchButton } from "./SaveSearchButton.tsx";
import { useChangeSearch, type SavedSearch } from "./saved-searches.ts";

/** The buttons at the name of a saved search that was changed: *Save* for its maker, *Save a copy* for anybody else. */
export function ChangedActions({
  search,
  text,
}: {
  readonly search: SavedSearch;
  readonly text: string;
}) {
  const change = useChangeSearch(search.id);
  return (
    <>
      {search.mine ? (
        <Button
          look={buttonLook.primary}
          isPending={change.isPending}
          onPress={() => {
            change.mutate({ text, version: search.version });
          }}
        >
          {t("search.save-changes")}
        </Button>
      ) : (
        <SaveSearchButton
          text={text}
          presentation={search.presentation}
          copy
          startName={t("search.copy-name", { name: search.name })}
        />
      )}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </>
  );
}
