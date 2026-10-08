import { t } from "@bugaboo/messages";
import { IconButton, iconName, SearchField, partOf, part } from "@bugaboo/ui";
import { useEffect, useRef, useState } from "react";
import { Autocomplete } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import { QuickResults } from "./QuickResults.tsx";
import { useQuickResults } from "./use-quick-results.ts";

/** The inside of the dialog, made new every time the dialog opens so the box starts empty. */
export function QuickSearchBox() {
  const [query, setQuery] = useState("");
  const found = useQuickResults(query);
  const field = useRef<HTMLDivElement>(null);
  // The dialog is made for typing: the box has the focus as soon as it appears.
  useEffect(() => {
    field.current?.querySelector("input")?.focus();
  }, []);
  return (
    <Autocomplete inputValue={query} onInputChange={setQuery}>
      <div {...partOf(part.quickSearch)}>
        <div {...partOf(part.quickSearchHead)}>
          <div ref={field} {...partOf(part.fill)}>
            <SearchField
              label={t("search.label")}
              placeholder={t("search.placeholder")}
              clearLabel={t("search.clear")}
              value={query}
              onChange={setQuery}
            />
          </div>
          <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        </div>
        <QuickResults found={found} />
        {found.failed ? <p {...partOf(part.quickSearchQuiet)}>{t("search.partial")}</p> : null}
      </div>
    </Autocomplete>
  );
}
