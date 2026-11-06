import { Alert, alertTone, Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useChoiceNumberBox } from "./choice-number.ts";
import { ChoiceNumberBox } from "./ChoiceNumberBox.tsx";

/** What a person gave for a choice: its name, and its number (none for no number). */
export interface ChoiceGiven {
  readonly name: string;
  readonly number: string | null;
}

export interface ChoiceFormProps {
  /** The name of the box of the name. */
  readonly nameLabel: string;
  /** The words of the button that sends the form. */
  readonly submitLabel: string;
  /** What the boxes start with; nothing for a new choice. */
  readonly first?: { readonly name: string; readonly number: string | null | undefined };
  readonly pending: boolean;
  /** What the last send failed with, if it did. */
  readonly failure: unknown;
  /** The form was sent with a name and a number that are fit to send; `clear` empties the boxes. */
  readonly onSubmit: (given: ChoiceGiven, clear: () => void) => void;
}

/**
 * The form of a choice of an own field, for one that is there and for a new one: the box of the
 * name, the box of the number, the button. A number that another choice has is told at the box of
 * the number; any other failure under the form.
 */
export function ChoiceForm({
  nameLabel,
  submitLabel,
  first,
  pending,
  failure,
  onSubmit,
}: ChoiceFormProps) {
  const [name, setName] = useState(first?.name ?? "");
  const number = useChoiceNumberBox(first?.number);
  const { problem, other } = number.split(failure);
  return (
    <Form
      onSubmit={() => {
        const typed = number.read();
        if (typed !== undefined) {
          onSubmit({ name: name.trim(), number: typed }, () => {
            setName("");
            number.clear();
          });
        }
      }}
    >
      <TextField
        label={nameLabel}
        requiredMessage={t("error.field.required")}
        value={name}
        onChange={setName}
      />
      <ChoiceNumberBox
        text={number.text}
        onChange={number.setText}
        invalid={number.invalid}
        problem={problem}
      />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={pending}>{submitLabel}</SubmitButton>
      </div>
      {other === undefined ? null : <Alert tone={alertTone.danger}>{messageOf(other)}</Alert>}
    </Form>
  );
}
