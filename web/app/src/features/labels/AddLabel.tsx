import { t } from "@bugaboo/messages";
import { Button, buttonType, Form, SelectField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { useLabels, type Chip } from "./labels.ts";
import type { Named } from "./use-label-changes.ts";

/** The choice that stands for no label picked yet. */
const NONE = "";

export interface AddLabelProps {
  /** The labels the issue has. */
  readonly has: readonly Chip[];
  /** Puts the label on the issue; says whether it was put there. */
  readonly onAdd: (label: Named) => Promise<boolean>;
  /** A change is on its way. */
  readonly pending: boolean;
}

/** *Add a label*: one of the labels of the organization that the issue does not have yet. */
export function AddLabel({ has, onAdd, pending }: AddLabelProps) {
  const [picked, setPicked] = useState(NONE);
  const labels = useLabels();
  const offered = (labels.data?.labels ?? []).filter(
    (label) => !label.retired && !has.some((each) => each.id === label.id),
  );
  if (offered.length === 0) {
    return <p {...partOf(part.muted)}>{t("labels.nothing-to-add")}</p>;
  }
  return (
    <Form
      onSubmit={() => {
        const label = offered.find((each) => each.id === picked);
        if (label !== undefined) {
          void onAdd(label).then((added) => {
            if (added) {
              setPicked(NONE);
            }
          });
        }
      }}
    >
      <SelectField
        hideLabel
        label={t("labels.add-label")}
        choices={[
          { value: NONE, label: t("labels.pick-label") },
          ...offered.map((label) => ({ value: label.id, label: label.name })),
        ]}
        value={picked}
        onChange={setPicked}
      />
      <div {...partOf(part.row)}>
        <Button type={buttonType.submit} isPending={pending} isDisabled={picked === NONE}>
          {t("labels.add-label-button")}
        </Button>
      </div>
    </Form>
  );
}
