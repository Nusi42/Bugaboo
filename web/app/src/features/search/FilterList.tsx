import { t } from "@bugaboo/messages";
import { Button, part, partOf } from "@bugaboo/ui";
import type { Expr, Field, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";

import { AddFilter } from "./AddFilter.tsx";
import { FilterButtons } from "./FilterButtons.tsx";

export interface FilterListProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  readonly filters: readonly Expr[];
  readonly onChange: (filter: Expr, index: number) => void;
  readonly onRemove: (index: number) => void;
  readonly onAdd: (field: Field) => void;
  readonly onAnyOf: () => void;
}

/** The filters of the search as buttons, then *+ Add filter* and *Any of* (UX design section 4.3). */
export function FilterList({
  language,
  fields,
  filters,
  onChange,
  onRemove,
  onAdd,
  onAnyOf,
}: FilterListProps) {
  return (
    <ul {...partOf(part.searchFilters)} aria-label={t("search.bar.filters")}>
      <FilterButtons
        language={language}
        filters={filters}
        onChange={onChange}
        onRemove={onRemove}
      />
      <li {...partOf(part.searchFiltersAction)}>
        <AddFilter fields={fields} onPick={onAdd} />
      </li>
      <li {...partOf(part.searchFiltersAction)}>
        <Button onPress={onAnyOf}>{t("search.bar.any-of")}</Button>
      </li>
    </ul>
  );
}
