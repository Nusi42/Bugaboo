import type { Expr, SearchLanguage } from "@bugaboo/wasm-search";

import { FilterButton } from "./FilterButton.tsx";
import { useFieldName } from "./field-name-context.ts";
import { filterLabel, sourceOf } from "./filter-text.ts";

export interface FilterButtonsProps {
  readonly language: SearchLanguage;
  readonly filters: readonly Expr[];
  /** A filter was pressed: it is changed in its own window. */
  readonly onChange: (filter: Expr, index: number) => void;
  /** The ✕ of a filter was pressed. */
  readonly onRemove: (index: number) => void;
}

/** What tells one filter from the others: its place and its text (two filters may be the same). */
function keyOf(language: SearchLanguage, filter: Expr, index: number): string {
  return `${String(index)}:${sourceOf(language, filter)}`;
}

/** The filters as buttons (UX design section 4.3): the rows of a list of filters, in the search bar and in an *Any of*. */
export function FilterButtons({ language, filters, onChange, onRemove }: FilterButtonsProps) {
  const nameOf = useFieldName();
  return filters.map((filter, index) => (
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
  ));
}
