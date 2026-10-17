import { t } from "@bugaboo/messages";
import { SelectField, TextField, partOf, part } from "@bugaboo/ui";

import { ProjectsToUse } from "../fields/index.ts";
import { choiceIdOf, choiceLabelOf, priorityChoices, type ProjectChange } from "../issues/index.ts";
import { useLabels } from "../labels/index.ts";
import type { RelationChoice } from "../relations/index.ts";
import { manyChangeKind, type ManyChangeKind } from "./many-changes.ts";
import { NONE, type Draft } from "./draft.ts";

export interface ActionFieldsProps {
  readonly kind: ManyChangeKind;
  readonly draft: Draft;
  readonly onDraft: (draft: Draft) => void;
  /** The words a relation can be made with. */
  readonly relations: readonly RelationChoice[];
  /** The status changes the issues of the project can have, by their words: the server lists them. */
  readonly statusChanges: readonly ProjectChange[];
}

/** What a pick of status or priority starts with: leave the field as it is. */
const keep = { value: NONE, label: t("many-changes.keep") };
const priorityOrKeep = [keep, ...priorityChoices];

/** *Change status or priority*: what to set; a field left as it is stays as it is. */
function ChangeFields({ draft, onDraft, statusChanges }: ActionFieldsProps) {
  const statusOrKeep = [
    keep,
    ...statusChanges.map((change) => ({
      value: choiceIdOf(change),
      label: choiceLabelOf(change, statusChanges),
    })),
  ];
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
        <p {...partOf(part.text)}>
          {t("many-changes.project-picked", { name: draft.project.name })}
        </p>
      )}
      <ProjectsToUse
        picked={draft.project === null ? [] : [draft.project.id]}
        words={{
          legend: t("many-changes.project-legend"),
          search: t("many-changes.project-search"),
          list: t("many-changes.project-list"),
          nothing: t("many-changes.project-nothing"),
          more: t("many-changes.project-more"),
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
  const offered = labels.labels.filter((label) => !label.retired);
  return (
    <SelectField
      label={t("many-changes.field-label")}
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
  if (props.kind === manyChangeKind.change) {
    return <ChangeFields {...props} />;
  }
  if (props.kind === manyChangeKind.move) {
    return <MoveFields {...props} />;
  }
  if (props.kind === manyChangeKind.label) {
    return <LabelFields {...props} />;
  }
  if (props.kind === manyChangeKind.relation) {
    return <RelationFields {...props} />;
  }
  return <p {...partOf(part.text)}>{t("many-changes.delete-text")}</p>;
}
