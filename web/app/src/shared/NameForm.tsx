import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "./errors.ts";

/** Where saving the name stands: the shape of the mutation that saves it. */
export interface NameSaving {
  readonly isPending: boolean;
  readonly isError: boolean;
  readonly isSuccess: boolean;
  readonly error: unknown;
}

export interface NameFormProps {
  /** The label of the field: *Name of the group*, *Name of the role*. */
  readonly label: string;
  /** The name as it was saved. */
  readonly name: string;
  readonly saving: NameSaving;
  /** Saves the name typed, without spaces at its ends. */
  readonly onSave: (name: string) => void;
}

/**
 * *Rename*: the name of a thing (a group, a role), saved with the button, with what the server answered under it. The one
 * form of a name, for every thing that has one.
 */
export function NameForm({ label, name, saving, onSave }: NameFormProps) {
  const [typed, setTyped] = useState(name);
  return (
    <Form
      onSubmit={() => {
        onSave(typed.trim());
      }}
    >
      <TextField
        label={label}
        requiredMessage={t("error.field.required")}
        value={typed}
        onChange={setTyped}
      />
      {saving.isError ? <Alert tone={alertTone.danger}>{messageOf(saving.error)}</Alert> : null}
      {saving.isSuccess ? <Alert tone={alertTone.success}>{t("save.saved")}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={saving.isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}
