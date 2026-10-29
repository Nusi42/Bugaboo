import { t } from "@bugaboo/messages";
import { Button, Icon, iconName, part, partOf } from "@bugaboo/ui";
import type { Expr, SearchLanguage } from "@bugaboo/wasm-search";

import { FilterButtons } from "./FilterButtons.tsx";

export interface AnyOfFiltersProps {
  readonly language: SearchLanguage;
  readonly filters: readonly Expr[];
  /** A filter was pressed: it is changed in its own window. */
  readonly onChange: (filter: Expr, index: number) => void;
  readonly onRemove: (index: number) => void;
  /** *+ Add a filter* was pressed. */
  readonly onAdd: () => void;
}

/** The filters of an *Any of* as buttons, like those of Simple search, then *+ Add a filter* on a row of its own. */
export function AnyOfFilters({ language, filters, onChange, onRemove, onAdd }: AnyOfFiltersProps) {
  return (
    <ul {...partOf(part.searchFilters)} aria-label={t("search.bar.any-of-filters")}>
      <FilterButtons
        language={language}
        filters={filters}
        onChange={onChange}
        onRemove={onRemove}
      />
      <li {...partOf(part.searchFiltersAction)}>
        <Button onPress={onAdd}>
          <Icon name={iconName.plus} />
          {t("search.bar.any-of-add")}
        </Button>
      </li>
    </ul>
  );
}
