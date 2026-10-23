import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, SelectField, StatusIcon, TextField, partOf, part } from "@bugaboo/ui";

import {
  changesFrom,
  moveStatus,
  removeStatus,
  renameStatus,
  setGroup,
  statusNameTaken,
  type Draft,
  type DraftStatus,
} from "./draft-model.ts";
import { groupChoices } from "./status-groups.ts";
import { AddChange } from "./AddChange.tsx";
import { ChangeRow } from "./ChangeRow.tsx";
import type { Edit } from "./use-draft-workflow.ts";
import { useNameKey } from "../../shared/use-name-key.ts";
import type { WorkflowEditor } from "./workflows.ts";

export interface StatusBlockProps {
  readonly editor: WorkflowEditor;
  readonly draft: Draft;
  readonly status: DraftStatus;
  readonly first: boolean;
  readonly last: boolean;
  readonly edit: Edit;
}

/** The state word of the block of a status: its group, which gives its color. */
const stateOfGroup = {
  "not-started": "group-not-started",
  "in-progress": "group-in-progress",
  done: "group-done",
} as const satisfies Readonly<Record<Schema<"StatusGroup">, string>>;

/** The name and the group of a status. */
function StatusFields({
  draft,
  status,
  edit,
}: Pick<StatusBlockProps, "draft" | "status" | "edit">) {
  const keyOf = useNameKey();
  const taken = keyOf !== undefined && statusNameTaken(draft, status.id, status.name, keyOf);
  return (
    <div {...partOf(part.workflowStatusHead, { variants: taken ? ["error"] : [] })}>
      <TextField
        label={t("workflows.status-name")}
        value={status.name}
        errorMessage={taken ? t("workflows.status-name-taken") : undefined}
        onChange={(name) => {
          edit((before) => renameStatus(before, status.id, name));
        }}
      />
      <SelectField<Schema<"StatusGroup">>
        label={t("workflows.status-group")}
        choices={groupChoices}
        value={status.group}
        onChange={(group) => {
          edit((before) => setGroup(before, status.id, group));
        }}
      />
    </div>
  );
}

/** The buttons that move a status up or down, or take it away. */
function StatusTools({ status, first, last, draft, edit }: StatusBlockProps) {
  return (
    <div {...partOf(part.workflowStatusTools)}>
      <Button
        look={buttonLook.quiet}
        aria-label={t("workflows.status-up", { name: status.name })}
        isDisabled={first}
        onPress={() => {
          edit((before) => moveStatus(before, status.id, -1));
        }}
      >
        {t("workflows.up")}
      </Button>
      <Button
        look={buttonLook.quiet}
        aria-label={t("workflows.status-down", { name: status.name })}
        isDisabled={last}
        onPress={() => {
          edit((before) => moveStatus(before, status.id, 1));
        }}
      >
        {t("workflows.down")}
      </Button>
      <Button
        look={buttonLook.quiet}
        aria-label={t("workflows.status-remove", { name: status.name })}
        isDisabled={draft.statuses.length < 2}
        onPress={() => {
          edit((before) => removeStatus(before, status.id));
        }}
      >
        {t("workflows.remove")}
      </Button>
    </div>
  );
}

/**
 * One status of the plain list: its name with the icon of its group, its group, whether new issues
 * start here, and the status changes that start in it, each with the words of its button and its
 * rules.
 */
export function StatusBlock(props: StatusBlockProps) {
  const { editor, draft, status, first, edit } = props;
  const changes = changesFrom(draft, status.id);
  return (
    <li {...partOf(part.workflowStatus, { states: [stateOfGroup[status.group]] })}>
      <div {...partOf(part.workflowStatusTitle)}>
        <h3 {...partOf(part.workflowStatusName)}>
          <StatusIcon group={status.group} />
          {status.name}
        </h3>
        {first ? <span {...partOf(part.muted)}>{t("workflows.starts-here")}</span> : null}
      </div>
      <StatusFields draft={draft} status={status} edit={edit} />
      <StatusTools {...props} />
      <p {...partOf(part.text)}>{t("workflows.from", { name: status.name })}</p>
      {changes.length === 0 ? (
        <p {...partOf(part.muted)}>{t("workflows.no-changes")}</p>
      ) : (
        <ul {...partOf(part.workflowChanges)}>
          {changes.map((change) => (
            <ChangeRow key={change.id} editor={editor} draft={draft} change={change} edit={edit} />
          ))}
        </ul>
      )}
      <AddChange draft={draft} from={status} edit={edit} />
    </li>
  );
}
