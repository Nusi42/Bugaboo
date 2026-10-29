import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, overlayEdge, part, partOf } from "@bugaboo/ui";
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
  children,
}: PhoneFiltersProps) {
  const count = search === null ? 0 : filtersOf(search).length;
  return (
    <div {...partOf(part.searchBar)}>
      <div {...partOf(part.searchBarHead)}>
        <Button
          onPress={() => {
            onOpenChange(true);
          }}
        >
          {search === null ? t("search.bar.filters") : t("search.bar.filters-count", { count })}
        </Button>
      </div>
      <Overlay
        isOpen={open}
        onOpenChange={onOpenChange}
        label={t("search.bar.filters")}
        from={overlayEdge.bottom}
      >
        <div {...partOf(part.searchPanel, { variants: ["sheet"] })}>
          <div {...partOf(part.searchBar)}>
            {children}
            {listView === undefined ? null : (
              <div {...partOf(part.listOptions)}>
                <ViewChoices {...listView} fields={fields} />
              </div>
            )}
          </div>
          <div {...partOf(part.searchPanelActions)}>
            <Button
              look={buttonLook.primary}
              onPress={() => {
                onOpenChange(false);
              }}
            >
              {t("common.close")}
            </Button>
          </div>
        </div>
      </Overlay>
    </div>
  );
}
