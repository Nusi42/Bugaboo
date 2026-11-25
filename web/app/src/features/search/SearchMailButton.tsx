import { t } from "@bugaboo/messages";
import { Button } from "@bugaboo/ui";
import { useState } from "react";

import type { SavedSearch } from "./saved-searches.ts";
import { SearchMailWindow } from "./SearchMailWindow.tsx";

/** *E-mail me this search* beside any saved search the person may open (QRY-07). */
export function SearchMailButton({ search }: { readonly search: SavedSearch }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("search.mail-me")}
      </Button>
      <SearchMailWindow
        search={search}
        isOpen={open}
        onClose={() => {
          setOpen(false);
        }}
      />
    </>
  );
}
