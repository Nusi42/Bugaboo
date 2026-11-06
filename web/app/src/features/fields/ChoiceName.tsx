import { t } from "@bugaboo/messages";

import { ChoiceForm } from "./ChoiceForm.tsx";
import type { Choice } from "./fields.ts";

/** Sends a change of the choice, on the version of the field the person looked at. */
type Send = (more: Readonly<Record<string, unknown>>) => void;

/** The name and the number of the choice, saved with their own button. */
export function ChoiceName({
  choice,
  send,
  pending,
  failure,
}: {
  readonly choice: Choice;
  readonly send: Send;
  readonly pending: boolean;
  /** What the last change failed with, if it did. */
  readonly failure: unknown;
}) {
  return (
    <ChoiceForm
      nameLabel={t("fields.choice-name")}
      submitLabel={t("common.save")}
      first={{ name: choice.name, number: choice.number }}
      pending={pending}
      failure={failure}
      onSubmit={(given) => {
        send({ name: given.name, number: given.number });
      }}
    />
  );
}
