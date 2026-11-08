import { t } from "@bugaboo/messages";
import { Alert, alertTone, Switch, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { type OwnField } from "./fields.ts";
import { fieldChange, useChangeField } from "./use-change-field.ts";

/**
 * *Never sent to AI* for the fields an administrator made (AI-03): one switch for each, beside the
 * built-in ones, so the whole list of what AI never gets is in one place. The same switch is on the
 * page of each field.
 */
export function OwnNeverSent({ fields }: { readonly fields: readonly OwnField[] }) {
  const change = useChangeField();
  if (fields.length === 0) {
    return null;
  }
  return (
    <div {...partOf(part.stack)}>
      <ul {...partOf(part.list, { variants: ["tight"] })} aria-label={t("fields.list")}>
        {fields.map((field) => (
          <li key={field.id}>
            <Switch
              label={field.name}
              isSelected={field.never_to_assistant}
              onChange={(never) => {
                change.mutate(fieldChange(field, { never_to_assistant: never }));
              }}
            />
          </li>
        ))}
      </ul>
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </div>
  );
}
