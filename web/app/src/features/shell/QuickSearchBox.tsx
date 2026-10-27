import { t } from "@bugaboo/messages";
import { Announce, IconButton, iconName, SearchField, partOf, part } from "@bugaboo/ui";
import { useEffect, useRef, useState } from "react";
import { Autocomplete } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import { QuickResults } from "./QuickResults.tsx";
import { useQuickResults, type QuickFound } from "./use-quick-results.ts";

/** What a screen reader says when the results change: how many there are, "No results" when there are none. */
function announcement(found: QuickFound): string {
  if (found.waiting) {
    return "";
  }
  const count = found.groups.reduce((sum, group) => sum + group.results.length, 0);
  return t("search.count", { count });
}

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
          <div ref={field} {...partOf(part.shellFill)}>
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
        <Announce>{announcement(found)}</Announce>
        {found.failed ? <p {...partOf(part.quickSearchQuiet)}>{t("search.partial")}</p> : null}
      </div>
    </Autocomplete>
  );
}
