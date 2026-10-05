import { t } from "@bugaboo/messages";
import { IconButton, iconName, SearchField } from "@bugaboo/ui";
import { useState } from "react";
import { Autocomplete } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import styles from "./QuickSearch.module.css";
import { QuickResults } from "./QuickResults.tsx";
import { useQuickResults } from "./use-quick-results.ts";

/** The inside of the dialog, made new every time the dialog opens so the box starts empty. */
export function QuickSearchBox() {
  const [query, setQuery] = useState("");
  const groups = useQuickResults(query);
  return (
    <div className={styles.search}>
      <div className={styles.head}>
        <div className={styles.field}>
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
      <Autocomplete inputValue={query} onInputChange={setQuery}>
        <QuickResults groups={groups} />
      </Autocomplete>
    </div>
  );
}
