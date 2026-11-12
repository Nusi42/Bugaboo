import { t } from "@bugaboo/messages";
import { TextField } from "@bugaboo/ui";

import type { ThemeValues } from "./themes.ts";
import { useTyped } from "./use-typed.ts";

export interface CheckedFieldProps {
  readonly label: string;
  readonly description?: string;
  readonly unit?: string;
  /** The theme's value, as the field shows it. */
  readonly value: string;
  /** What the values become with this text; `null` for a text that is no value at all. */
  readonly make: (text: string) => ThemeValues | null;
  readonly accepts: (values: ThemeValues) => boolean;
  readonly onChange: (values: ThemeValues) => void;
}

/** A field of *Shadow, corners and fonts* whose text counts once `bb-core` takes what it makes of it. */
export function CheckedField(props: CheckedFieldProps) {
  const { label, description, unit, value, make, accepts, onChange } = props;
  const field = useTyped(accepts, onChange);
  return (
    <TextField
      label={label}
      {...(description === undefined ? {} : { description })}
      {...(unit === undefined ? {} : { unit })}
      value={field.typed ?? value}
      errorMessage={field.typed === null ? undefined : t("theme-editor.not-allowed")}
      onChange={(text) => {
        field.offer(text, make(text));
      }}
    />
  );
}
