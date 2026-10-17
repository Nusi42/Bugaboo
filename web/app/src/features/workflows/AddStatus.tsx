import { statusGroupWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, SelectField, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { addStatus, type StatusGroupWord } from "./draft-model.ts";
import type { Edit } from "./use-draft-workflow.ts";

/** The status groups, by the words people read. */
const groupChoices = statusGroupWords.map((word) => ({
  value: word,
  label: t(`status-group.${word}`),
}));

/** The group a new status starts in: the one in between. */
const startingGroup: StatusGroupWord = "in-progress";

/** *Add a status*: a name and a group, and the status comes at the end of the list. */
export function AddStatus({ edit }: { readonly edit: Edit }) {
  const [name, setName] = useState("");
  const [group, setGroup] = useState<StatusGroupWord>(startingGroup);
  const noName = name.trim() === "";
  return (
    <div {...partOf(part.workflowAdd)}>
      <TextField label={t("workflows.new-status")} value={name} onChange={setName} />
      <SelectField<StatusGroupWord>
        label={t("workflows.new-status-group")}
        choices={groupChoices}
        value={group}
        onChange={setGroup}
      />
      <Button
        isDisabled={noName}
        onPress={() => {
          edit((before) => addStatus(before, name, group));
          setName("");
        }}
      >
        {t("workflows.add-status")}
      </Button>
    </div>
  );
}
