import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, overlayEdge, part, partOf } from "@bugaboo/ui";
import type { Search } from "@bugaboo/wasm-search";
import type { ReactNode } from "react";

import { filtersOf } from "../../shared/search-tree.ts";

/** The bar in a sheet from the bottom of a phone, with a title and a button that says it is done. */
export function PhoneSheet({
  open,
  onOpenChange,
  footer,
  children,
}: {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** What is done with the search (*Save search*, *Share*), next to the button that is done. */
  readonly footer?: ReactNode;
  readonly children: ReactNode;
}) {
  return (
    <Overlay
      isOpen={open}
      onOpenChange={onOpenChange}
      label={t("search.bar.filters")}
      from={overlayEdge.bottom}
    >
      <div {...partOf(part.searchPanel, { variants: ["sheet"] })}>
        <h2 {...partOf(part.searchPanelTitle)}>{t("search.bar.filters")}</h2>
        <div {...partOf(part.searchBar)}>{children}</div>
        <div {...partOf(part.searchPanelActions)}>
          {footer}
          <Button
            look={buttonLook.primary}
            onPress={() => {
              onOpenChange(false);
            }}
          >
            {t("search.bar.sheet-done")}
          </Button>
        </div>
      </div>
    </Overlay>
  );
}

/** The one button of the bar on a phone: how many filters, and how many mistakes the text has (A10). */
export function FiltersButton({
  search,
  mistakes,
  onPress,
}: {
  readonly search: Search | null;
  readonly mistakes: number;
  readonly onPress: () => void;
}) {
  const filters =
    search === null
      ? t("search.bar.filters")
      : t("search.bar.filters-count", { count: filtersOf(search).length });
  return (
    <Button onPress={onPress}>
      {mistakes === 0 ? filters : t("search.bar.with-mistakes", { filters, count: mistakes })}
    </Button>
  );
}
