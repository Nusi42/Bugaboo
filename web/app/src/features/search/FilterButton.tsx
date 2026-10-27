import { t } from "@bugaboo/messages";
import { Button, buttonLook, IconButton, iconName, part, partOf } from "@bugaboo/ui";

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

/**
 * One filter of Simple search as a button (UX design section 4.3): press it to change the filter,
 * or its ✕ to take it away. A filter with no picker is shown with a dashed outline and its text.
 */
export function FilterButton({ label, onChange, onRemove }: FilterButtonProps) {
  return (
    <li {...partOf(part.searchFilter, { variants: [label.asText && "text"] })}>
      <Button
        look={buttonLook.quiet}
        aria-label={t("search.bar.change-filter", { filter: spoken(label) })}
        onPress={onChange}
      >
        {label.name === undefined ? null : (
          <span {...partOf(part.searchFilterName)}>
            {t("search.bar.name-colon", { name: label.name })}
          </span>
        )}
        <span {...partOf(part.searchFilterValue)}>{label.value}</span>
      </Button>
      <IconButton
        icon={iconName.close}
        label={t("search.bar.remove-filter", { filter: spoken(label) })}
        onPress={onRemove}
      />
    </li>
  );
}
