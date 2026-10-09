import { issueIconWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { SelectField, type IssueTypeIconName } from "@bugaboo/ui";

const choices = issueIconWords.map((word) => ({ value: word, label: t(`issue-icon.${word}`) }));

export interface IconFieldProps {
  readonly value: IssueTypeIconName;
  readonly onChange: (icon: IssueTypeIconName) => void;
}

/** *Icon*: one of the fixed list of icons an issue type can have, by name. */
export function IconField({ value, onChange }: IconFieldProps) {
  return (
    <SelectField
      label={t("issue-types.field-icon")}
      choices={choices}
      value={value}
      onChange={onChange}
    />
  );
}
