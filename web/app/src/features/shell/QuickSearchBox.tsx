import { t } from "@bugaboo/messages";
import { Announce, IconButton, iconName, SearchField, partOf, part } from "@bugaboo/ui";
import { useEffect, useRef, useState } from "react";
import { Autocomplete } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import { QuickResults } from "./QuickResults.tsx";
import { announcement } from "./quick-search-announce.ts";
import { useOpenResult } from "./use-open-result.ts";
import { useQuickResults } from "./use-quick-results.ts";
import { useReadyFirst } from "./use-ready-first.ts";

/** The inside of the dialog, made new every time the dialog opens so the box starts empty. */
export function QuickSearchBox() {
  const [query, setQuery] = useState("");
  const found = useQuickResults(query);
  const open = useOpenResult(found.groups);
  const ready = useReadyFirst(found.groups, open);
  const field = useRef<HTMLDivElement>(null);
  const changed = (text: string): void => {
    setQuery(text);
    ready.wordsChanged();
  };
  // The dialog is made for typing: the box has the focus as soon as it appears.
  useEffect(() => {
    field.current?.querySelector("input")?.focus();
  }, []);
  return (
    <Autocomplete disableAutoFocusFirst inputValue={query} onInputChange={changed}>
      <div {...partOf(part.quickSearch)} onKeyDownCapture={ready.onKeyDownCapture}>
        <div {...partOf(part.quickSearchHead)}>
          <div ref={field} {...partOf(part.shellFill)}>
            <SearchField
              label={t("search.label")}
              placeholder={t("search.placeholder")}
              clearLabel={t("search.clear")}
              value={query}
              onChange={changed}
            />
          </div>
          <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        </div>
        <QuickResults found={found} firstIsReady={ready.firstIsReady} onOpen={open} />
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
