import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { AddLabel } from "./AddLabel.tsx";
import { AddTag } from "./AddTag.tsx";
import { Chips } from "./Chips.tsx";
import { afterSorting, type Counted } from "./labels.ts";

/** What the two parts get: the answer for the issue, and the issue it is about. */
export interface SortingPartProps {
  readonly body: Readonly<Schema<"IssueSortingBody">>;
  readonly issueKey: string;
}

/** The tags of the issue, with the ways to take one off and to add one. */
export function TagsPart({
  body,
  issueKey,
  project,
}: SortingPartProps & { readonly project: string | undefined }) {
  const takeOff = api.query.useMutation("delete", "/api/v1/issues/{issue}/tags/{tag}", {
    onSuccess: (sorting: Counted) => {
      afterSorting(issueKey, sorting);
    },
  });
  return (
    <>
      <h4>{t("labels.tags-heading")}</h4>
      <Chips
        label={t("labels.tags-heading")}
        chips={body.tags ?? []}
        none={t("labels.none-tags")}
        onTakeOff={
          body.may_change_tags
            ? (tag) => {
                takeOff.mutate({ params: { path: { issue: issueKey, tag: tag.id } } });
              }
            : undefined
        }
      />
      {body.may_change_tags && project !== undefined ? (
        <AddTag issue={issueKey} project={project} />
      ) : null}
      {takeOff.isError ? <Alert tone={alertTone.danger}>{messageOf(takeOff.error)}</Alert> : null}
    </>
  );
}

/** The labels of the issue, with the ways to take one off and to add one. */
export function LabelsPart({ body, issueKey }: SortingPartProps) {
  const takeOff = api.query.useMutation("delete", "/api/v1/issues/{issue}/labels/{label}", {
    onSuccess: (sorting: Counted) => {
      afterSorting(issueKey, sorting);
    },
  });
  return (
    <>
      <h4>{t("labels.labels-heading")}</h4>
      <Chips
        label={t("labels.labels-heading")}
        chips={body.labels ?? []}
        none={t("labels.none-labels")}
        onTakeOff={
          body.may_change_labels
            ? (label) => {
                takeOff.mutate({ params: { path: { issue: issueKey, label: label.id } } });
              }
            : undefined
        }
      />
      {body.may_change_labels ? <AddLabel issue={issueKey} has={body.labels ?? []} /> : null}
      {takeOff.isError ? <Alert tone={alertTone.danger}>{messageOf(takeOff.error)}</Alert> : null}
    </>
  );
}
