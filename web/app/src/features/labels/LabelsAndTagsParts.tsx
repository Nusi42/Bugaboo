import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, NameValueRow } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { AddLabel } from "./AddLabel.tsx";
import { AddTag } from "./AddTag.tsx";
import { Chips } from "./Chips.tsx";
import type { LabelChanges, LabelRow } from "./use-label-changes.ts";

/** What the two rows get: the answer for the issue, and how to change it. */
export interface LabelsAndTagsRowProps {
  readonly body: Readonly<Schema<"IssueLabelsAndTagsBody">>;
  readonly changes: LabelChanges;
  /** What went wrong with the last change; `null` when nothing did. */
  readonly problem: unknown;
}

/** What went wrong with the last change, in the row where it happened. */
function Problem({ problem }: { readonly problem: unknown }) {
  return problem === null ? null : <Alert tone={alertTone.danger}>{messageOf(problem)}</Alert>;
}

/** The `status` property of a row, when the row has a last change to tell about. */
function statusOf(changes: LabelChanges, row: LabelRow) {
  const status = changes.status(row);
  return status === undefined ? {} : { status };
}

/**
 * The row *Tags* of the Details: the tags of the issue, each with the button that takes it off,
 * and the way to add one. A change is made in place and the row says *Saved* with *Undo*.
 */
export function TagsRow({
  body,
  changes,
  problem,
  project,
}: LabelsAndTagsRowProps & { readonly project: string | undefined }) {
  const tags = body.tags ?? [];
  return (
    <NameValueRow name={t("labels.tags-heading")} {...statusOf(changes, "tags")}>
      <Chips
        label={t("labels.tags-heading")}
        chips={tags}
        none={t("labels.none-tags")}
        onTakeOff={body.may_change_tags ? changes.takeTagOff : undefined}
      />
      {body.may_change_tags && project !== undefined ? (
        <AddTag
          project={project}
          issueType={body.issue_type.id}
          onAdd={(name) => changes.addTag(name, tags)}
          pending={changes.pending}
        />
      ) : null}
      <Problem problem={problem} />
    </NameValueRow>
  );
}

/** The row *Labels* of the Details: the labels of the issue, and the way to add one. */
export function LabelsRow({ body, changes, problem }: LabelsAndTagsRowProps) {
  const labels = body.labels ?? [];
  return (
    <NameValueRow name={t("labels.labels-heading")} {...statusOf(changes, "labels")}>
      <Chips
        label={t("labels.labels-heading")}
        chips={labels}
        none={t("labels.none-labels")}
        onTakeOff={body.may_change_labels ? changes.takeLabelOff : undefined}
      />
      {body.may_change_labels ? (
        <AddLabel has={labels} onAdd={changes.addLabel} pending={changes.pending} />
      ) : null}
      <Problem problem={problem} />
    </NameValueRow>
  );
}
