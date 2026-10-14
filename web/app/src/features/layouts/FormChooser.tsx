import { layoutKindWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { SelectField, partOf, part } from "@bugaboo/ui";

import { issueTypeLinesOf, useIssueTypes } from "../issue-types/index.ts";
import { kindName, type FormKind } from "./layouts.ts";

/** What the list of issue types holds for all of them: the word of no type in particular. */
const allTypes = "";

export interface FormChooserProps {
  readonly kind: FormKind;
  /** The ID of the issue type that is open; nothing for all types. */
  readonly type: string | undefined;
  readonly onChange: (kind: FormKind, type: string | undefined) => void;
}

/** Which layout is open: the form, and the issue type it is for. */
export function FormChooser({ kind, type, onChange }: FormChooserProps) {
  const types = issueTypeLinesOf(useIssueTypes().data?.pages);
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
      <SelectField
        label={t("layouts.type")}
        choices={[
          { value: allTypes, label: t("layouts.type-all") },
          ...types.map((each) => ({ value: each.id, label: each.name })),
        ]}
        value={type ?? allTypes}
        onChange={(next) => {
          onChange(kind, next === allTypes ? undefined : next);
        }}
      />
    </div>
  );
}
