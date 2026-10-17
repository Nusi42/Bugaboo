import { statusGroupWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { IconButton, iconName, SelectField, TextField, partOf, part } from "@bugaboo/ui";

import {
  changesFrom,
  moveStatus,
  removeStatus,
  renameStatus,
  setGroup,
  type Draft,
  type DraftStatus,
  type StatusGroupWord,
} from "./draft-model.ts";
import { AddChange } from "./AddChange.tsx";
import { ChangeRow } from "./ChangeRow.tsx";
import type { Edit } from "./use-draft-workflow.ts";
import type { WorkflowEditor } from "./workflows.ts";

export interface StatusBlockProps {
  readonly editor: WorkflowEditor;
  readonly draft: Draft;
  readonly status: DraftStatus;
  readonly first: boolean;
  readonly last: boolean;
  readonly edit: Edit;
}

/** The status groups, by the words people read. */
const groupChoices = statusGroupWords.map((word) => ({
  value: word,
  label: t(`status-group.${word}`),
}));

/** The name and group of a status, and the buttons that move or remove it. */
function StatusHead({ status, first, last, draft, edit }: StatusBlockProps) {
  return (
    <div {...partOf(part.workflowStatusHead)}>
      <TextField
        label={t("workflows.status-name")}
        value={status.name}
        onChange={(name) => {
          edit((before) => renameStatus(before, status.id, name));
        }}
      />
      <SelectField<StatusGroupWord>
        label={t("workflows.status-group")}
        choices={groupChoices}
        value={status.group}
        onChange={(group) => {
          edit((before) => setGroup(before, status.id, group));
        }}
      />
      <IconButton
        icon={iconName.up}
        label={t("workflows.status-up", { name: status.name })}
        isDisabled={first}
        onPress={() => {
          edit((before) => moveStatus(before, status.id, -1));
        }}
      />
      <IconButton
        icon={iconName.down}
        label={t("workflows.status-down", { name: status.name })}
        isDisabled={last}
        onPress={() => {
          edit((before) => moveStatus(before, status.id, 1));
        }}
      />
      <IconButton
        icon={iconName.close}
        label={t("workflows.status-remove", { name: status.name })}
        isDisabled={draft.statuses.length < 2}
        onPress={() => {
          edit((before) => removeStatus(before, status.id));
        }}
      />
    </div>
  );
}

/**
 * One status of the plain list: its name, its group, whether new issues start here, and the status
 * changes that start in it, each with the words of its button and its rules.
 */
export function StatusBlock(props: StatusBlockProps) {
  const { editor, draft, status, first, edit } = props;
  const changes = changesFrom(draft, status.id);
  return (
    <li {...partOf(part.workflowStatus)}>
      <h3 {...partOf(part.workflowStatusName)}>{status.name}</h3>
      {first ? <p {...partOf(part.muted)}>{t("workflows.starts-here")}</p> : null}
      <StatusHead {...props} />
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
