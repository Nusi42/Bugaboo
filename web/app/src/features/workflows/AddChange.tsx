import { t } from "@bugaboo/messages";
import { Button, SelectField, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { addChange, changesFrom, type Draft, type DraftStatus } from "./draft-model.ts";
import type { Edit } from "./use-draft-workflow.ts";

export interface AddChangeProps {
  readonly draft: Draft;
  /** The status the new status change starts in. */
  readonly from: DraftStatus;
  readonly edit: Edit;
}

/** *Add a status change*: where it goes and the words of its button. */
export function AddChange({ draft, from, edit }: AddChangeProps) {
  const free = draft.statuses.filter(
    (status) =>
      status.id !== from.id && !changesFrom(draft, from.id).some((each) => each.to === status.id),
  );
  const [to, setTo] = useState("");
  const [button, setButton] = useState("");
  const target = free.some((status) => status.id === to) ? to : (free[0]?.id ?? "");
  const noWords = button.trim() === "";
  if (free.length === 0) {
    return null;
  }
  return (
    <fieldset
      aria-label={t("workflows.add-change", { from: from.name })}
      {...partOf(part.workflowAdd)}
    >
      <SelectField
        label={t("workflows.add-change-to")}
        choices={free.map((status) => ({ value: status.id, label: status.name }))}
        value={target}
        onChange={setTo}
      />
      <TextField label={t("workflows.add-change-button")} value={button} onChange={setButton} />
      <Button
        isDisabled={noWords}
        onPress={() => {
          edit((before) => addChange(before, from.id, target, button));
          setButton("");
        }}
      >
        {t("workflows.add-change-do")}
      </Button>
    </fieldset>
  );
}
