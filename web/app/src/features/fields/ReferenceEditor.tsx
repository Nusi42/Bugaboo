import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { usePermissions } from "../../shared/use-me.ts";
import { IssueChoice } from "./IssueChoice.tsx";
import { GroupChoice } from "./OtherChoices.tsx";
import { PeopleChoice } from "./PeopleChoice.tsx";
import { type Picked, forApi, picksOf } from "../../shared/reference.ts";
import { ValueView } from "./ValueView.tsx";
import type { ValueEditorProps } from "./ValueEditor.tsx";

/**
 * In the form of a new issue the things picked are shown by the names they were picked with. A
 * value without names (a draft kept before the form held them) says how many there are.
 */
function Chosen({ field, value }: Pick<ValueEditorProps, "field" | "value">) {
  const picks = picksOf(value);
  if (picks.length > 0 && picks.every((pick) => pick.name !== "")) {
    return <ValueView field={field} value={value} />;
  }
  const count = Array.isArray(value) ? value.length : Number(value !== undefined && value !== null);
  return count === 0 ? null : <p {...partOf(part.muted)}>{t("fields.chosen-count", { count })}</p>;
}

/** People, groups and issues: the value as shown, *Choose* to pick another, and *Take away*. */
export function ReferenceEditor({
  field,
  value,
  project,
  issue,
  issueType,
  onCommit,
  isPending = false,
  live = false,
}: ValueEditorProps) {
  const hasValue = value !== undefined && value !== null && field.rules.required !== true;
  // On the form the picks go on with their names; on an issue the server gets their IDs.
  const commit = (picks: readonly Picked[] | Picked | null): void => {
    onCommit(live ? picks : forApi(picks));
  };
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.fieldLabel)}>{field.name}</p>
      {live ? <Chosen field={field} value={value} /> : <ValueView field={field} value={value} />}
      <div {...partOf(part.row)}>
        {project === undefined ? null : (
          <Chooser
            field={field}
            value={value}
            project={project}
            issue={issue}
            issueType={issueType}
            onCommit={commit}
          />
        )}
        {hasValue ? (
          <Button
            look={buttonLook.quiet}
            isPending={isPending}
            onPress={() => {
              onCommit(null);
            }}
          >
            {t("fields.take-away")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}

/** The dialog that picks what a field of people, a group or an issue points to. */
function Chooser({
  field,
  value,
  project,
  issue,
  issueType,
  onCommit,
}: Pick<ValueEditorProps, "field" | "value" | "issue" | "issueType"> & {
  readonly project: string;
  readonly onCommit: (picks: readonly Picked[] | Picked | null) => void;
}) {
  const permissions = usePermissions();
  if (field.kind === "group") {
    // The list of groups is read with *Manage groups* (F62): without it a group can be seen and
    // taken away, not chosen.
    return permissions.includes("manage-groups") ? (
      <GroupChoice field={field} onCommit={onCommit} />
    ) : null;
  }
  if (field.kind === "issue") {
    return <IssueChoice field={field} onCommit={onCommit} />;
  }
  return (
    <PeopleChoice
      field={field}
      value={value}
      project={project}
      issue={issue}
      issueType={issueType}
      onCommit={onCommit}
    />
  );
}
