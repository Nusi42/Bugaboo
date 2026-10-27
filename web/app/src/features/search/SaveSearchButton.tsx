import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, Button, TextField } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { savedSearchAddress } from "../../shared/paths.ts";
import { useSaveSearch, type Presentation, type SavedSearch } from "./saved-searches.ts";

export interface SaveSearchButtonProps {
  /** The canonical text of the search to keep. */
  readonly text: string;
  /** How the results are shown (columns, grouping); the list as the project has it when left out. */
  readonly presentation?: Presentation | undefined;
  /** What the button says, and the window with it: *Save search*, or *Save a copy*. */
  readonly copy?: boolean;
  /** The name the box starts with. */
  readonly startName?: string;
}

/**
 * *Save search* (QRY-06): asks for a name and keeps the search with how its results are shown. It
 * is private at first; the page of the saved search is opened. *Save a copy* is the same for a
 * search somebody else made.
 */
export function SaveSearchButton({
  text,
  presentation,
  copy = false,
  startName = "",
}: SaveSearchButtonProps) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState(startName);
  const save = useSaveSearch();
  const navigate = useNavigate();
  return (
    <>
      <Button
        isDisabled={save.isPending}
        onPress={() => {
          setName(startName);
          setOpen(true);
        }}
      >
        {copy ? t("search.save-copy") : t("search.save")}
      </Button>
      <AskDialog
        isOpen={open}
        onOpenChange={setOpen}
        title={copy ? t("search.save-copy") : t("search.save")}
        text={t("search.save-text")}
        confirmLabel={copy ? t("search.save-copy-confirm") : t("search.save-confirm")}
        cancelLabel={t("common.cancel")}
        confirmDisabled={name.trim().length === 0}
        onConfirm={() => {
          save.mutate(
            { name: name.trim(), text, presentation },
            {
              onSuccess: (saved: SavedSearch) => {
                void navigate({ href: savedSearchAddress(saved.id) });
              },
            },
          );
        }}
      >
        <TextField label={t("search.save-name")} value={name} onChange={setName} />
      </AskDialog>
      {save.isError ? <Alert tone={alertTone.danger}>{messageOf(save.error)}</Alert> : null}
    </>
  );
}
