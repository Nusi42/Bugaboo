import { t } from "@bugaboo/messages";
import { BottomSheet, Button, part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { filtersOf } from "../../shared/search-tree.ts";
import type { SearchFieldsAnswer } from "./search-fields.ts";
import { ViewChoices, type ListViewControl } from "./ViewChoices.tsx";

export interface PhoneFiltersProps {
  /** The search in the box, or none while its text has a mistake. */
  readonly search: Parameters<typeof filtersOf>[0] | null;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** The names of the fields the person reads, for *Group by*. */
  readonly fields: SearchFieldsAnswer;
  readonly listView: ListViewControl | undefined;
  /** Buttons before *Filters* in its row. */
  readonly start?: ReactNode;
  /** The bar itself, which the sheet holds. */
  readonly children: ReactNode;
}

/**
 * The search bar of a phone: one button, *Filters (n)*, and the sheet from the bottom it opens. The
 * sheet holds the bar, then *Group by* and *Spacing* (M5 exit decision D2), then a button that closes it.
 */
export function PhoneFilters({
  search,
  open,
  onOpenChange,
  fields,
  listView,
  start,
  children,
}: PhoneFiltersProps) {
  const count = search === null ? 0 : filtersOf(search).length;
  return (
    <div {...partOf(part.searchBar)}>
      <div {...partOf(part.searchBarHead)}>
        {start}
        <Button
          onPress={() => {
            onOpenChange(true);
          }}
        >
          {count === 0 ? t("search.bar.filters") : t("search.bar.filters-count", { count })}
        </Button>
      </div>
      <BottomSheet
        isOpen={open}
        onOpenChange={onOpenChange}
        label={t("search.bar.filters")}
        title={t("search.bar.filters")}
        closeLabel={t("common.close")}
        tall
      >
        <div {...partOf(part.searchBar)}>
          {children}
          {listView === undefined ? null : (
            <div {...partOf(part.listOptions)}>
              <ViewChoices {...listView} fields={fields} />
            </div>
          )}
        </div>
      </BottomSheet>
    </div>
  );
}
