import { t } from "@bugaboo/messages";
import { BottomSheet, Button, part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { filtersOf } from "../../shared/search-tree.ts";
import type { SearchFieldsAnswer } from "./search-fields.ts";
import { ViewChoices, type ListViewControl } from "./ViewChoices.tsx";

export interface PhoneFiltersProps {
  /** The search in the box, or none while its text has a mistake. */
  readonly search: Parameters<typeof filtersOf>[0] | null;
  /** How many mistakes the text has (A10). */
  readonly mistakes: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** The names of the fields the person reads, for *Group by*. */
  readonly fields: SearchFieldsAnswer;
  readonly listView: ListViewControl | undefined;
  /** Buttons before *Filters* in its row. */
  readonly start?: ReactNode;
  /** What is done with the search (*Save search*, *Share*), next to the button that is done. */
  readonly footer?: ReactNode;
  /** The bar itself, which the sheet holds. */
  readonly children: ReactNode;
}

/** What the one button of the bar of a phone says: how many filters, if any, and how many mistakes the text has. */
function filtersWords(search: PhoneFiltersProps["search"], mistakes: number): string {
  const count = search === null ? 0 : filtersOf(search).length;
  const filters = count === 0 ? t("search.bar.filters") : t("search.bar.filters-count", { count });
  return mistakes === 0 ? filters : t("search.bar.with-mistakes", { filters, count: mistakes });
}

/**
 * The search bar of a phone: one button, *Filters*, with a count only where there is one (and the
 * mistakes of the text), and the sheet from the bottom it opens. The sheet has a title, holds the bar,
 * then *Group by* (M5 exit decision D2), then what is done with the search and the button that
 * says it is done. *Spacing* is the person's, in *My settings › Appearance* (ADR-0034, amendment).
 */
export function PhoneFilters({
  search,
  mistakes,
  open,
  onOpenChange,
  fields,
  listView,
  start,
  footer,
  children,
}: PhoneFiltersProps) {
  return (
    <div {...partOf(part.searchBar)}>
      <div {...partOf(part.searchBarHead)}>
        {start}
        <Button
          onPress={() => {
            onOpenChange(true);
          }}
        >
          {filtersWords(search, mistakes)}
        </Button>
      </div>
      <BottomSheet
        isOpen={open}
        onOpenChange={onOpenChange}
        label={t("search.bar.filters")}
        title={t("search.bar.filters")}
        footer={footer}
        closeLabel={t("search.bar.sheet-done")}
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
