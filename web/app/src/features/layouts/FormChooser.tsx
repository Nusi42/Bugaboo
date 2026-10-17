import { layoutKindWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { SelectField, partOf, part } from "@bugaboo/ui";

import { IssueTypeChooser } from "../issue-types/index.ts";
import { kindName, type FormKind } from "./layouts.ts";

export interface FormChooserProps {
  readonly kind: FormKind;
  /** The ID of the issue type that is open; nothing for all types. */
  readonly type: string | undefined;
  readonly onChange: (kind: FormKind, type: string | undefined) => void;
}

/** Which layout is open: the form, and the issue type it is for. */
export function FormChooser({ kind, type, onChange }: FormChooserProps) {
  return (
    <div {...partOf(part.row)}>
      <SelectField
        label={t("layouts.form")}
        choices={layoutKindWords.map((word) => ({ value: word, label: kindName(word) }))}
        value={kind}
        onChange={(next) => {
          onChange(next, type);
        }}
      />
      <IssueTypeChooser
        type={type}
        onChange={(next) => {
          onChange(kind, next);
        }}
      />
    </div>
  );
}
