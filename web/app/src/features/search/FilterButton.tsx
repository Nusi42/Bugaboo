import { t } from "@bugaboo/messages";
import type { ReactNode } from "react";
import { Button, ChoiceText, IconButton, iconName, part, partOf } from "@bugaboo/ui";

import type { FilterLabel } from "./filter-text.ts";

export interface FilterButtonProps {
  readonly label: FilterLabel;
  /** Opens the window that changes the filter. */
  readonly onChange: () => void;
  /** Takes the filter out of the search. */
  readonly onRemove: () => void;
}

/** The words of a filter for screen readers: `Status: not Done`, or the text alone. */
function spoken(label: FilterLabel): string {
  return label.name === undefined
    ? label.value
    : t("search.bar.name-value", { name: label.name, value: label.value });
}

/** The value of a filter: its words, with the choices it names (each with its number in the code font). */
function Value({ label }: { readonly label: FilterLabel }) {
  const parts: ReactNode[] = [];
  let rest = label.value;
  for (const choice of label.choices ?? []) {
    const at = rest.indexOf(choice.shownAs);
    if (at < 0) {
      // The words do not hold the choice as written: the plain words are the truth.
      return label.value;
    }
    parts.push(
      rest.slice(0, at),
      <ChoiceText key={choice.shownAs} number={choice.number} name={choice.name} />,
    );
    rest = rest.slice(at + choice.shownAs.length);
  }
  return [...parts, rest];
}

/**
 * One filter of Simple search as a button (UX design section 4.3): press it to change the filter,
 * or its ✕ to take it away. A filter with no picker is shown with a dashed outline and its text.
 */
export function FilterButton({ label, onChange, onRemove }: FilterButtonProps) {
  return (
    <li {...partOf(part.searchFilter, { variants: [label.asText && "text"] })}>
      <Button
        aria-label={t("search.bar.change-filter", { filter: spoken(label) })}
        onPress={onChange}
      >
        {label.name === undefined ? null : (
          <span {...partOf(part.searchFilterName)}>
            {t("search.bar.name-colon", { name: label.name })}
          </span>
        )}
        {label.name === undefined ? null : " "}
        <span {...partOf(part.searchFilterValue)}>
          <Value label={label} />
        </span>
      </Button>
      <IconButton
        icon={iconName.close}
        label={t("search.bar.remove-filter", { filter: spoken(label) })}
        onPress={onRemove}
      />
    </li>
  );
}
