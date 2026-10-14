import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { usePermissions } from "../../shared/use-me.ts";
import { GroupChoice, IssueChoice } from "./OtherChoices.tsx";
import { PeopleChoice } from "./PeopleChoice.tsx";
import { ValueView } from "./ValueView.tsx";
import type { ValueEditorProps } from "./ValueEditor.tsx";

/** In the form of a new issue the things picked have no names yet, only IDs: it says how many. */
function Chosen({ value }: { readonly value: unknown }) {
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
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.fieldLabel)}>{field.name}</p>
      {live ? <Chosen value={value} /> : <ValueView field={field} value={value} />}
      <div {...partOf(part.row)}>
        {project === undefined ? null : (
          <Chooser
            field={field}
            value={value}
            project={project}
            issue={issue}
            issueType={issueType}
            onCommit={onCommit}
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
}: Pick<ValueEditorProps, "field" | "value" | "issue" | "issueType" | "onCommit"> & {
  readonly project: string;
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
