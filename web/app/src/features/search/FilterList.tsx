import { t } from "@bugaboo/messages";
import { Button, part, partOf } from "@bugaboo/ui";
import type { Expr, Field, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";

import { AddFilter } from "./AddFilter.tsx";
import { FilterButton } from "./FilterButton.tsx";
import { useFieldName } from "./field-names.ts";
import { filterLabel, sourceOf } from "./filter-text.ts";

export interface FilterListProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  readonly filters: readonly Expr[];
  readonly onChange: (filter: Expr, index: number) => void;
  readonly onRemove: (index: number) => void;
  readonly onAdd: (field: Field) => void;
  readonly onAnyOf: () => void;
}

/** What tells one filter from the others: its place and its text (two filters may be the same). */
function keyOf(language: SearchLanguage, filter: Expr, index: number): string {
  return `${String(index)}:${sourceOf(language, filter)}`;
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
  const nameOf = useFieldName();
  return (
    <ul {...partOf(part.searchFilters)} aria-label={t("search.bar.filters")}>
      {filters.map((filter, index) => (
        <FilterButton
          key={keyOf(language, filter, index)}
          label={filterLabel(language, filter, nameOf)}
          onChange={() => {
            onChange(filter, index);
          }}
          onRemove={() => {
            onRemove(index);
          }}
        />
      ))}
      <li {...partOf(part.searchFiltersAction)}>
        <AddFilter fields={fields} onPick={onAdd} />
      </li>
      <li {...partOf(part.searchFiltersAction)}>
        <Button onPress={onAnyOf}>{t("search.bar.any-of")}</Button>
      </li>
    </ul>
  );
}
