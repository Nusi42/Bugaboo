import { t } from "@bugaboo/messages";
import { SelectField, TextField, partOf, part } from "@bugaboo/ui";

import { ProjectsToUse } from "../fields/index.ts";
import { priorityChoices, statusChoices } from "../issues/index.ts";
import { useLabels } from "../labels/index.ts";
import type { RelationChoice } from "../relations/index.ts";
import { bulkKind, type BulkKind } from "./bulk.ts";
import { NONE, type Draft } from "./draft.ts";

export interface ActionFieldsProps {
  readonly kind: BulkKind;
  readonly draft: Draft;
  readonly onDraft: (draft: Draft) => void;
  /** The words a relation can be made with. */
  readonly relations: readonly RelationChoice[];
}

/** What a pick of status or priority starts with: leave the field as it is. */
const keep = { value: NONE, label: t("bulk.keep") };
const statusOrKeep = [keep, ...statusChoices];
const priorityOrKeep = [keep, ...priorityChoices];

/** *Change status or priority*: what to set; a field left as it is stays as it is. */
function ChangeFields({ draft, onDraft }: ActionFieldsProps) {
  return (
    <>
      <SelectField
        label={t("issue-field.status")}
        choices={statusOrKeep}
        value={draft.status}
        onChange={(status) => {
          onDraft({ ...draft, status });
        }}
      />
      <SelectField
        label={t("issue-field.priority")}
        choices={priorityOrKeep}
        value={draft.priority}
        onChange={(priority) => {
          onDraft({ ...draft, priority });
        }}
      />
    </>
  );
}

/** *Move to a project*: the project is found by words and picked. */
function MoveFields({ draft, onDraft }: ActionFieldsProps) {
  return (
    <>
      {draft.project === null ? null : (
        <p {...partOf(part.text)}>{t("bulk.project-picked", { name: draft.project.name })}</p>
      )}
      <ProjectsToUse
        picked={draft.project === null ? [] : [draft.project.id]}
        words={{
          legend: t("bulk.project-legend"),
          search: t("bulk.project-search"),
          list: t("bulk.project-list"),
          nothing: t("bulk.project-nothing"),
          more: t("bulk.project-more"),
        }}
        onPick={(project) => {
          onDraft({ ...draft, project });
        }}
      />
    </>
  );
}

/** *Add a label*: one of the labels that are not retired. */
function LabelFields({ draft, onDraft }: ActionFieldsProps) {
  const labels = useLabels();
  const offered = (labels.data?.labels ?? []).filter((label) => !label.retired);
  return (
    <SelectField
      label={t("bulk.field-label")}
      choices={[
        { value: NONE, label: t("labels.pick-label") },
        ...offered.map((label) => ({ value: label.id, label: label.name })),
      ]}
      value={draft.label}
      onChange={(label) => {
        onDraft({ ...draft, label });
      }}
    />
  );
}

/** *Add a relation*: the words of the relation, and the other issue by its readable ID. */
function RelationFields({ draft, onDraft, relations }: ActionFieldsProps) {
  return (
    <>
      <SelectField
        label={t("related-issues.kind-field")}
        choices={[
          { value: NONE, label: t("labels.pick-label") },
          ...relations.map((each) => ({ value: each.key, label: each.name })),
        ]}
        value={draft.relation}
        onChange={(relation) => {
          onDraft({ ...draft, relation });
        }}
      />
      <TextField
        label={t("common.issue-id")}
        value={draft.other}
        onChange={(other) => {
          onDraft({ ...draft, other });
        }}
      />
    </>
  );
}

/** What the person fills in for the thing they chose; deleting asks for nothing. */
export function ActionFields(props: ActionFieldsProps) {
  if (props.kind === bulkKind.change) {
    return <ChangeFields {...props} />;
  }
  if (props.kind === bulkKind.move) {
    return <MoveFields {...props} />;
  }
  if (props.kind === bulkKind.label) {
    return <LabelFields {...props} />;
  }
  if (props.kind === bulkKind.relation) {
    return <RelationFields {...props} />;
  }
  return <p {...partOf(part.text)}>{t("bulk.delete-text")}</p>;
}
