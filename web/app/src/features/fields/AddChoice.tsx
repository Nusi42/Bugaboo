import { t } from "@bugaboo/messages";

import { ChoiceForm } from "./ChoiceForm.tsx";
import type { OwnField } from "./fields.ts";
import { useAddChoice } from "./use-change-field.ts";

/** The form that adds a choice at the end. */
export function AddChoice({ field }: { readonly field: OwnField }) {
  const add = useAddChoice();
  return (
    <ChoiceForm
      nameLabel={t("fields.add-choice")}
      submitLabel={t("fields.add-choice-button")}
      pending={add.isPending}
      failure={add.error}
      onSubmit={(given, clear) => {
        add.mutate(
          {
            params: { path: { field: field.id } },
            body: { version: field.version, name: given.name, number: given.number },
          },
          { onSuccess: clear },
        );
      }}
    />
  );
}
