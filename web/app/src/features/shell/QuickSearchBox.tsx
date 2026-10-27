import { t } from "@bugaboo/messages";
import { Announce, IconButton, iconName, SearchField, partOf, part } from "@bugaboo/ui";
import { useEffect, useRef, useState } from "react";
import { Autocomplete } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import { QuickResults } from "./QuickResults.tsx";
import { announcement } from "./quick-search-announce.ts";
import { useFollowFirst } from "./use-follow-first.ts";
import { useOpenResult } from "./use-open-result.ts";
import { useQuickResults } from "./use-quick-results.ts";

/** The inside of the dialog, made new every time the dialog opens so the box starts empty. */
export function QuickSearchBox() {
  const [query, setQuery] = useState("");
  const found = useQuickResults(query);
  const open = useOpenResult(found.groups);
  const root = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLDivElement>(null);
  // The first result has the focus of the list, and so is what Enter opens, until the person moves it.
  // The list does it itself (`disableAutoFocusFirst`): the box's own way waits half a second and
  // would leave the box pointing at a result that is gone.
  useFollowFirst(root, found.groups, query);
  // The dialog is made for typing: the box has the focus as soon as it appears.
  useEffect(() => {
    field.current?.querySelector("input")?.focus();
  }, []);
  return (
    <Autocomplete disableAutoFocusFirst inputValue={query} onInputChange={setQuery}>
      <div ref={root} {...partOf(part.quickSearch)}>
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
        <QuickResults found={found} onOpen={open} />
        <Announce>
          {announcement(found).map((sentence) => (
            <span key={sentence}>{sentence}</span>
          ))}
        </Announce>
        {found.failed ? <p {...partOf(part.quickSearchQuiet)}>{t("search.partial")}</p> : null}
      </div>
    </Autocomplete>
  );
}
