import { Input, Label, SearchField as AriaSearchField } from "react-aria-components";

import { cx } from "./cx.ts";
import { Icon } from "./Icon.tsx";
import { iconName } from "./icons.ts";
import { IconButton } from "./IconButton.tsx";
import styles from "./SearchField.module.css";
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
}

/** A box to type a search in, with a button that empties it. */
export function SearchField({ label, value, onChange, clearLabel, placeholder }: SearchFieldProps) {
  return (
    <AriaSearchField className={cx(styles.search)} value={value} onChange={onChange}>
      <Label {...partOf(part.fieldLabel)}>{label}</Label>
      <div className={styles.box}>
        <Icon name={iconName.search} size={iconSize.small} />
        <Input
          className={cx(styles.input)}
          {...(placeholder === undefined ? {} : { placeholder })}
        />
        {value === "" ? null : (
          <ClearButton
            label={clearLabel}
            onPress={() => {
              onChange("");
            }}
          />
        )}
      </div>
    </AriaSearchField>
  );
}

function ClearButton({ label, onPress }: { readonly label: string; readonly onPress: () => void }) {
  return <IconButton icon={iconName.close} label={label} onPress={onPress} />;
}
