import { t } from "@bugaboo/messages";
import { Button, SelectField, TextField, partOf, part } from "@bugaboo/ui";
import type { Schema } from "@bugaboo/api-client";
import { useState } from "react";

import { addStatus } from "./draft-model.ts";
import { groupChoices } from "./status-groups.ts";
import type { Edit } from "./use-draft-workflow.ts";

/** The group a new status starts in: the one in between. */
const startingGroup: Schema<"StatusGroup"> = "in-progress";

/** *Add a status*: a name and a group, and the status comes at the end of the list. */
export function AddStatus({ edit }: { readonly edit: Edit }) {
  const [name, setName] = useState("");
  const [group, setGroup] = useState<Schema<"StatusGroup">>(startingGroup);
  const noName = name.trim() === "";
  return (
    <div {...partOf(part.workflowAdd)}>
      <TextField label={t("workflows.new-status")} value={name} onChange={setName} />
      <SelectField<Schema<"StatusGroup">>
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
        {t("workflows.create-status")}
      </Button>
    </div>
  );
}
