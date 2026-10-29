import { t } from "@bugaboo/messages";
import { Button, Icon, iconName, part, partOf } from "@bugaboo/ui";
import type { Expr, SearchLanguage } from "@bugaboo/wasm-search";

import { FilterButton } from "./FilterButton.tsx";
import { useFieldName } from "./field-name-context.ts";
import { filterLabel, sourceOf } from "./filter-text.ts";

export interface AnyOfConditionsProps {
  readonly language: SearchLanguage;
  readonly conditions: readonly Expr[];
  /** A condition was pressed: it is changed in its own window. */
  readonly onChange: (condition: Expr, index: number) => void;
  readonly onRemove: (index: number) => void;
  /** *+ Add a condition* was pressed. */
  readonly onAdd: () => void;
}

/** What tells one condition from the others: its place and its text (two may be the same). */
function keyOf(language: SearchLanguage, condition: Expr, index: number): string {
  return `${String(index)}:${sourceOf(language, condition)}`;
}

/** The conditions of an *Any of* as buttons, like the filters of Simple search, then *+ Add a condition*. */
export function AnyOfConditions({
  language,
  conditions,
  onChange,
  onRemove,
  onAdd,
}: AnyOfConditionsProps) {
  const nameOf = useFieldName();
  return (
    <ul {...partOf(part.searchFilters)} aria-label={t("search.bar.any-of-conditions")}>
      {conditions.map((condition, index) => (
        <FilterButton
          key={keyOf(language, condition, index)}
          label={filterLabel(language, condition, nameOf)}
          onChange={() => {
            onChange(condition, index);
          }}
          onRemove={() => {
            onRemove(index);
          }}
        />
      ))}
      <li {...partOf(part.searchFiltersAction)}>
        <Button onPress={onAdd}>
          <Icon name={iconName.plus} />
          {t("search.bar.any-of-add")}
        </Button>
      </li>
    </ul>
  );
}
