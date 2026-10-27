import { t } from "@bugaboo/messages";
import { Button } from "@bugaboo/ui";
import { useState } from "react";

import type { SavedSearch } from "./saved-searches.ts";
import { ShareWindow } from "./ShareWindow.tsx";

/** *Share* opens the window that chooses the groups and projects a saved search is shared with. */
export function ShareButton({ search }: { readonly search: SavedSearch }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("search.share")}
      </Button>
      <ShareWindow
        search={search}
        isOpen={open}
        onClose={() => {
          setOpen(false);
        }}
      />
    </>
  );
}
