import { Input, Label, SearchField as AriaSearchField } from "react-aria-components";

import { Icon } from "./Icon.tsx";
import { iconName } from "./icons.ts";
import { IconButton } from "./IconButton.tsx";
import { partOf } from "./parts.ts";
import { iconSize } from "./props.ts";
import { part } from "./tokens.ts";

export interface SearchFieldProps {
  /** The visible name of the box, already in the person's language. */
  readonly label: string;
  readonly value: string;
  readonly onChange: (value: string) => void;
  /** The name of the button that empties the box. */
  readonly clearLabel: string;
  /** What the box is for, shown while it is empty: "Search issues and settings". */
  readonly placeholder?: string;
  /** Another label stands over the box already (the legend of a picker): this one is for screen readers only. */
  readonly hideLabel?: boolean;
}

/** A box to type a search in, with a button that empties it. */
export function SearchField({
  label,
  value,
  onChange,
  clearLabel,
  placeholder,
  hideLabel = false,
}: SearchFieldProps) {
  return (
    <AriaSearchField {...partOf(part.field)} value={value} onChange={onChange}>
      <Label {...partOf(hideLabel ? part.visuallyHidden : part.fieldLabel)}>{label}</Label>
      <div {...partOf(part.searchFieldBox)}>
        <Icon name={iconName.search} size={iconSize.small} />
        <Input
          {...partOf(part.searchFieldInput)}
          {...(placeholder === undefined ? {} : { placeholder })}
        />
        {/* The place of the clear mark is kept while the box is empty, so the box does not narrow under
            the text when the first letter brings the mark (M5 exit F-A7: the end of the text hid). */}
        <span {...partOf(part.searchFieldClear)}>
          {value === "" ? null : (
            <ClearButton
              label={clearLabel}
              onPress={() => {
                onChange("");
              }}
            />
          )}
        </span>
      </div>
    </AriaSearchField>
  );
}

function ClearButton({ label, onPress }: { readonly label: string; readonly onPress: () => void }) {
  return <IconButton icon={iconName.close} label={label} onPress={onPress} />;
}
